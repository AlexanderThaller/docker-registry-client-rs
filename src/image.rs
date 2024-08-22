use serde::{
    Deserialize,
    Serialize,
};
use tracing::error;

#[allow(clippy::module_name_repetitions)]
pub mod image_name;
pub mod registry;

use image_name::ImageName;
use registry::Registry;

#[derive(Debug)]
pub enum FromStrError {
    MissingFirstComponent,
    UnsupportedImageName(String),
    ParseImageName(image_name::FromStrError),
    MissingRegistry,
    MissingImageName,
    ParseRegistry(registry::FromStrError),
    MissingRepository,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Image {
    pub registry: Registry,
    pub repository: Option<String>,
    pub image_name: ImageName,
}

impl std::fmt::Display for FromStrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFirstComponent => write!(f, "missing first component"),
            Self::UnsupportedImageName(s) => write!(f, "unsupported image name: {s}"),
            Self::ParseImageName(err) => write!(f, "{err}"),

            _ => todo!(),
        }
    }
}

impl std::error::Error for FromStrError {}

impl std::str::FromStr for Image {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let components = s.split('/').collect::<Vec<_>>();

        // alpine
        // prom/prometheus:v2.53.2
        // quay.io/openshift-community-operators/external-secrets-operator:v0.9.9
        // registry.access.redhat.com/ubi8:8.9
        match components.len() {
            0 => Err(FromStrError::MissingFirstComponent),

            // Case where only a docker image name is provided without a registry we default to
            // DockerHub as a registry and the library repository
            1 => {
                let image_name = s.parse().map_err(Self::Err::ParseImageName)?;

                Ok(Image {
                    registry: Registry::DockerHub,
                    repository: Some("library".to_string()),
                    image_name,
                })
            }

            // Case where we have a registry and a docker image name without a repository, or a
            // registry without a repository and a docker image name
            2 => {
                let registry = components
                    .first()
                    .expect("should never fail as we have 2 components")
                    .parse();

                match registry {
                    Err(_) => {
                        let repository = (*components
                            .first()
                            .expect("should never fail as we have 2 components"))
                        .to_string();

                        let image_name = components
                            .get(1)
                            .expect("should never fail as we have 2 components")
                            .parse()
                            .map_err(Self::Err::ParseImageName)?;

                        Ok(Image {
                            registry: Registry::DockerHub,
                            repository: Some(repository),
                            image_name,
                        })
                    }

                    Ok(registry) => {
                        let image_name = components
                            .get(1)
                            .expect("should never fail as we have 2 components")
                            .parse()
                            .map_err(Self::Err::ParseImageName)?;

                        Ok(Image {
                            registry,
                            repository: None,
                            image_name,
                        })
                    }
                }
            }

            // Case where we have a registry, a repository and a docker image name
            3 => {
                let registry = components
                    .first()
                    .expect("should never fail as we have 3 components")
                    .parse()
                    .map_err(Self::Err::ParseRegistry)?;

                let repository = (*components
                    .get(1)
                    .expect("should never fail as we have 3 components"))
                .to_string();

                let image_name = components
                    .get(2)
                    .expect("should never fail as we have 3 components")
                    .parse()
                    .map_err(Self::Err::ParseImageName)?;

                Ok(Image {
                    registry,
                    repository: Some(repository),
                    image_name,
                })
            }

            // Other cases are not supported
            _ => {
                let err = Self::Err::UnsupportedImageName(s.to_string());
                error!("{err}");

                Err(err)
            }
        }
    }
}

impl std::fmt::Display for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{registry}/{repository}{image_name}",
            registry = self.registry.registry_domain(),
            repository = match self.repository {
                Some(ref repository) => format!("{repository}/"),
                None => String::new(),
            },
            image_name = self.image_name
        )
    }
}

impl Serialize for Image {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(&self.to_string(), serializer)
    }
}

impl<'de> Deserialize<'de> for Image {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        let image_name = string.parse().map_err(serde::de::Error::custom)?;

        Ok(image_name)
    }
}

#[cfg(test)]
mod tests {
    mod from_str {
        use either::Either;
        use pretty_assertions::assert_eq;

        use crate::{
            Image,
            ImageName,
            Registry,
            Tag,
        };

        #[test]
        fn full_tag() {
            let expected = Image {
                registry: Registry::Github,
                repository: Some("aquasecurity".to_string()),
                image_name: ImageName {
                    name: "trivy".to_string(),
                    identifier: Either::Left(Tag::Specific("0.52.0".to_string())),
                },
            };

            let got = "ghcr.io/aquasecurity/trivy:0.52.0"
                .parse::<Image>()
                .unwrap();

            assert_eq!(expected, got);

            let expected = Image {
                registry: Registry::Quay,
                repository: Some("openshift-community-operators".to_string()),
                image_name: ImageName {
                    name: "external-secrets-operator".to_string(),
                    identifier: Either::Left(Tag::Specific("v0.9.9".to_string())),
                },
            };

            let got = "quay.io/openshift-community-operators/external-secrets-operator:v0.9.9"
                .parse::<Image>()
                .unwrap();

            assert_eq!(expected, got);
        }

        #[test]
        fn just_name() {
            let expected = Image {
                registry: Registry::DockerHub,
                repository: Some("library".to_string()),
                image_name: ImageName {
                    name: "archlinux".to_string(),
                    identifier: Either::Left(Tag::Latest),
                },
            };

            let got = "archlinux:latest".parse::<Image>().unwrap();

            assert_eq!(expected, got);
        }

        #[test]
        fn digest() {
            let expected = Image {
                registry: Registry::Quay,
                repository: Some("openshift-community-operators".to_string()),
                image_name: ImageName {
                    name: "external-secrets-operator".to_string(),
                    identifier: Either::Right(
                        "sha256:2247f14d217577b451727b3015f95e97d47941e96b99806f8589a34c43112ec3"
                            .parse()
                            .unwrap(),
                    ),
                },
            };

            let got = "quay.io/openshift-community-operators/external-secrets-operator@sha256:\
                       2247f14d217577b451727b3015f95e97d47941e96b99806f8589a34c43112ec3"
                .parse::<Image>()
                .unwrap();

            assert_eq!(expected, got);
        }

        mod dockerhub {
            use either::Either;
            use pretty_assertions::assert_eq;

            use crate::{
                Image,
                ImageName,
                Registry,
                Tag,
            };

            #[test]
            fn prometheus() {
                const INPUT: &str = "prom/prometheus:v2.53.2";

                let expected = Image {
                    registry: Registry::DockerHub,
                    repository: Some("prom".to_string()),
                    image_name: ImageName {
                        name: "prometheus".to_string(),
                        identifier: Either::Left(Tag::Specific("v2.53.2".to_string())),
                    },
                };

                let got = INPUT.parse::<Image>().unwrap();

                assert_eq!(expected, got);
            }
        }

        mod redhat {
            use either::Either;
            use pretty_assertions::assert_eq;

            use crate::{
                Image,
                ImageName,
                Registry,
                Tag,
            };

            #[test]
            fn ubi8() {
                const INPUT: &str = "registry.access.redhat.com/ubi8:8.9";

                let expected = Image {
                    registry: Registry::RedHat,
                    repository: None,
                    image_name: ImageName {
                        name: "ubi8".to_string(),
                        identifier: Either::Left(Tag::Specific("8.9".to_string())),
                    },
                };

                let got = INPUT.parse::<Image>().unwrap();

                assert_eq!(expected, got);
            }
        }
    }
}
