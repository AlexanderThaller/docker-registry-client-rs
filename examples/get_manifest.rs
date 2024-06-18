use docker_registry_client::docker::Client;

#[tokio::main]
async fn main() -> Result<(), eyre::Error> {
    let client = Client::new();

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
