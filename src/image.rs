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

#[derive(Debug)]
pub enum FromUrlError {}

#[derive(Debug, PartialEq, Clone)]
pub struct Image {
    pub registry: Registry,
    pub namespace: Option<String>,
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

impl std::fmt::Display for FromUrlError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for FromUrlError {}

impl std::str::FromStr for Image {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let components = s.split('/').collect::<Vec<_>>();

        // alpine
        // prom/prometheus:v2.53.2
        // quay.io/openshift-community-operators/external-secrets-operator:v0.9.9
        // registry.access.redhat.com/ubi8:8.9
        match components.as_slice() {
            [] => Err(FromStrError::MissingFirstComponent),

            // Case where only a docker image name is provided without a registry we default to
            // DockerHub as a registry and the library repository
            [image_name] => {
                let image_name = image_name.parse().map_err(Self::Err::ParseImageName)?;

                Ok(Image {
                    registry: Registry::DockerHub,
                    namespace: None,
                    repository: Some("library".to_string()),
                    image_name,
                })
            }

            // Case where we have a registry and a docker image name without a repository, or a
            // registry without a repository and a docker image name
            [registry_or_repository, image_name] => {
                let result = registry_or_repository.parse();

                if let Ok(registry) = result {
                    let image_name = image_name.parse().map_err(Self::Err::ParseImageName)?;

                    Ok(Image {
                        registry,
                        namespace: None,
                        repository: None,
                        image_name,
                    })
                } else {
                    // Case where we have a repository and a docker image name as the registry
                    // could not be parsed
                    let repository = (*registry_or_repository).to_string();
                    let image_name = image_name.parse().map_err(Self::Err::ParseImageName)?;

                    Ok(Image {
                        registry: Registry::DockerHub,
                        namespace: None,
                        repository: Some(repository),
                        image_name,
                    })
                }
            }

            // Case where we have a registry, a repository and a docker image name
            [registry, repository, image_name] => {
                let registry = registry.parse().map_err(Self::Err::ParseRegistry)?;
                let repository = (*repository).to_string();
                let image_name = image_name.parse().map_err(Self::Err::ParseImageName)?;

                Ok(Image {
                    registry,
                    namespace: None,
                    repository: Some(repository),
                    image_name,
                })
            }

            // Case where we have a registry, a repository and a docker image name and a namespace
            [registry, namespace, repository, image_name] => {
                let registry = registry.parse().map_err(Self::Err::ParseRegistry)?;
                let namespace = (*namespace).to_string();
                let repository = (*repository).to_string();
                let image_name = image_name.parse().map_err(Self::Err::ParseImageName)?;

                Ok(Image {
                    registry,
                    namespace: Some(namespace),
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
            "{registry}/{namespace}{repository}{image_name}",
            registry = self.registry.registry_domain(),
            namespace = match self.namespace {
                Some(ref namespace) => format!("{namespace}/"),
                None => String::new(),
            },
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
                namespace: None,
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
                namespace: None,
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
                namespace: None,
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
                namespace: None,
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
                    namespace: None,
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
                    namespace: None,
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

        mod k8s {
            use either::Either;
            use pretty_assertions::assert_eq;

            use crate::{
                Image,
                ImageName,
                Registry,
                Tag,
            };

            #[test]
            fn vpa() {
                const INPUT: &str = "registry.k8s.io/autoscaling/vpa-recommender:1.1.2";

                let expected = Image {
                    registry: Registry::K8s,
                    namespace: None,
                    repository: Some("autoscaling".to_string()),
                    image_name: ImageName {
                        name: "vpa-recommender".to_string(),
                        identifier: Either::Left(Tag::Specific("1.1.2".to_string())),
                    },
                };

                let got = INPUT.parse::<Image>().unwrap();

                assert_eq!(expected, got);
            }
        }

        mod github {
            use either::Either;
            use pretty_assertions::assert_eq;

            use crate::{
                Image,
                ImageName,
                Registry,
                Tag,
            };

            #[test]
            fn cosign() {
                const INPUT: &str = "ghcr.io/sigstore/cosign/cosign:v2.4.0";

                let expected = Image {
                    registry: Registry::Github,
                    namespace: Some("sigstore".to_string()),
                    repository: Some("cosign".to_string()),
                    image_name: ImageName {
                        name: "cosign".to_string(),
                        identifier: Either::Left(Tag::Specific("v2.4.0".to_string())),
                    },
                };

                let got = INPUT.parse::<Image>().unwrap();

                assert_eq!(expected, got);
            }
        }
    }
}
