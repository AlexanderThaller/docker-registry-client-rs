#[derive(Debug)]
pub enum FromStrError {}

#[derive(Debug, PartialEq, Clone)]
pub enum Tag {
    Latest,
    Specific(String),
}

impl std::fmt::Display for FromStrError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for FromStrError {}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Latest => write!(f, "latest"),
            Self::Specific(s) => write!(f, "{s}"),
        }
    }
}

impl std::str::FromStr for Tag {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "latest" => Ok(Self::Latest),
            s => Ok(Self::Specific(s.to_string())),
        }
    }
}
