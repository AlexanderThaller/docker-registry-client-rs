use url::Url;

use crate::image::registry::Authentication;

/// A bearer token challenge of a Docker registry v2 API.
///
/// Either taken from the `WWW-Authenticate` header a registry answers with (see
/// [`Challenge::from_www_authenticate`]) or from what we already know about a
/// registry (see [`Authentication`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Challenge {
    realm: String,
    service: Option<String>,
}

impl Challenge {
    pub(super) fn new(realm: impl Into<String>, service: impl Into<String>) -> Self {
        Self {
            realm: realm.into(),
            service: Some(service.into()),
        }
    }

    /// Parses the value of a `WWW-Authenticate` header.
    ///
    /// Returns [`None`] if the header does not start with a bearer challenge
    /// that has a realm, e.g. because the registry uses basic authentication.
    ///
    /// The scope of the challenge is ignored as registries tend to answer with
    /// a placeholder scope (`scope="*"`). The scope we actually want is built
    /// from the image we are asking for instead.
    pub(super) fn from_www_authenticate(header: &str) -> Option<Self> {
        let parameters = header
            .trim_start()
            .strip_prefix_ignore_ascii_case("bearer")?
            .trim_start();

        let mut realm = None;
        let mut service = None;

        for (key, value) in split_parameters(parameters) {
            match key.to_ascii_lowercase().as_str() {
                "realm" => realm = Some(value),
                "service" => service = Some(value),
                _ => {}
            }
        }

        Some(Self {
            realm: realm?,
            service,
        })
    }

    /// The URL a token for the given scope has to be fetched from.
    ///
    /// # Errors
    /// Returns an error if the realm of the challenge is not a valid URL.
    pub(super) fn token_url(&self, scope: &str) -> Result<Url, url::ParseError> {
        let mut url = Url::parse(&self.realm)?;

        {
            let mut query = url.query_pairs_mut();
            query.append_pair("scope", scope);

            if let Some(service) = &self.service {
                query.append_pair("service", service);
            }
        }

        Ok(url)
    }
}

/// Whether the value of a `WWW-Authenticate` header is a basic challenge,
/// i.e. whether the registry wants the username and password on the request
/// itself rather than a token.
pub(super) fn is_basic(header: &str) -> bool {
    header
        .trim_start()
        .strip_prefix_ignore_ascii_case("basic")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
}

impl TryFrom<Authentication> for Challenge {
    type Error = ();

    fn try_from(authentication: Authentication) -> Result<Self, Self::Error> {
        match authentication {
            Authentication::Bearer { realm, service } => Ok(Self::new(realm, service)),
            Authentication::None | Authentication::Discover => Err(()),
        }
    }
}

/// Splits the comma separated `key="value"` parameters of a challenge.
///
/// Quoted values may contain commas so we can not simply split on them.
fn split_parameters(parameters: &str) -> impl Iterator<Item = (&str, String)> {
    let mut rest = parameters;

    std::iter::from_fn(move || {
        loop {
            if rest.is_empty() {
                return None;
            }

            let (parameter, remainder) = split_parameter(rest);
            rest = remainder;

            if let Some((key, value)) = parameter.split_once('=') {
                let value = value.trim().trim_matches('"').replace("\\\"", "\"");

                return Some((key.trim(), value));
            }
        }
    })
}

/// Splits off the first parameter and returns it together with the remaining
/// parameters.
fn split_parameter(parameters: &str) -> (&str, &str) {
    let mut quoted = false;
    let mut escaped = false;

    for (index, character) in parameters.char_indices() {
        match character {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            ',' if !quoted => return (&parameters[..index], &parameters[index + 1..]),
            _ => {}
        }
    }

    (parameters, "")
}

trait StripPrefixIgnoreAsciiCase {
    fn strip_prefix_ignore_ascii_case(&self, prefix: &str) -> Option<&str>;
}

impl StripPrefixIgnoreAsciiCase for str {
    fn strip_prefix_ignore_ascii_case(&self, prefix: &str) -> Option<&str> {
        let (start, rest) = self.split_at_checked(prefix.len())?;

        if start.eq_ignore_ascii_case(prefix) {
            Some(rest)
        } else {
            None
        }
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "using unwrap in tests is fine")]
mod tests {
    mod is_basic {
        use crate::docker::auth::is_basic;

        #[test]
        fn basic_challenges() {
            assert!(is_basic(r#"Basic realm="registry""#));
            assert!(is_basic(r#"basic realm="registry""#));
            assert!(is_basic("Basic"));
        }

        #[test]
        fn anything_else() {
            assert!(!is_basic(r#"Bearer realm="https://example.com/token""#));
            assert!(!is_basic("Basically"));
            assert!(!is_basic(""));
        }
    }

    mod from_www_authenticate {
        use pretty_assertions::assert_eq;

        use crate::docker::auth::Challenge;

        #[test]
        fn codeberg() {
            const INPUT: &str = r#"Bearer realm="https://codeberg.org/v2/token",service="container_registry",scope="*""#;

            let got = Challenge::from_www_authenticate(INPUT).unwrap();

            assert_eq!(
                Challenge::new("https://codeberg.org/v2/token", "container_registry"),
                got
            );
        }

        #[test]
        fn without_service() {
            const INPUT: &str = r#"Bearer realm="https://registry.example.com/token""#;

            let got = Challenge::from_www_authenticate(INPUT).unwrap();

            assert_eq!(
                "https://registry.example.com/token?scope=repository%3Aexample%3Apull",
                got.token_url("repository:example:pull").unwrap().as_str()
            );
        }

        #[test]
        fn quoted_comma() {
            const INPUT: &str = r#"bearer realm="https://example.com/token",error="expired, get a new one",service="example.com""#;

            let got = Challenge::from_www_authenticate(INPUT).unwrap();

            assert_eq!(
                Challenge::new("https://example.com/token", "example.com"),
                got
            );
        }

        #[test]
        fn not_a_bearer_challenge() {
            assert_eq!(
                None,
                Challenge::from_www_authenticate(r#"Basic realm="registry""#)
            );
            assert_eq!(None, Challenge::from_www_authenticate("Bearer"));
            assert_eq!(
                None,
                Challenge::from_www_authenticate(r#"Bearer service="example.com""#)
            );
        }
    }

    mod token_url {
        use pretty_assertions::assert_eq;

        use crate::docker::auth::Challenge;

        #[test]
        fn dockerhub() {
            let challenge = Challenge::new("https://auth.docker.io/token", "registry.docker.io");

            assert_eq!(
                "https://auth.docker.io/token?scope=repository%3Alibrary%2Falpine%3Apull&service=registry.docker.io",
                challenge.token_url("repository:library/alpine:pull").unwrap().as_str()
            );
        }

        #[test]
        fn realm_with_query() {
            let challenge = Challenge::new("https://example.com/token?foo=bar", "example.com");

            assert_eq!(
                "https://example.com/token?foo=bar&scope=repository%3Aexample%3Apull&service=example.com",
                challenge.token_url("repository:example:pull").unwrap().as_str()
            );
        }

        #[test]
        fn invalid_realm() {
            let challenge = Challenge::new("not a url", "example.com");

            assert!(challenge.token_url("repository:example:pull").is_err());
        }
    }
}
