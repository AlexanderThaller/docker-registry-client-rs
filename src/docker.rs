use reqwest::{
    header::HeaderMap,
    Client as HTTPClient,
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
    pub async fn get_manifest(&self, image_name: &ImageName) -> Result<Manifest, Error> {
        let mut headers = match image_name.registry {
            Registry::DockerHub => self.get_dockerio_headers(image_name).await?,
            Registry::Github => self.get_ghcr_headers(image_name).await?,
            _ => HeaderMap::new(),
        };

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
        let body = response.text().await.map_err(Error::ExtractManifestBody)?;

        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(Error::ManifestNotFound(url));
            }

            return Err(Error::FailedManifestRequest(status, body));
        }

        serde_json::from_str(&body).map_err(|e| Error::DeserializeManifestBody(e, body))
    }

    async fn get_ghcr_headers(&self, image_name: &ImageName) -> Result<HeaderMap, Error> {
        // https://gist.github.com/eggplants/a046346571de66656f4d4d34de69fdd0

        #[derive(Debug, serde::Deserialize)]
        struct Token {
            token: String,
        }

        let token_url = Url::parse(&format!(
            "https://ghcr.io/token?scope=repository:{}/{}:pull",
            image_name.repository, image_name.image_name
        ))
        .map_err(Error::InvalidGHCRTokenUrl)?;

        let response = self
            .client
            .get(token_url)
            .send()
            .await
            .map_err(Error::GetGHCRToken)?;

        let body = response.text().await.map_err(Error::ExtractGHCRTokenBody)?;

        let token: Token =
            serde_json::from_str(&body).map_err(|e| Error::DeserializeGHCRToken(e, body))?;

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token.token)
                .parse()
                .map_err(Error::ParseGHCRAuthorizationHeader)?,
        );

        Ok(headers)
    }

    async fn get_dockerio_headers(&self, image_name: &ImageName) -> Result<HeaderMap, Error> {
        // https://docs.docker.com/docker-hub/download-rate-limit/

        #[derive(Debug, serde::Deserialize)]
        struct Token {
            token: String,
        }

        let token_url = Url::parse(&format!(
            "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}/{}:pull",
            image_name.repository, image_name.image_name
        ))
        .map_err(Error::InvalidDockerIoTokenUrl)?;

        let response = self
            .client
            .get(token_url)
            .send()
            .await
            .map_err(Error::GetDockerIoToken)?;

        let body = response
            .text()
            .await
            .map_err(Error::ExtractDockerIoTokenBody)?;

        let token: Token =
            serde_json::from_str(&body).map_err(|e| Error::DeserializeDockerIoToken(e, body))?;

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token.token)
                .parse()
                .map_err(Error::ParseDockerIoAuthorizationHeader)?,
        );

        Ok(headers)
    }
}
