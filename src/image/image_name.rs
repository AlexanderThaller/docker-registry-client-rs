use either::Either;

pub mod digest;
pub mod tag;

use digest::Digest;
use tag::Tag;

#[derive(Debug)]
pub enum FromStrError {
    MissingNameDigest,
    MissingNameTag,
    MissingDigest,
    ParseDigest(digest::FromStrError),
    ParseTag(tag::FromStrError),
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ImageName {
    pub name: String,
    pub identifier: Either<Tag, Digest>,
}

impl std::fmt::Display for FromStrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingNameDigest => f.write_str("missing name and digest"),
            Self::MissingNameTag => f.write_str("missing name and tag"),
            Self::MissingDigest => f.write_str("missing digest"),
            Self::ParseDigest(e) => write!(f, "error parsing digest: {e}"),
            Self::ParseTag(e) => write!(f, "error parsing tag: {e}"),
        }
    }
}

impl std::error::Error for FromStrError {}

impl std::fmt::Display for ImageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // A tag follows the name after a `:`, a digest after an `@` -- the
        // same split `from_str` makes, so what is printed parses back into the
        // same name. `name:sha256:...` is not a reference anything accepts.
        match &self.identifier {
            Either::Left(tag) => write!(f, "{name}:{tag}", name = self.name),
            Either::Right(digest) => write!(f, "{name}@{digest}", name = self.name),
        }
    }
}

impl std::str::FromStr for ImageName {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let has_digest = s.contains('@');

        if has_digest {
            let parts: Vec<&str> = s.split('@').collect();
            let name = (*parts.first().ok_or(Self::Err::MissingNameDigest)?).to_string();

            let digest = parts
                .get(1)
                .ok_or(Self::Err::MissingDigest)?
                .parse()
                .map_err(Self::Err::ParseDigest)?;

            Ok(Self {
                name,
                identifier: Either::Right(digest),
            })
        } else {
            let parts: Vec<&str> = s.split(':').collect();
            let name = (*parts.first().ok_or(Self::Err::MissingNameTag)?).to_string();

            let tag = parts
                .get(1)
                .unwrap_or(&"latest")
                .parse()
                .map_err(Self::Err::ParseTag)?;

            Ok(Self {
                name,
                identifier: Either::Left(tag),
            })
        }
    }
}
