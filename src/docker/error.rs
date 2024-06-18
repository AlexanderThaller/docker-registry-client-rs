use url::Url;

#[derive(Debug)]
pub enum Error {
    GetManifest(reqwest::Error),
    InvalidManifestUrl(url::ParseError),
    ExtractManifestBody(reqwest::Error),
    FailedManifestRequest(reqwest::StatusCode, String),
    DeserializeManifestBody(serde_json::Error, String),
    ParseManifestAcceptHeader(reqwest::header::InvalidHeaderValue),
    ManifestNotFound(Url),

    InvalidGHCRTokenUrl(url::ParseError),
    GetGHCRToken(reqwest::Error),
    ExtractGHCRTokenBody(reqwest::Error),
    DeserializeGHCRToken(serde_json::Error, String),
    ParseGHCRAuthorizationHeader(reqwest::header::InvalidHeaderValue),

    InvalidDockerIoTokenUrl(url::ParseError),
    GetDockerIoToken(reqwest::Error),
    ExtractDockerIoTokenBody(reqwest::Error),
    DeserializeDockerIoToken(serde_json::Error, String),
    ParseDockerIoAuthorizationHeader(reqwest::header::InvalidHeaderValue),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GetManifest(e) => write!(f, "Failed to get manifest: {e}"),
            Self::InvalidManifestUrl(e) => write!(f, "Invalid manifest URL: {e}"),
            Self::ExtractManifestBody(e) => write!(f, "Failed to extract manifest body: {e}"),
            Self::FailedManifestRequest(e, s) => {
                write!(f, "Failed manifest request: status: {e}, body: {s}")
            }
            Self::DeserializeManifestBody(e, s) => {
                write!(f, "Failed to deserialize manifest body: {e}, body: {s}")
            }
            Self::ParseManifestAcceptHeader(e) => {
                write!(f, "Failed to parse manifest accept header: {e}")
            }
            Self::ManifestNotFound(u) => write!(f, "Manifest at url {u} was not found"),

            Self::InvalidGHCRTokenUrl(e) => write!(f, "Invalid GHCR token URL: {e}"),
            Self::GetGHCRToken(e) => write!(f, "Failed to get GHCR token: {e}"),
            Self::ExtractGHCRTokenBody(e) => write!(f, "Failed to extract GHCR token body: {e}"),
            Self::DeserializeGHCRToken(e, s) => {
                write!(f, "Failed to deserialize GHCR token: {e}, body: {s}")
            }
            Self::ParseGHCRAuthorizationHeader(e) => {
                write!(f, "Failed to parse GHCR authorization header: {e}")
            }

            Self::InvalidDockerIoTokenUrl(e) => write!(f, "Invalid Docker.io token URL: {e}"),
            Self::GetDockerIoToken(e) => write!(f, "Failed to get Docker.io token: {e}"),
            Self::ExtractDockerIoTokenBody(e) => {
                write!(f, "Failed to extract Docker.io token body: {e}")
            }
            Self::DeserializeDockerIoToken(e, s) => {
                write!(f, "Failed to deserialize Docker.io token: {e}, body: {s}")
            }
            Self::ParseDockerIoAuthorizationHeader(e) => {
                write!(f, "Failed to parse Docker.io authorization header: {e}")
            }
        }
    }
}

impl std::error::Error for Error {}
