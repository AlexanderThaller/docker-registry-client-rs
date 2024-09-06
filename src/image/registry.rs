#[derive(Debug)]
pub enum FromStrError {
    UnkownRegistry(String),
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Registry {
    DockerHub,
    Github,
    Quay,
    RedHat,
    K8s,
}

impl std::fmt::Display for FromStrError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for FromStrError {}

impl std::fmt::Display for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.registry_domain())
    }
}

impl std::str::FromStr for Registry {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "docker.io" | "index.docker.io" => Ok(Registry::DockerHub),
            "ghcr.io" => Ok(Registry::Github),
            "quay.io" => Ok(Registry::Quay),
            "registry.access.redhat.com" => Ok(Registry::RedHat),
            "registry.k8s.io" => Ok(Registry::K8s),

            _ => Err(FromStrError::UnkownRegistry(s.to_string())),
        }
    }
}

impl Registry {
    #[must_use]
    pub fn registry_domain(&self) -> &str {
        match self {
            Self::DockerHub => "index.docker.io",
            Self::Github => "ghcr.io",
            Self::Quay => "quay.io",
            Self::RedHat => "registry.access.redhat.com",
            Self::K8s => "registry.k8s.io",
        }
    }

    #[must_use]
    pub fn needs_authentication(&self) -> bool {
        match self {
            Self::DockerHub | Self::Github | Self::Quay => true,
            Self::RedHat | Self::K8s => false,
        }
    }
}
