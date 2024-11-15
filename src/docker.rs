use reqwest::{
    header::HeaderMap,
    Client as HTTPClient,
};
use serde::{
    Deserialize,
    Serialize,
};
use tracing::{
    info_span,
    Instrument,
};
use url::Url;

use crate::{
    Image,
    Manifest,
    Registry,
};

mod error;
pub mod token;
pub mod token_cache;

pub use error::Error;
use token::Token;
use token_cache::Cache as TokenCache;

#[derive(Debug, Clone)]
pub struct Client {
    client: HTTPClient,
    token_cache: Box<dyn TokenCache + Send>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub digest: Option<String>,
    pub manifest: Manifest,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            client: HTTPClient::new(),
            token_cache: Box::new(token_cache::MemoryTokenCache::default()),
        }
    }
}

impl Client {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_cache_memory(&mut self) {
        self.token_cache = Box::new(token_cache::MemoryTokenCache::default());
    }

    pub fn disable_caching(&mut self) {
        self.token_cache = Box::new(token_cache::NoCache);
    }

    #[cfg(feature = "redis_cache")]
    pub fn set_cache_redis(&mut self, redis_client: redis::Client) {
        self.token_cache = Box::new(token_cache::RedisCache::new(redis_client));
    }

    #[tracing::instrument]
    pub async fn get_manifest_url(&self, url: &Url, image: &Image) -> Result<Response, Error> {
        let mut headers = self.get_headers(image).await?;

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
                return Err(Error::ManifestNotFound(url.clone()));
            }

            return Err(Error::FailedManifestRequest(status, body));
        }

        let manifest =
            serde_json::from_str(&body).map_err(|e| Error::DeserializeManifestBody(e, body))?;

        Ok(Response { digest, manifest })
    }

    /// # Errors
    /// Returns an error if the request fails.
    /// Returns an error if the response body is not valid JSON.
    /// Returns an error if the response body is not a valid manifest.
    /// Returns an error if the response status is not successful.
    #[tracing::instrument(skip_all)]
    pub async fn get_manifest(&self, image: &Image) -> Result<Response, Error> {
        let registry_domain = image.registry.registry_domain();

        let url = Url::parse(&format!(
            "https://{domain}/v2/{namespace}{repository}{image_name}/manifests/{identifier}",
            domain = registry_domain,
            namespace = match image.namespace {
                Some(ref namespace) => format!("{namespace}/"),
                None => String::new(),
            },
            repository = match image.repository {
                Some(ref repository) => format!("{repository}/"),
                None => String::new(),
            },
            image_name = image.image_name.name,
            identifier = image.image_name.identifier
        ))
        .map_err(Error::InvalidManifestUrl)?;

        self.get_manifest_url(&url, image).await
    }

    #[tracing::instrument(skip_all)]
    async fn get_headers(&self, image: &Image) -> Result<HeaderMap, Error> {
        if !image.registry.needs_authentication() {
            return Ok(HeaderMap::new());
        }

        let cache_key = image.into();

        let token = self
            .token_cache
            .fetch(&cache_key)
            .await
            .map_err(Error::FetchToken)?;

        let token = if let Some(token) = token {
            token
        } else {
            let namespace = match &image.namespace {
                Some(namespace) => format!("{namespace}/"),
                None => String::new(),
            };

            let repository = match &image.repository {
                Some(repository) => format!("{repository}/"),
                None => String::new(),
            };

            let token_url = match image.registry {
                Registry::Github => format!(
                    "https://ghcr.io/token?scope=repository:{namespace}{repository}{image_name}:pull&service=ghcr.io",
                    image_name = image.image_name.name
                ),

                Registry::DockerHub => format!("https://auth.docker.io/token?service=registry.docker.io&scope=repository:{namespace}{repository}{image_name}:pull&service=registry.docker.io", image_name = image.image_name.name),

                Registry::Quay => format!("https://quay.io/v2/auth?scope=repository:{namespace}{repository}{image_name}:pull&service=quay.io", image_name = image.image_name.name),

                Registry::RedHat | Registry::K8s | Registry::Google | Registry::Microsoft => return Ok(HeaderMap::new()),
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

            self.token_cache
                .store(cache_key, token.clone())
                .await
                .map_err(Error::StoreToken)?;

            token
        };

        let headers = token.try_into().map_err(Error::ParseAuthorizationHeader)?;

        Ok(headers)
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "using unwrap in tests is fine")]
mod tests {
    mod dockerhub {
        use crate::{
            Client,
            Image,
            ImageName,
            Registry,
            Tag,
        };
        use either::Either;

        #[tokio::test]
        async fn alpine() {
            let client = Client::new();

            let image_name = Image {
                registry: Registry::DockerHub,
                namespace: None,
                repository: Some("library".to_string()),
                image_name: ImageName {
                    name: "alpine".to_string(),
                    identifier: Either::Left(Tag::Specific("3.20".to_string())),
                },
            };

            let response = client.get_manifest(&image_name).await.unwrap();

            insta::assert_json_snapshot!(response);
        }
    }

    mod redhat {
        use crate::{
            Client,
            Image,
            ImageName,
            Registry,
            Tag,
        };
        use either::Either;

        #[tokio::test]
        async fn ubi8() {
            let client = Client::new();

            let image = Image {
                registry: Registry::RedHat,
                namespace: None,
                repository: None,
                image_name: ImageName {
                    name: "ubi8".to_string(),
                    identifier: Either::Left(Tag::Specific("8.9".to_string())),
                },
            };

            let response = client.get_manifest(&image).await.unwrap();

            insta::assert_json_snapshot!(response);
        }

        #[tokio::test]
        async fn cosign() {
            const INPUT: &str = "ghcr.io/sigstore/cosign/cosign:v2.4.0";

            let client = Client::new();
            let image = INPUT.parse().unwrap();
            let response = client.get_manifest(&image).await.unwrap();

            insta::assert_json_snapshot!(response);
        }

        #[tokio::test]
        async fn playwright() {
            const INPUT: &str = "mcr.microsoft.com/playwright:v1.48.2-noble";

            let client = Client::new();
            let image = INPUT.parse().unwrap();
            let response = client.get_manifest(&image).await.unwrap();

            insta::assert_json_snapshot!(response);
        }
    }
}
