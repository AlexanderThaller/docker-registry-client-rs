#![allow(dead_code)]

use std::collections::BTreeMap;

use chrono::{
    DateTime,
    Utc,
};
use serde::{
    de::{
        self,
        Deserializer,
    },
    Deserialize,
    Serialize,
};
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Manifest {
    Image(Image),
    List(List),
    Single(Single),
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Image {
    #[serde(rename = "schemaVersion")]
    schema_version: SchemaVersion,

    #[serde(rename = "mediaType")]
    media_type: String,

    config: Config,

    layers: Vec<Layer>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct List {
    #[serde(rename = "schemaVersion")]
    schema_version: SchemaVersion,

    #[serde(rename = "mediaType")]
    media_type: String,

    manifests: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Single {
    #[serde(rename = "schemaVersion")]
    schema_version: SchemaVersion,

    name: String,
    tag: String,
    architecture: Architecture,

    #[serde(rename = "fsLayers")]
    fs_layers: Vec<FsLayer>,

    history: Vec<History>,
}

#[derive(Debug)]
pub(super) enum SchemaVersion {
    V1,
    V2,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct ManifestEntry {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,
    platform: Platform,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Platform {
    architecture: Architecture,
    os: OperatingSystem,

    #[serde(rename = "os.version")]
    #[serde(skip_serializing_if = "Option::is_none")]
    os_version: Option<String>,

    #[serde(rename = "os.features")]
    #[serde(skip_serializing_if = "Option::is_none")]
    os_features: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    variant: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    features: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Architecture {
    #[serde(rename = "386")]
    I386,

    Amd64,
    Arm,
    Arm64,
    Loong64,
    Mips,
    Mips64,
    Mips64le,
    Mipsle,
    Ppc64,
    Ppc64le,
    Riscv64,
    S390x,
    Wasm,

    Unknown,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum OperatingSystem {
    Aix,
    Android,
    Darwin,
    Dragonfly,
    Freebsd,
    Illumos,
    Ios,
    Js,
    Linux,
    Netbsd,
    Openbsd,
    Plan9,
    Solaris,
    Wasip1,
    Windows,

    Unknown,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Config {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Layer {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    urls: Option<Vec<Url>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct FsLayer {
    #[serde(rename = "blobSum")]
    blob_sum: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct History {
    #[serde(
        rename = "v1Compatibility",
        deserialize_with = "deserialize_v1_compatibility"
    )]
    v1_compatibility: V1Compatibility,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct V1Compatibility {
    id: String,
    created: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    container: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    container_config: Option<ContainerConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct ContainerConfig {
    #[serde(rename = "Hostname")]
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<String>,

    #[serde(rename = "Domainname")]
    #[serde(skip_serializing_if = "Option::is_none")]
    domainname: Option<String>,

    #[serde(rename = "User")]
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,

    #[serde(rename = "AttachStdin")]
    #[serde(skip_serializing_if = "Option::is_none")]
    attach_stdin: Option<bool>,

    #[serde(rename = "AttachStdout")]
    #[serde(skip_serializing_if = "Option::is_none")]
    attach_stdout: Option<bool>,

    #[serde(rename = "AttachStderr")]
    #[serde(skip_serializing_if = "Option::is_none")]
    attach_stderr: Option<bool>,

    #[serde(rename = "Tty")]
    #[serde(skip_serializing_if = "Option::is_none")]
    tty: Option<bool>,

    #[serde(rename = "OpenStdin")]
    #[serde(skip_serializing_if = "Option::is_none")]
    open_stdin: Option<bool>,

    #[serde(rename = "StdinOnce")]
    #[serde(skip_serializing_if = "Option::is_none")]
    stdin_once: Option<bool>,

    #[serde(rename = "Env")]
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<Vec<String>>,

    #[serde(rename = "Cmd")]
    #[serde(skip_serializing_if = "Option::is_none")]
    cmd: Option<Vec<String>>,

    #[serde(rename = "Image")]
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<String>,

    #[serde(rename = "Volumes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    volumes: Option<BTreeMap<String, String>>,

    #[serde(rename = "WorkingDir")]
    #[serde(skip_serializing_if = "Option::is_none")]
    working_dir: Option<String>,

    #[serde(rename = "Entrypoint")]
    #[serde(skip_serializing_if = "Option::is_none")]
    entrypoint: Option<Vec<String>>,

    #[serde(rename = "OnBuild")]
    #[serde(skip_serializing_if = "Option::is_none")]
    on_build: Option<Vec<String>>,

    #[serde(rename = "Labels")]
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<BTreeMap<String, String>>,
}

fn deserialize_v1_compatibility<'de, D>(deserializer: D) -> Result<V1Compatibility, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    serde_json::from_str(&s).map_err(de::Error::custom)
}

impl Serialize for SchemaVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SchemaVersion::V1 => 1i32.serialize(serializer),
            SchemaVersion::V2 => 2i32.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = i32::deserialize(deserializer)?;

        match value {
            1 => Ok(Self::V1),
            2 => Ok(Self::V2),
            _ => Err(serde::de::Error::custom("invalid enum value")),
        }
    }
}

#[cfg(test)]
mod tests {
    mod list {
        mod deserialize {
            use crate::manifest::List;

            #[test]
            fn example() {
                const INPUT: &str = include_str!("../resources/manifest/list/example.json");

                let out: List = serde_json::from_str(INPUT).unwrap();

                insta::assert_json_snapshot!(out);
            }

            #[test]
            fn trivy() {
                const INPUT: &str = include_str!("../resources/manifest/list/trivy.json");

                let out: List = serde_json::from_str(INPUT).unwrap();

                insta::assert_json_snapshot!(out);
            }
        }
    }

    mod image {
        mod deserialize {
            use crate::manifest::Image;

            #[test]
            fn example() {
                const INPUT: &str = include_str!("../resources/manifest/image/example.json");

                let out: Image = serde_json::from_str(INPUT).unwrap();

                insta::assert_json_snapshot!(out);
            }
        }
    }

    mod single {
        mod deserialize {
            use crate::manifest::Single;

            #[test]
            fn example() {
                const INPUT: &str =
                    include_str!("../resources/manifest/single/external-secrets-operator.json");

                let out: Single = serde_json::from_str(INPUT).unwrap();

                insta::assert_json_snapshot!(out);
            }
        }
    }

    mod v1_compatibility {
        mod deserialize {
            use crate::manifest::V1Compatibility;

            #[test]
            fn example() {
                const INPUT: &str = include_str!(
                    "../resources/manifest/v1_compatibility/external-secrets-operator.json"
                );

                let out: Vec<V1Compatibility> = serde_json::from_str(INPUT).unwrap();

                insta::assert_json_snapshot!(out);
            }
        }
    }
}
