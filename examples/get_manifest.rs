use docker_registry_client::docker::Client;

#[tokio::main]
async fn main() -> Result<(), eyre::Error> {
    let client = Client::new();

    let image_name = "archlinux:latest".parse()?;
    // dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    // dbg!(&manifest);

    let image_name = "index.docker.io/grafana/grafana:\
                      sha256-b018257986b51ee1179f5a416c2c90e9698941d3f3104ccecfdc250e9bf07555.sig"
        .parse()?;
    // dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    // dbg!(&manifest);

    Ok(())
}

async fn bla(client: Client) -> Result<(), eyre::Error> {
    let image_name = "ghcr.io/aquasecurity/trivy:0.52.0".parse()?;
    // dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    // dbg!(&manifest);

    let image_name = "quay.io/argoproj/argocd:latest".parse()?;
    // dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    // dbg!(&manifest);

    let image_name =
        "quay.io/openshift-community-operators/external-secrets-operator:v0.9.9".parse()?;
    // dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    // dbg!(&manifest);

    let image_name = "quay.io/jetstack/cert-manager-controller:v1.14.5".parse()?;
    // dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    // dbg!(&manifest);

    let image_name = "quay.io/argoproj/argo-events:\
                      sha256-33f2261769bc73b375798d730884ea5c574a1b3e1503f75381053a4bafe7731e.sig"
        .parse()?;
    // dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    // dbg!(&manifest);

    Ok(())
}
