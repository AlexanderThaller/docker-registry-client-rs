#[derive(Debug)]
pub enum FromStrError {
    EmptyRegistry,
    NotARegistryDomain(String),
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Registry {
    Codeberg,
    DockerHub,
    Github,
    Google,
    K8s,
    Microsoft,
    Quay,
    RedHat,

    /// Any registry that is not known ahead of time.
    ///
    /// Those registries are talked to using the plain Docker registry v2 API
    /// and their authentication is discovered at runtime (see
    /// [`Authentication::Discover`]).
    Other(String),
}

/// How a client has to authenticate against a registry.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Authentication {
    /// The registry is known to serve manifests without any authentication.
    None,

    /// The registry is known to hand out bearer tokens at the given token
    /// endpoint.
    Bearer {
        /// Endpoint that hands out the bearer token.
        realm: &'static str,

        /// Service the token is requested for.
        service: &'static str,
    },

    /// Nothing is known about the registry so the authentication is discovered
    /// from the `WWW-Authenticate` challenge the registry answers with.
    Discover,
}

impl std::fmt::Display for FromStrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyRegistry => write!(f, "registry is empty"),
            Self::NotARegistryDomain(s) => write!(f, "not a registry domain: {s}"),
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
            "" => Err(FromStrError::EmptyRegistry),

            "codeberg.org" => Ok(Registry::Codeberg),
            "docker.io" | "index.docker.io" => Ok(Registry::DockerHub),
            "gcr.io" => Ok(Registry::Google),
            "ghcr.io" => Ok(Registry::Github),
            "mcr.microsoft.com" => Ok(Registry::Microsoft),
            "quay.io" => Ok(Registry::Quay),
            "registry.access.redhat.com" => Ok(Registry::RedHat),
            "registry.k8s.io" => Ok(Registry::K8s),

            // Every other domain is treated as a registry we don't know
            // anything about instead of being rejected.
            _ if Self::is_registry_domain(s) => Ok(Registry::Other(s.to_string())),

            _ => Err(FromStrError::NotARegistryDomain(s.to_string())),
        }
    }
}

impl Registry {
    #[must_use]
    pub fn registry_domain(&self) -> &str {
        match self {
            Self::Codeberg => "codeberg.org",
            Self::DockerHub => "index.docker.io",
            Self::Github => "ghcr.io",
            Self::Google => "gcr.io",
            Self::K8s => "registry.k8s.io",
            Self::Microsoft => "mcr.microsoft.com",
            Self::Quay => "quay.io",
            Self::RedHat => "registry.access.redhat.com",
            Self::Other(domain) => domain,
        }
    }

    /// How to authenticate against this registry.
    #[must_use]
    pub fn authentication(&self) -> Authentication {
        match self {
            Self::Codeberg => Authentication::Bearer {
                realm: "https://codeberg.org/v2/token",
                service: "container_registry",
            },

            Self::DockerHub => Authentication::Bearer {
                realm: "https://auth.docker.io/token",
                service: "registry.docker.io",
            },

            Self::Github => Authentication::Bearer {
                realm: "https://ghcr.io/token",
                service: "ghcr.io",
            },

            Self::Quay => Authentication::Bearer {
                realm: "https://quay.io/v2/auth",
                service: "quay.io",
            },

            Self::Google | Self::K8s | Self::Microsoft | Self::RedHat => Authentication::None,

            Self::Other(_) => Authentication::Discover,
        }
    }

    /// Whether a token has to be fetched before the first request.
    ///
    /// This is only `true` for registries that are known to require
    /// authentication. Unknown registries return `false` as their
    /// authentication is discovered while talking to them.
    #[must_use]
    pub fn needs_authentication(&self) -> bool {
        matches!(self.authentication(), Authentication::Bearer { .. })
    }

    /// Whether the given string looks like the domain of a registry instead of
    /// the first component of an image path.
    ///
    /// This uses the same heuristic as the docker cli: anything that has a dot
    /// in it (`example.com`), has a port (`localhost:5000`) or is `localhost`
    /// is a registry domain, everything else is part of the image path
    /// (`prom/prometheus`).
    fn is_registry_domain(s: &str) -> bool {
        let host = s.split_once(':').map_or(s, |(host, _port)| host);

        host == "localhost" || host.contains('.')
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "using unwrap in tests is fine")]
mod tests {
    mod from_str {
        use pretty_assertions::assert_eq;

        use crate::image::registry::Registry;

        #[test]
        fn known() {
            assert_eq!(Registry::Codeberg, "codeberg.org".parse().unwrap());
            assert_eq!(Registry::DockerHub, "docker.io".parse().unwrap());
            assert_eq!(Registry::DockerHub, "index.docker.io".parse().unwrap());
            assert_eq!(Registry::Github, "ghcr.io".parse().unwrap());
        }

        #[test]
        fn unknown() {
            assert_eq!(
                Registry::Other("public.ecr.aws".to_string()),
                "public.ecr.aws".parse().unwrap()
            );

            assert_eq!(
                Registry::Other("localhost:5000".to_string()),
                "localhost:5000".parse().unwrap()
            );

            assert_eq!(
                Registry::Other("localhost".to_string()),
                "localhost".parse().unwrap()
            );
        }

        #[test]
        fn not_a_domain() {
            assert!("".parse::<Registry>().is_err());
            assert!("prom".parse::<Registry>().is_err());
            assert!("library".parse::<Registry>().is_err());
        }

        #[test]
        fn roundtrip() {
            let registry = Registry::Other("public.ecr.aws".to_string());

            assert_eq!(registry.to_string(), "public.ecr.aws");
            assert_eq!(registry, registry.to_string().parse().unwrap());
        }
    }
}
