use docker_registry_client::docker::Client;

#[tokio::main]
async fn main() -> Result<(), eyre::Error> {
    let client = Client::new();

    let image_name = "archlinux:latest".parse()?;
    dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    dbg!(&manifest);

    let image_name = "index.docker.io/grafana/grafana:\
                      sha256-b018257986b51ee1179f5a416c2c90e9698941d3f3104ccecfdc250e9bf07555.sig"
        .parse()?;
    dbg!(&image_name);
    let manifest = client.get_manifest(&image_name).await?;
    dbg!(&manifest);

    Ok(())
}
