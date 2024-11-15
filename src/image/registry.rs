#[derive(Debug)]
pub enum FromStrError {
    UnkownRegistry(String),
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Registry {
    DockerHub,
    Github,
    Google,
    K8s,
    Quay,
    RedHat,
    Microsoft,
}

impl std::fmt::Display for FromStrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnkownRegistry(s) => write!(f, "unknown registry: {s}"),
        }
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
            "gcr.io" => Ok(Registry::Google),
            "ghcr.io" => Ok(Registry::Github),
            "mcr.microsoft.com" => Ok(Registry::Microsoft),
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
            Self::Google => "gcr.io",
            Self::K8s => "registry.k8s.io",
            Self::Microsoft => "mcr.microsoft.com",
            Self::Quay => "quay.io",
            Self::RedHat => "registry.access.redhat.com",
        }
    }

    #[must_use]
    pub fn needs_authentication(&self) -> bool {
        match self {
            Self::DockerHub | Self::Github | Self::Quay => true,
            Self::RedHat | Self::K8s | Self::Google | Self::Microsoft => false,
        }
    }
}
