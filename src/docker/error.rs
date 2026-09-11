use url::Url;

use crate::docker::token_cache;

#[derive(Debug)]
pub enum Error {
    GetManifest(reqwest::Error),
    InvalidManifestUrl(url::ParseError),
    ExtractManifestBody(reqwest::Error),
    FailedManifestRequest(reqwest::StatusCode, String),
    DeserializeManifestBody(serde_json::Error, String),
    ParseManifestAcceptHeader(reqwest::header::InvalidHeaderValue),
    ManifestNotFound(Url),
    MissingDockerContentDigestHeader,
    ParseDockerContentDigestHeader(reqwest::header::ToStrError),

    InvalidTokenUrl(url::ParseError),
    GetToken(reqwest::Error),
    ExtractTokenBody(reqwest::Error),
    DeserializeToken(serde_json::Error, String),
    ParseAuthorizationHeader(reqwest::header::InvalidHeaderValue),
    InvalidImageUrl(crate::image::FromUrlError),
    FetchToken(token_cache::FetchError),
    StoreToken(token_cache::StoreError),

    GetBlob(reqwest::Error),
    InvalidBlobUrl(url::ParseError),
    ExtractBlobBody(reqwest::Error),
    FailedBlobRequest(reqwest::StatusCode, String),
    ParseBlobAcceptHeader(reqwest::header::InvalidHeaderValue),
    BlobNotFound(Url),
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
            Self::MissingDockerContentDigestHeader => {
                write!(f, "Missing Docker content digest header")
            }
            Self::ParseDockerContentDigestHeader(e) => {
                write!(f, "Failed to parse Docker content digest header: {e}")
            }

            Self::InvalidTokenUrl(e) => write!(f, "Invalid token URL: {e}"),
            Self::GetToken(e) => write!(f, "Failed to get token: {e}"),
            Self::ExtractTokenBody(e) => write!(f, "Failed to extract token body: {e}"),
            Self::DeserializeToken(e, s) => {
                write!(f, "Failed to deserialize token: {e}, body: {s}")
            }
            Self::ParseAuthorizationHeader(e) => {
                write!(f, "Failed to parse authorization header: {e}")
            }
            Self::InvalidImageUrl(e) => {
                write!(f, "Failed to parse image from url: {e}")
            }
            Self::FetchToken(e) => write!(f, "Failed to fetch token from cache: {e}"),
            Self::StoreToken(e) => write!(f, "Failed to store token in cache: {e}"),

            Self::GetBlob(e) => write!(f, "Failed to get blob: {e}"),
            Self::InvalidBlobUrl(e) => write!(f, "Invalid blob URL: {e}"),
            Self::ExtractBlobBody(e) => write!(f, "Failed to extract blob body: {e}"),
            Self::FailedBlobRequest(e, s) => {
                write!(f, "Failed blob request: status: {e}, body: {s}")
            }
            Self::ParseBlobAcceptHeader(e) => {
                write!(f, "Failed to parse blob accept header: {e}")
            }
            Self::BlobNotFound(u) => write!(f, "Blob at url {u} was not found"),
        }
    }
}

impl std::error::Error for Error {}
