use either::Either;

#[derive(Debug)]
#[non_exhaustive]
pub enum FromStrError {}

#[derive(Debug, PartialEq, Clone)]
pub struct ImageName {
    pub(super) registry: Registry,
    pub(super) repository: String,
    pub(super) image_name: String,
    pub(super) identifier: Either<Tag, Digest>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Registry {
    DockerHub,
    Github,
    Quay,

    Specific(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Tag {
    Latest,

    Specific(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Digest(String);

impl std::fmt::Display for FromStrError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl std::error::Error for FromStrError {}

impl std::str::FromStr for ImageName {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let components = s.split('/').collect::<Vec<_>>();

        let registry = if components.len() == 3 {
            match *components.first().expect("already checked the length") {
                "docker.io" => Registry::DockerHub,
                "ghcr.io" => Registry::Github,
                "quay.io" => Registry::Quay,
                _ => Registry::Specific(components[0].to_string()),
            }
        } else {
            Registry::DockerHub
        };

        let repository = if components.len() == 1 {
            "library".to_string()
        } else {
            components[components.len() - 2].to_string()
        };

        let tag_or_digest = components[components.len() - 1];

        let (image_name, identifier) = if tag_or_digest.contains('@') {
            let digest_split = tag_or_digest.split('@').collect::<Vec<_>>();

            (
                digest_split[0].to_string(),
                Either::Right(Digest(digest_split[1].to_string())),
            )
        } else {
            let tag_split = tag_or_digest.split(':').collect::<Vec<_>>();

            let tag = if tag_split.len() == 1 || tag_split[1] == "latest" {
                Tag::Latest
            } else {
                Tag::Specific(tag_split[1].to_string())
            };

            (tag_split[0].to_string(), Either::Left(tag))
        };

        Ok(Self {
            registry,
            repository,
            image_name,
            identifier,
        })
    }
}

impl Registry {
    pub(super) fn registry_domain(&self) -> &str {
        match self {
            Self::DockerHub => "index.docker.io",
            Self::Github => "ghcr.io",
            Self::Quay => "quay.io",
            Self::Specific(s) => s,
        }
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Latest => write!(f, "latest"),
            Self::Specific(s) => write!(f, "{s}"),
        }
    }
}

impl std::fmt::Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for ImageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}:{}",
            self.registry.registry_domain(),
            self.repository,
            self.image_name,
            match &self.identifier {
                Either::Left(tag) => tag.to_string(),
                Either::Right(digest) => digest.to_string(),
            }
        )
    }
}

#[cfg(test)]
mod tests {
    mod from_str {
        use either::Either;
        use pretty_assertions::assert_eq;

        use crate::image_name::{
            Digest,
            ImageName,
            Registry,
            Tag,
        };

        #[test]
        fn full_tag() {
            let expected = ImageName {
                registry: Registry::Github,
                repository: "aquasecurity".to_string(),
                image_name: "trivy".to_string(),
                identifier: Either::Left(Tag::Specific("0.52.0".to_string())),
            };

            let got = "ghcr.io/aquasecurity/trivy:0.52.0"
                .parse::<ImageName>()
                .unwrap();

            assert_eq!(expected, got);

            let expected = ImageName {
                registry: Registry::Quay,
                repository: "openshift-community-operators".to_string(),
                image_name: "external-secrets-operator".to_string(),
                identifier: Either::Left(Tag::Specific("v0.9.9".to_string())),
            };

            let got = "quay.io/openshift-community-operators/external-secrets-operator:v0.9.9"
                .parse::<ImageName>()
                .unwrap();

            assert_eq!(expected, got);
        }

        #[test]
        fn just_name() {
            let expected = ImageName {
                registry: Registry::DockerHub,
                repository: "library".to_string(),
                image_name: "archlinux".to_string(),
                identifier: Either::Left(Tag::Latest),
            };

            let got = "archlinux:latest".parse::<ImageName>().unwrap();

            assert_eq!(expected, got);
        }

        #[test]
        fn digest() {
            let expected = ImageName {
                registry: Registry::Quay,
                repository: "openshift-community-operators".to_string(),
                image_name: "external-secrets-operator".to_string(),
                identifier: Either::Right(Digest(
                    "sha256:2247f14d217577b451727b3015f95e97d47941e96b99806f8589a34c43112ec3"
                        .to_string(),
                )),
            };

            let got = "quay.io/openshift-community-operators/external-secrets-operator@sha256:\
                       2247f14d217577b451727b3015f95e97d47941e96b99806f8589a34c43112ec3"
                .parse::<ImageName>()
                .unwrap();

            assert_eq!(expected, got);
        }
    }
}
