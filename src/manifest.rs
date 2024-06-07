use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub(super) struct Image {
    #[serde(rename = "schemaVersion")]
    schema_version: SchemaVersion,

    #[serde(rename = "mediaType")]
    media_type: String,

    config: Config,

    layers: Vec<Layer>,
}

#[derive(Debug, Deserialize)]
pub(super) struct List {
    #[serde(rename = "schemaVersion")]
    schema_version: SchemaVersion,

    #[serde(rename = "mediaType")]
    media_type: String,

    manifests: Vec<Manifest>,
}

#[derive(Debug)]
pub(super) enum SchemaVersion {
    V2,
}

#[derive(Debug, Deserialize)]
pub(super) struct Manifest {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,
    platform: Platform,
}

#[derive(Debug, Deserialize)]
pub(super) struct Platform {
    architecture: Architecture,
    os: OperatingSystem,

    #[serde(rename = "os.version")]
    os_version: Option<String>,

    #[serde(rename = "os.features")]
    os_features: Option<String>,

    variant: Option<String>,

    #[serde(default)]
    features: Vec<String>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub(super) struct Config {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct Layer {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,

    #[serde(default)]
    urls: Vec<Url>,
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = i32::deserialize(deserializer)?;

        match value {
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

                insta::assert_debug_snapshot!(out);
            }

            #[test]
            fn trivy() {
                const INPUT: &str = include_str!("../resources/manifest/list/trivy.json");

                let out: List = serde_json::from_str(INPUT).unwrap();

                insta::assert_debug_snapshot!(out);
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

                insta::assert_debug_snapshot!(out);
            }
        }
    }
}
