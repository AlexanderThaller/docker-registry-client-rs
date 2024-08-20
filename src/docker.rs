use std::{
    collections::HashMap,
    sync::Arc,
};

use reqwest::{
    header::HeaderMap,
    Client as HTTPClient,
};
use serde::{
    Deserialize,
    Serialize,
};
use tokio::sync::RwLock;
use tracing::{
    info_span,
    Instrument,
};
use url::Url;

use crate::{
    image_name::{
        ImageName,
        Registry,
    },
    manifest::Manifest,
};

mod error;

pub use error::Error;

#[derive(Debug, Default, Clone)]
pub struct Client {
    client: HTTPClient,

    token_cache: Arc<RwLock<HashMap<CacheKey, String>>>,
}

#[derive(Debug, Eq, PartialEq, Hash)]
struct CacheKey {
    registry: Registry,
    repository: Option<String>,
    image_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub digest: Option<String>,
    pub manifest: Manifest,
}

impl Client {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// # Errors
    /// Returns an error if the request fails.
    /// Returns an error if the response body is not valid JSON.
    /// Returns an error if the response body is not a valid manifest.
    /// Returns an error if the response status is not successful.
    #[tracing::instrument]
    pub async fn get_manifest(&self, image_name: &ImageName) -> Result<Response, Error> {
        let mut headers = self.get_headers(image_name).await?;

        let accept_header = [
            "application/vnd.docker.container.image.v1+json",
            "application/vnd.docker.distribution.manifest.list.v2+json",
            "application/vnd.docker.distribution.manifest.v2+json",
            "application/vnd.docker.image.rootfs.diff.tar.gzip",
            "application/vnd.docker.image.rootfs.foreign.diff.tar.gzip",
            "application/vnd.docker.plugin.v1+json",
            "application/vnd.oci.image.index.v1+json",
            "application/vnd.oci.image.manifest.v1+json",
        ]
        .join(", ");

        headers.insert(
            "Accept",
            accept_header
                .parse()
                .map_err(Error::ParseManifestAcceptHeader)?,
        );

        let registry_domain = image_name.registry.registry_domain();

        let url = Url::parse(&format!(
            "https://{domain}/v2/{repository}{image_name}/manifests/{identifier}",
            domain = registry_domain,
            repository = match image_name.repository {
                Some(ref repository) => format!("{repository}/"),
                None => String::new(),
            },
            image_name = image_name.image_name,
            identifier = image_name.identifier
        ))
        .map_err(Error::InvalidManifestUrl)?;

        let response = self
            .client
            .get(url.as_str())
            .headers(headers)
            .send()
            .instrument(info_span!("get manifest request"))
            .await
            .map_err(Error::GetManifest)?;

        let status = response.status();

        let digest = response
            .headers()
            .get("Docker-Content-Digest")
            .map(|header| {
                header
                    .to_str()
                    .map(String::from)
                    .map_err(Error::ParseDockerContentDigestHeader)
            })
            .transpose()?;

        let body = response
            .text()
            .instrument(info_span!("extract manifest request body"))
            .await
            .map_err(Error::ExtractManifestBody)?;

        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(Error::ManifestNotFound(url));
            }

            return Err(Error::FailedManifestRequest(status, body));
        }

        let manifest =
            serde_json::from_str(&body).map_err(|e| Error::DeserializeManifestBody(e, body))?;

        Ok(Response { digest, manifest })
    }

    #[tracing::instrument]
    async fn get_headers(&self, image_name: &ImageName) -> Result<HeaderMap, Error> {
        #[derive(Debug, serde::Deserialize)]
        struct Token {
            token: String,
        }

        let cache_key = CacheKey {
            registry: image_name.registry.clone(),
            repository: image_name.repository.clone(),
            image_name: image_name.image_name.clone(),
        };

        let token_cache = self
            .token_cache
            .read()
            .instrument(info_span!("get token from cache"))
            .await;

        let token = token_cache.get(&cache_key).cloned();
        drop(token_cache);

        let token = if let Some(token) = token {
            token
        } else {
            let repository = match &image_name.repository {
                Some(repository) => format!("{repository}/"),
                None => String::new(),
            };

            let token_url = match image_name.registry {
                Registry::Github => format!(
                    "https://ghcr.io/token?scope=repository:{repository}{image_name}:pull&service=ghcr.io",
                    image_name = image_name.image_name
                ),

                Registry::DockerHub => format!("https://auth.docker.io/token?service=registry.docker.io&scope=repository:{repository}{image_name}:pull&service=registry.docker.io", image_name = image_name.image_name),

                Registry::Quay => format!("https://quay.io/v2/auth?scope=repository:{repository}{image_name}:pull&service=quay.io", image_name = image_name.image_name),

                Registry::Specific(_) => return Ok(HeaderMap::new()),
            };

            let token_url = Url::parse(&token_url).map_err(Error::InvalidTokenUrl)?;

            let response = self
                .client
                .get(token_url)
                .send()
                .instrument(info_span!("get token request"))
                .await
                .map_err(Error::GetToken)?;

            let body = response
                .text()
                .instrument(info_span!("extract token request body"))
                .await
                .map_err(Error::ExtractTokenBody)?;

            let token: Token =
                serde_json::from_str(&body).map_err(|e| Error::DeserializeToken(e, body))?;

            let mut token_cache = self
                .token_cache
                .write()
                .instrument(info_span!("write token to cache"))
                .await;
            token_cache.insert(cache_key, token.token.clone());

            token.token
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {token}")
                .parse()
                .map_err(Error::ParseAuthorizationHeader)?,
        );

        Ok(headers)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Client,
        ImageName,
        Registry,
        Tag,
    };
    use either::Either;

    #[tokio::test]
    async fn get_manifest_dockerhub() {
        let client = Client::new();

        let image_name = ImageName {
            registry: Registry::DockerHub,
            repository: Some("library".to_string()),
            image_name: "alpine".to_string(),
            identifier: Either::Left(Tag::Specific("3.20".to_string())),
        };

        let response = client.get_manifest(&image_name).await.unwrap();

        insta::assert_json_snapshot!(response);
    }

    #[tokio::test]
    async fn get_manifest_redhat() {
        let client = Client::new();

        let image_name = ImageName {
            registry: Registry::DockerHub,
            repository: None,
            image_name: "ubi8".to_string(),
            identifier: Either::Left(Tag::Specific("8.9".to_string())),
        };

        let response = client.get_manifest(&image_name).await.unwrap();

        insta::assert_json_snapshot!(response);
    }
}
