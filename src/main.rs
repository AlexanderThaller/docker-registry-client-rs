use reqwest::{
    header::HeaderMap,
    Client,
};
use url::Url;

mod image_name;
mod manifest;

use image_name::{
    ImageName,
    Registry,
};
use manifest::Manifest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = DockerClient::new();

    let image_name = "ghcr.io/aquasecurity/trivy:0.52.0".parse()?;
    dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    dbg!(&manifest);

    let image_name = "archlinux:latest".parse()?;
    dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    dbg!(&manifest);

    let image_name = "quay.io/argoproj/argocd:latest".parse()?;
    dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    dbg!(&manifest);

    let image_name =
        "quay.io/openshift-community-operators/external-secrets-operator:v0.9.9".parse()?;
    dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    dbg!(&manifest);

    Ok(())
}

struct DockerClient {
    client: Client,
}

impl DockerClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

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
                let value: serde_json::Value = serde_json::from_str(&body).unwrap();
                println!("{}", serde_json::to_string_pretty(&value).unwrap());

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
