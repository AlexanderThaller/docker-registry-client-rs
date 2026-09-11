use reqwest::{
    Client as HTTPClient,
    RequestBuilder,
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

/// Blob content varies a lot more than manifests (JSON, tar, arbitrary
/// binary data), so we accept anything instead of a curated list of media
/// types.
const BLOB_ACCEPT: &str = "application/octet-stream, */*";

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

    /// Returns the raw content of the blob with the given digest, e.g. an
    /// image layer or a config blob.
    ///
    /// Follows the same authentication flow as [`Client::get_manifest_url`]:
    /// if the registry answers with `401 Unauthorized` and a
    /// `WWW-Authenticate` challenge, a token is fetched and the request is
    /// retried once.
    ///
    /// # Errors
    /// Returns an error if the request fails.
    /// Returns an error if the response status is not successful.
    #[tracing::instrument(skip_all)]
    pub async fn get_blob(&self, image: &Image, digest: &str) -> Result<Vec<u8>, Error> {
        let url = Url::parse(&format!(
            "https://{domain}/v2/{path}/blobs/{digest}",
            domain = image.registry.registry_domain(),
            path = image.path(),
        ))
        .map_err(Error::InvalidBlobUrl)?;

        let headers = self.get_headers(image).await?;
        let response = self.request_blob(&url, headers).await?;

        let response = match Self::challenge(&response) {
            Some(challenge) => {
                debug!("registry asked us to authenticate: {challenge:?}");

                let headers = self.authenticate(image, &challenge).await?;

                self.request_blob(&url, headers).await?
            }

            None => response,
        };

        Self::into_blob_response(&url, response).await
    }

    /// Builds the `GET` request for the given URL and headers, inserting the
    /// given `Accept` header.
    fn build_request(
        &self,
        url: &Url,
        mut headers: HeaderMap,
        accept: reqwest::header::HeaderValue,
    ) -> RequestBuilder {
        headers.insert("Accept", accept);

        self.client.get(url.as_str()).headers(headers)
    }

    #[tracing::instrument(skip_all)]
    async fn request_manifest(
        &self,
        url: &Url,
        headers: HeaderMap,
    ) -> Result<reqwest::Response, Error> {
        let accept = ACCEPT
            .join(", ")
            .parse()
            .map_err(Error::ParseManifestAcceptHeader)?;

        self.build_request(url, headers, accept)
            .send()
            .instrument(info_span!("get manifest request"))
            .await
            .map_err(Error::GetManifest)
    }

    #[tracing::instrument(skip_all)]
    async fn request_blob(
        &self,
        url: &Url,
        headers: HeaderMap,
    ) -> Result<reqwest::Response, Error> {
        let accept = BLOB_ACCEPT.parse().map_err(Error::ParseBlobAcceptHeader)?;

        self.build_request(url, headers, accept)
            .send()
            .instrument(info_span!("get blob request"))
            .await
            .map_err(Error::GetBlob)
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

    #[tracing::instrument(skip_all)]
    async fn into_blob_response(url: &Url, response: reqwest::Response) -> Result<Vec<u8>, Error> {
        let status = response.status();

        let body = response
            .bytes()
            .instrument(info_span!("extract blob request body"))
            .await
            .map_err(Error::ExtractBlobBody)?;

        if !status.is_success() {
            if status == StatusCode::NOT_FOUND {
                return Err(Error::BlobNotFound(url.clone()));
            }

            return Err(Error::FailedBlobRequest(
                status,
                String::from_utf8_lossy(&body).into_owned(),
            ));
        }

        Ok(body.to_vec())
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
            Manifest,
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

        /// Fetches the cosign-attached SBOM manifest for an image (following
        /// the `<digest-with-dashes>.sbom` tag convention cosign uses) and
        /// downloads the raw content of its first layer as a blob, asserting
        /// it is non-empty, valid JSON.
        #[tokio::test]
        async fn cosign_sbom_blob() {
            const INPUT: &str = "ghcr.io/sigstore/cosign/cosign:v2.4.1";

            let client = Client::new();
            let image: Image = INPUT.parse().unwrap();

            let manifest_response = client.get_manifest(&image).await.unwrap();
            let digest = manifest_response
                .digest
                .expect("registry did not return a Docker-Content-Digest header");

            let sbom_tag = format!("{}.sbom", digest.replace(':', "-"));

            let sbom_image = Image {
                image_name: ImageName {
                    identifier: Either::Left(Tag::Specific(sbom_tag)),
                    ..image.image_name.clone()
                },
                ..image.clone()
            };

            let sbom_response = client.get_manifest(&sbom_image).await.unwrap();

            let layer_digest = match sbom_response.manifest {
                Manifest::Image(image_manifest) => image_manifest
                    .layers
                    .first()
                    .expect("sbom manifest has no layers")
                    .digest
                    .clone(),
                other => panic!("expected an image manifest, got: {other:?}"),
            };

            let blob = client.get_blob(&sbom_image, &layer_digest).await.unwrap();

            assert_ne!(blob, Vec::<u8>::new());

            let _: serde_json::Value =
                serde_json::from_slice(&blob).expect("sbom blob is not valid JSON");
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
