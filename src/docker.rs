use std::{
    collections::HashMap,
    sync::Arc,
};

use reqwest::{
    header::HeaderMap,
    Client as HTTPClient,
};
use tokio::sync::RwLock;
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
    repository: String,
    image_name: String,
}

#[derive(Debug, Clone)]
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
            "https://{}/v2/{}/{}/manifests/{}",
            registry_domain, image_name.repository, image_name.image_name, image_name.identifier
        ))
        .map_err(Error::InvalidManifestUrl)?;

        let response = self
            .client
            .get(url.as_str())
            .headers(headers)
            .send()
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

        let body = response.text().await.map_err(Error::ExtractManifestBody)?;

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

        let token_cache = self.token_cache.read().await;
        let token = token_cache.get(&cache_key).cloned();
        drop(token_cache);

        let token = if let Some(token) = token {
            token
        } else {
            let token_url = match image_name.registry {
                Registry::Github => format!(
                    "https://ghcr.io/token?scope=repository:{}/{}:pull&service=ghcr.io",
                    image_name.repository, image_name.image_name
                ),
                Registry::DockerHub => format!("https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}/{}:pull&service=registry.docker.io", image_name.repository, image_name.image_name),
                Registry::Quay => format!("https://quay.io/v2/auth?scope=repository:{}/{}:pull&service=quay.io", image_name.repository, image_name.image_name),

                Registry::Specific(_) => return Ok(HeaderMap::new()),
            };

            let token_url = Url::parse(&token_url).map_err(Error::InvalidTokenUrl)?;

            let response = self
                .client
                .get(token_url)
                .send()
                .await
                .map_err(Error::GetToken)?;

            let body = response.text().await.map_err(Error::ExtractTokenBody)?;

            let token: Token =
                serde_json::from_str(&body).map_err(|e| Error::DeserializeToken(e, body))?;

            let mut token_cache = self.token_cache.write().await;
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
