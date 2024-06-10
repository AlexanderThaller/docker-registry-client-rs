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
    pub async fn get_manifest(
        &self,
        image_name: &ImageName,
    ) -> Result<Manifest, Box<dyn std::error::Error>> {
        let mut headers = match image_name.registry {
            Registry::DockerHub => self.get_dockerio_headers(image_name).await?,
            Registry::Github => self.get_ghcr_headers(image_name).await?,
            _ => HeaderMap::new(),
        };

        headers.insert(
            "Accept",
            "application/vnd.docker.distribution.manifest.list.v2+json".parse()?,
        );

        headers.insert(
            "Accept",
            "application/vnd.oci.image.manifest.v1+json".parse()?,
        );

        headers.insert("Accept", "application/vnd.oci.image.index.v1+json".parse()?);

        let registry_domain = image_name.registry.registry_domain();
        let url = Url::parse(&format!(
            "https://{}/v2/{}/{}/manifests/{}",
            registry_domain, image_name.repository, image_name.image_name, image_name.identifier
        ))?;

        let response = self.client.get(url).headers(headers).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            println!("{body}");
            return Err("Failed to fetch manifest".into());
        }

        match serde_json::from_str(&body) {
            Err(err) => {
                let value: serde_json::Value = serde_json::from_str(&body)?;
                println!("{}", serde_json::to_string_pretty(&value)?);

                Err(err.into())
            }

            Ok(v) => Ok(v),
        }
    }

    async fn get_ghcr_headers(
        &self,
        image_name: &ImageName,
    ) -> Result<HeaderMap, Box<dyn std::error::Error>> {
        // https://gist.github.com/eggplants/a046346571de66656f4d4d34de69fdd0

        #[derive(Debug, serde::Deserialize)]
        struct Token {
            token: String,
        }

        let token_url = Url::parse(&format!(
            "https://ghcr.io/token?scope=repository:{}/{}:pull",
            image_name.repository, image_name.image_name
        ))?;

        let response = self.client.get(token_url).send().await?;
        let token = response.json::<Token>().await?;

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", token.token).parse()?);

        Ok(headers)
    }

    async fn get_dockerio_headers(
        &self,
        image_name: &ImageName,
    ) -> Result<HeaderMap, Box<dyn std::error::Error>> {
        // https://docs.docker.com/docker-hub/download-rate-limit/

        #[derive(Debug, serde::Deserialize)]
        struct Token {
            token: String,
        }

        let token_url = Url::parse(&format!(
            "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}/{}:pull",
            image_name.repository, image_name.image_name
        ))?;

        let response = self.client.get(token_url).send().await?;
        let token = response.json::<Token>().await?;

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", token.token).parse()?);

        Ok(headers)
    }
}
