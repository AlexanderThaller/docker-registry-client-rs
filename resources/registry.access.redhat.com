2024/08/20 16:44:37 --> GET https://registry.access.redhat.com/v2/
2024/08/20 16:44:37 GET /v2/ HTTP/1.1
Host: registry.access.redhat.com
User-Agent: cosign/v2.2.4 (linux; amd64) go-containerregistry/v0.19.1
Accept-Encoding: gzip


2024/08/20 16:44:37 <-- 200 https://registry.access.redhat.com/v2/ (29.882743ms)
2024/08/20 16:44:37 HTTP/1.1 200 OK
Content-Length: 2
Cache-Control: max-age=0, no-cache, no-store
Connection: keep-alive
Content-Type: application/json
Date: Tue, 20 Aug 2024 14:44:37 GMT
Docker-Distribution-Api-Version: registry/2.0
Expires: Tue, 20 Aug 2024 14:44:37 GMT
Pragma: no-cache

{}
2024/08/20 16:44:37 --> GET https://registry.access.redhat.com/v2/ubi8/manifests/8.9
2024/08/20 16:44:37 GET /v2/ubi8/manifests/8.9 HTTP/1.1
Host: registry.access.redhat.com
User-Agent: cosign/v2.2.4 (linux; amd64) go-containerregistry/v0.19.1
Accept: application/vnd.docker.distribution.manifest.v1+json,application/vnd.docker.distribution.manifest.v1+prettyjws,application/vnd.docker.distribution.manifest.v2+json,application/vnd.oci.image.manifest.v1+json,application/vnd.docker.distribution.manifest.list.v2+json,application/vnd.oci.image.index.v1+json
Accept-Encoding: gzip


2024/08/20 16:44:37 <-- 200 https://registry.access.redhat.com/v2/ubi8/manifests/8.9 (273.449835ms)
2024/08/20 16:44:37 HTTP/1.1 200 OK
Cache-Control: max-age=0, no-cache, no-store
Connection: keep-alive
Content-Type: application/vnd.docker.distribution.manifest.list.v2+json
Date: Tue, 20 Aug 2024 14:44:37 GMT
Docker-Content-Digest: sha256:83068ea81dd02717b8e39b55cdeb2c1b2c9a3db260f01381b991755d44b15073
Expires: Tue, 20 Aug 2024 14:44:37 GMT
Pragma: no-cache
Strict-Transport-Security: max-age=63072000; preload
Vary: Accept-Encoding
X-Frame-Options: DENY

{
    "manifests": [
        {
            "digest": "sha256:04c033b3b44df719273c43cbb1bb69e59c0ebd04bbea51b3faf0adb0740c7c9b",
            "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
            "platform": {
                "architecture": "amd64",
                "os": "linux"
            },
            "size": 429
        },
        {
            "digest": "sha256:35190bf93a6567245e68bcb62b74f260eb65d352d5f897b781567118591c8520",
            "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
            "platform": {
                "architecture": "arm64",
                "os": "linux"
            },
            "size": 429
        },
        {
            "digest": "sha256:3711e13ea02656208859e96319b05de6d166f97d483df9514210b91475ff5eba",
            "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
            "platform": {
                "architecture": "ppc64le",
                "os": "linux"
            },
            "size": 429
        },
        {
            "digest": "sha256:72b611a8a588828b195842ea5829340616c6e4067ac2c459f7c0a7d8c5d4e3d3",
            "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
            "platform": {
                "architecture": "s390x",
                "os": "linux"
            },
            "size": 429
        }
    ],
    "mediaType": "application/vnd.docker.distribution.manifest.list.v2+json",
    "schemaVersion": 2
}
2024/08/20 16:44:37 --> GET https://registry.access.redhat.com/v2/ubi8/manifests/sha256-83068ea81dd02717b8e39b55cdeb2c1b2c9a3db260f01381b991755d44b15073.sig
2024/08/20 16:44:37 GET /v2/ubi8/manifests/sha256-83068ea81dd02717b8e39b55cdeb2c1b2c9a3db260f01381b991755d44b15073.sig HTTP/1.1
Host: registry.access.redhat.com
User-Agent: cosign/v2.2.4 (linux; amd64) go-containerregistry/v0.19.1
Accept: application/vnd.docker.distribution.manifest.v1+json,application/vnd.docker.distribution.manifest.v1+prettyjws,application/vnd.docker.distribution.manifest.v2+json,application/vnd.oci.image.manifest.v1+json,application/vnd.docker.distribution.manifest.list.v2+json,application/vnd.oci.image.index.v1+json
Accept-Encoding: gzip


2024/08/20 16:44:38 <-- 404 https://registry.access.redhat.com/v2/ubi8/manifests/sha256-83068ea81dd02717b8e39b55cdeb2c1b2c9a3db260f01381b991755d44b15073.sig (318.710658ms)
2024/08/20 16:44:38 HTTP/1.1 404 Not Found
Content-Length: 82
Cache-Control: max-age=0, no-cache, no-store
Connection: keep-alive
Content-Type: application/json
Date: Tue, 20 Aug 2024 14:44:38 GMT
Expires: Tue, 20 Aug 2024 14:44:38 GMT
Pragma: no-cache

{"errors":[{"code":"MANIFEST_UNKNOWN","detail":{},"message":"manifest unknown"}]}

Error: no signatures found
main.go:69: error during command execution: no signatures found
