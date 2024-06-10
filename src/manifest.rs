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
pub struct Image {
    #[serde(rename = "schemaVersion")]
    pub schema_version: SchemaVersion,

    #[serde(rename = "mediaType")]
    pub media_type: String,

    pub config: Config,

    pub layers: Vec<Layer>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct List {
    #[serde(rename = "schemaVersion")]
    schema_version: SchemaVersion,

    #[serde(rename = "mediaType")]
    media_type: String,

    pub manifests: Vec<Entry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Single {
    #[serde(rename = "schemaVersion")]
    pub schema_version: SchemaVersion,

    pub name: String,
    pub tag: String,
    pub architecture: Architecture,

    #[serde(rename = "fsLayers")]
    pub fs_layers: Vec<FsLayer>,

    pub history: Vec<History>,
}

#[derive(Debug)]
pub enum SchemaVersion {
    V1,
    V2,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub size: u64,
    pub digest: String,
    pub platform: Platform,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Platform {
    pub architecture: Architecture,
    pub os: OperatingSystem,

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
pub enum Architecture {
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
pub enum OperatingSystem {
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
pub struct Config {
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Layer {
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub size: u64,
    pub digest: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<Url>>,

    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FsLayer {
    #[serde(rename = "blobSum")]
    pub blob_sum: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct History {
    #[serde(
        rename = "v1Compatibility",
        deserialize_with = "deserialize_v1_compatibility"
    )]
    pub v1_compatibility: V1Compatibility,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct V1Compatibility {
    pub id: String,
    pub created: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_config: Option<ContainerConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContainerConfig {
    #[serde(rename = "Hostname")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    #[serde(rename = "Domainname")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domainname: Option<String>,

    #[serde(rename = "User")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    #[serde(rename = "AttachStdin")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach_stdin: Option<bool>,

    #[serde(rename = "AttachStdout")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach_stdout: Option<bool>,

    #[serde(rename = "AttachStderr")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach_stderr: Option<bool>,

    #[serde(rename = "Tty")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tty: Option<bool>,

    #[serde(rename = "OpenStdin")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_stdin: Option<bool>,

    #[serde(rename = "StdinOnce")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdin_once: Option<bool>,

    #[serde(rename = "Env")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,

    #[serde(rename = "Cmd")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<Vec<String>>,

    #[serde(rename = "Image")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    #[serde(rename = "Volumes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<BTreeMap<String, String>>,

    #[serde(rename = "WorkingDir")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,

    #[serde(rename = "Entrypoint")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<Vec<String>>,

    #[serde(rename = "OnBuild")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_build: Option<Vec<String>>,

    #[serde(rename = "Labels")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<BTreeMap<String, String>>,
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

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            Self::I386 => "386",
            Self::Amd64 => "amd64",
            Self::Arm => "arm",
            Self::Arm64 => "arm64",
            Self::Loong64 => "loong64",
            Self::Mips => "mips",
            Self::Mips64 => "mips64",
            Self::Mips64le => "mips64le",
            Self::Mipsle => "mipsle",
            Self::Ppc64 => "ppc64",
            Self::Ppc64le => "ppc64le",
            Self::Riscv64 => "riscv64",
            Self::S390x => "s390x",
            Self::Wasm => "wasm",
            Self::Unknown => "unknown",
        };

        f.write_str(out)
    }
}

impl std::fmt::Display for OperatingSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            Self::Aix => "aix",
            Self::Android => "android",
            Self::Darwin => "darwin",
            Self::Dragonfly => "dragonfly",
            Self::Freebsd => "freebsd",
            Self::Illumos => "illumos",
            Self::Ios => "ios",
            Self::Js => "js",
            Self::Linux => "linux",
            Self::Netbsd => "netbsd",
            Self::Openbsd => "openbsd",
            Self::Plan9 => "plan9",
            Self::Solaris => "solaris",
            Self::Wasip1 => "wasip1",
            Self::Windows => "windows",
            Self::Unknown => "unknown",
        };

        f.write_str(out)
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

            #[test]
            fn vaultwarden() {
                const INPUT: &str = include_str!("../resources/manifest/list/vaultwarden.json");

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
