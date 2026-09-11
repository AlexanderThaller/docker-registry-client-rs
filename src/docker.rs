use reqwest::{
    Client as HTTPClient,
    StatusCode,
    header::HeaderMap,
};
use serde::{
    Deserialize,
    Serialize,
};
use tracing::{
    Instrument,
    debug,
    info_span,
};
use url::Url;

use crate::{
    Image,
    Manifest,
    image::registry::Authentication,
};

mod auth;
mod error;
pub mod token;
pub mod token_cache;

use auth::Challenge;
pub use error::Error;
use token::Token;
use token_cache::Cache as TokenCache;

const ACCEPT: [&str; 8] = [
    "application/vnd.docker.container.image.v1+json",
    "application/vnd.docker.distribution.manifest.list.v2+json",
    "application/vnd.docker.distribution.manifest.v2+json",
    "application/vnd.docker.image.rootfs.diff.tar.gzip",
    "application/vnd.docker.image.rootfs.foreign.diff.tar.gzip",
    "application/vnd.docker.plugin.v1+json",
    "application/vnd.oci.image.index.v1+json",
    "application/vnd.oci.image.manifest.v1+json",
];

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

    /// Caches tokens in Redis using the given [`fred`] client.
    ///
    /// The client has to be connected (see
    /// [`fred::interfaces::ClientLike::init`]) before it is passed in.
    #[cfg(feature = "redis_cache")]
    pub fn set_cache_redis(&mut self, redis_client: fred::clients::Client) {
        self.token_cache = Box::new(token_cache::RedisCache::new(redis_client));
    }

    /// Returns the manifest for the given URL and image.
    ///
    /// If the registry answers with `401 Unauthorized` and tells us how to
    /// authenticate in its `WWW-Authenticate` header the token is fetched and
    /// the request is retried once. This is what makes registries work that
    /// are not known to this crate.
    ///
    /// # Errors
    /// Returns an error if the request fails.
    /// Returns an error if the response body is not valid JSON.
    /// Returns an error if the response body is not a valid manifest.
    /// Returns an error if the response status is not successful.
    #[tracing::instrument]
    pub async fn get_manifest_url(&self, url: &Url, image: &Image) -> Result<Response, Error> {
        let headers = self.get_headers(image).await?;
        let response = self.request_manifest(url, headers).await?;

        let response = match Self::challenge(&response) {
            Some(challenge) => {
                debug!("registry asked us to authenticate: {challenge:?}");

                let headers = self.authenticate(image, &challenge).await?;

                self.request_manifest(url, headers).await?
            }

            None => response,
        };

        Self::into_response(url, response).await
    }

    /// # Errors
    /// Returns an error if the request fails.
    /// Returns an error if the response body is not valid JSON.
    /// Returns an error if the response body is not a valid manifest.
    /// Returns an error if the response status is not successful.
    #[tracing::instrument(skip_all)]
    pub async fn get_manifest(&self, image: &Image) -> Result<Response, Error> {
        let url = Url::parse(&format!(
            "https://{domain}/v2/{path}/manifests/{identifier}",
            domain = image.registry.registry_domain(),
            path = image.path(),
            identifier = image.image_name.identifier
        ))
        .map_err(Error::InvalidManifestUrl)?;

        self.get_manifest_url(&url, image).await
    }

    #[tracing::instrument(skip_all)]
    async fn request_manifest(
        &self,
        url: &Url,
        mut headers: HeaderMap,
    ) -> Result<reqwest::Response, Error> {
        headers.insert(
            "Accept",
            ACCEPT
                .join(", ")
                .parse()
                .map_err(Error::ParseManifestAcceptHeader)?,
        );

        self.client
            .get(url.as_str())
            .headers(headers)
            .send()
            .instrument(info_span!("get manifest request"))
            .await
            .map_err(Error::GetManifest)
    }

    #[tracing::instrument(skip_all)]
    async fn into_response(url: &Url, response: reqwest::Response) -> Result<Response, Error> {
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
            if status == StatusCode::NOT_FOUND {
                return Err(Error::ManifestNotFound(url.clone()));
            }

            return Err(Error::FailedManifestRequest(status, body));
        }

        let manifest =
            serde_json::from_str(&body).map_err(|e| Error::DeserializeManifestBody(e, body))?;

        Ok(Response { digest, manifest })
    }

    /// The bearer challenge the registry answered an unauthorized request with.
    ///
    /// Returns [`None`] if the request was not rejected or if the registry does
    /// not tell us how to get a token.
    fn challenge(response: &reqwest::Response) -> Option<Challenge> {
        if response.status() != StatusCode::UNAUTHORIZED {
            return None;
        }

        let header = response
            .headers()
            .get(reqwest::header::WWW_AUTHENTICATE)?
            .to_str()
            .ok()?;

        Challenge::from_www_authenticate(header)
    }

    /// Headers for a manifest request of the given image.
    ///
    /// Registries that are known to need a token get one upfront, for every
    /// other registry we try without a token first and let
    /// [`Client::get_manifest_url`] handle the challenge we might get back.
    #[tracing::instrument(skip_all)]
    async fn get_headers(&self, image: &Image) -> Result<HeaderMap, Error> {
        let authentication = image.registry.authentication();

        if authentication == Authentication::None {
            return Ok(HeaderMap::new());
        }

        if let Some(token) = self
            .token_cache
            .fetch(&image.into())
            .await
            .map_err(Error::FetchToken)?
        {
            return token.try_into().map_err(Error::ParseAuthorizationHeader);
        }

        match Challenge::try_from(authentication) {
            Ok(challenge) => self.authenticate(image, &challenge).await,
            Err(()) => Ok(HeaderMap::new()),
        }
    }

    /// Fetches a token for the given image following the given challenge and
    /// returns the headers to authenticate with.
    #[tracing::instrument(skip_all)]
    async fn authenticate(&self, image: &Image, challenge: &Challenge) -> Result<HeaderMap, Error> {
        let scope = format!("repository:{path}:pull", path = image.path());

        let token_url = challenge
            .token_url(&scope)
            .map_err(Error::InvalidTokenUrl)?;

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
            .store(image.into(), token.clone())
            .await
            .map_err(Error::StoreToken)?;

        token.try_into().map_err(Error::ParseAuthorizationHeader)
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

    mod codeberg {
        use crate::Client;

        #[tokio::test]
        async fn forgejo() {
            const INPUT: &str = "codeberg.org/forgejo/forgejo:1.20.1-0-rootless";

            let client = Client::new();
            let image = INPUT.parse().unwrap();
            let response = client.get_manifest(&image).await.unwrap();

            insta::assert_json_snapshot!(response);
        }
    }

    /// Registries that are not known to this crate are talked to by
    /// discovering their authentication from the `WWW-Authenticate` header.
    mod unknown_registry {
        use crate::Client;

        #[tokio::test]
        async fn public_ecr() {
            const INPUT: &str = "public.ecr.aws/docker/library/alpine:3.20";

            let client = Client::new();
            let image = INPUT.parse().unwrap();
            let response = client.get_manifest(&image).await.unwrap();

            insta::assert_json_snapshot!(response);
        }
    }
}
