# docker-registry-client

Communicate with Docker Registries to fetch image metadata.

## Registries

Images are parsed with the same rules the docker cli uses: the first component
of an image is treated as a registry when it has a dot in it
(`registry.example.com/image`), has a port (`localhost:5000/image`) or is
`localhost`, otherwise it is part of the image path (`prom/prometheus`).

Some registries are known to this crate so no round trip is wasted on finding
out how to authenticate against them:

- `codeberg.org`
- `docker.io` / `index.docker.io`
- `gcr.io`
- `ghcr.io`
- `mcr.microsoft.com`
- `quay.io`
- `registry.access.redhat.com`
- `registry.k8s.io`

Every other registry works as well. Requests are sent without a token first and
if the registry answers with `401 Unauthorized` the token endpoint is taken
from its `WWW-Authenticate` header, a token is fetched and the request is
retried:

```rust,no_run
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
use docker_registry_client::Client;

let client = Client::new();

let image = "public.ecr.aws/docker/library/alpine:3.20".parse()?;
let response = client.get_manifest(&image).await?;
# Ok(())
# }
```
