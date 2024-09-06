use std::{
    collections::HashMap,
    sync::{
        Arc,
        RwLock,
    },
};

use chrono::{
    DateTime,
    Utc,
};
use reqwest::header::HeaderMap;
use serde::{
    Deserialize,
    Serialize,
};
use tracing::{
    info_span,
    Instrument,
};

#[cfg(feature = "redis_cache")]
use redis::AsyncCommands;

use crate::Image;

#[cfg(feature = "redis_cache")]
const REDIS_PREFIX: &str = "docker-registry-client:token";

#[derive(Debug, Eq, PartialEq, Hash)]
pub(super) struct CacheKey {
    image: Image,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub(super) struct Token {
    #[serde(rename = "token")]
    value: String,
    expires_in: Option<i64>,
    issued_at: Option<DateTime<Utc>>,
}

#[async_trait::async_trait]
pub(super) trait Cache: std::fmt::Debug + Send + Sync + dyn_clone::DynClone {
    async fn fetch(&self, key: &CacheKey) -> Option<Token>;
    async fn store(&self, key: CacheKey, token: Token);
}

dyn_clone::clone_trait_object!(Cache);

/// `NoCache` is a token cache that does not cache tokens.
#[derive(Debug, Default, Clone)]
pub(super) struct NoCache;

/// `MemoryTokenCache` is a token cache that caches tokens in memory.
#[derive(Debug, Default, Clone)]
pub(super) struct MemoryTokenCache {
    cache: Arc<RwLock<HashMap<CacheKey, Token>>>,
}

#[cfg(feature = "redis_cache")]
/// `RedisCache` is a token cache that caches tokens in Redis.
#[derive(Debug, Clone)]
pub(super) struct RedisCache {
    client: redis::Client,
}

impl std::fmt::Display for CacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let registry = self.image.registry.to_string();
        let namespace = self.image.namespace.as_ref();
        let repository = self.image.repository.as_ref();
        let image_name = &self.image.image_name.name;

        write!(f, "{registry}{namespace:?}{repository:?}{image_name}")
    }
}

#[async_trait::async_trait]
impl Cache for NoCache {
    async fn fetch(&self, _key: &CacheKey) -> Option<Token> {
        None
    }

    async fn store(&self, _key: CacheKey, _token: Token) {}
}

#[async_trait::async_trait]
impl Cache for MemoryTokenCache {
    #[tracing::instrument]
    async fn fetch(&self, key: &CacheKey) -> Option<Token> {
        self.cache
            .read()
            .expect("failed to get read lock")
            .get(key)
            .cloned()
            .and_then(|token| {
                if let Some(expires_in) = token.expires_in {
                    token
                        .issued_at
                        .map(|issued_at| issued_at + chrono::Duration::seconds(expires_in))
                        .and_then(|expires_at| {
                            if expires_at < Utc::now() {
                                None
                            } else {
                                Some(token)
                            }
                        })
                } else {
                    Some(token)
                }
            })
    }

    #[tracing::instrument]
    async fn store(&self, key: CacheKey, token: Token) {
        self.cache
            .write()
            .expect("failed to get write lock")
            .insert(key, token);
    }
}

impl From<&Image> for CacheKey {
    fn from(image: &Image) -> Self {
        Self {
            image: image.clone(),
        }
    }
}

impl TryInto<HeaderMap> for Token {
    type Error = reqwest::header::InvalidHeaderValue;

    fn try_into(self) -> Result<HeaderMap, Self::Error> {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", self.value).parse()?);

        Ok(headers)
    }
}

#[cfg(feature = "redis_cache")]
impl RedisCache {
    #[must_use]
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }
}

#[cfg(feature = "redis_cache")]
#[async_trait::async_trait]
impl Cache for RedisCache {
    #[tracing::instrument]
    async fn fetch(&self, key: &CacheKey) -> Option<Token> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .instrument(info_span!("get redis connection"))
            .await
            .expect("failed to get connection");

        let key = format!("{REDIS_PREFIX}:{key}");

        let exists: bool = connection
            .exists(&key)
            .instrument(info_span!("check if key exists"))
            .await
            .expect("failed to check if key exists");

        if !exists {
            return None;
        }

        let value: String = connection
            .get(&key)
            .instrument(info_span!("get value"))
            .await
            .expect("failed to get value");

        let token = serde_json::from_str(&value).expect("failed to deserialize token");

        Some(token)
    }

    #[tracing::instrument]
    async fn store(&self, key: CacheKey, token: Token) {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .instrument(info_span!("get redis connection"))
            .await
            .expect("failed to get connection");

        let key = format!("{REDIS_PREFIX}:{key}");

        let value = serde_json::to_string(&token).expect("failed to serialize token");

        connection
            .set::<&String, String, String>(&key, value)
            .instrument(info_span!("set value"))
            .await
            .expect("failed to set value");

        if let Some(expires_in) = token.expires_in {
            connection
                .expire::<&String, String>(&key, expires_in)
                .instrument(info_span!("set expire"))
                .await
                .expect("failed to set expiration");
        }
    }
}

#[cfg(test)]
mod tests {
    mod token {
        mod deserialize {
            use crate::docker::Token;

            #[test]
            fn github() {
                const INPUT: &str =
                    r#"{"token":"djE6c2lnc3RvcmUvY29zaWduL2Nvc2lnbjoxNzI1NDM2OTAwNTczODMyMzM2"}"#;

                let got = serde_json::from_str::<Token>(INPUT).unwrap();
                insta::assert_json_snapshot!(got);
            }

            #[test]
            fn dockerhub() {
                const INPUT: &str = r#"{"token":"eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsIng1YyI6WyJNSUlFRmpDQ0F2NmdBd0lCQWdJVVZOajJRbU1JWnUzeGl0NUJ1RTlvRWdoVU5KUXdEUVlKS29aSWh2Y05BUUVMQlFBd2dZWXhDekFKQmdOVkJBWVRBbFZUTVJNd0VRWURWUVFJRXdwRFlXeHBabTl5Ym1saE1SSXdFQVlEVlFRSEV3bFFZV3h2SUVGc2RHOHhGVEFUQmdOVkJBb1RERVJ2WTJ0bGNpd2dTVzVqTGpFVU1CSUdBMVVFQ3hNTFJXNW5hVzVsWlhKcGJtY3hJVEFmQmdOVkJBTVRHRVJ2WTJ0bGNpd2dTVzVqTGlCRmJtY2dVbTl2ZENCRFFUQWVGdzB5TkRBeE1UWXdOak0yTURCYUZ3MHlOVEF4TVRVd05qTTJNREJhTUlHRk1Rc3dDUVlEVlFRR0V3SlZVekVUTUJFR0ExVUVDQk1LUTJGc2FXWnZjbTVwWVRFU01CQUdBMVVFQnhNSlVHRnNieUJCYkhSdk1SVXdFd1lEVlFRS0V3eEViMk5yWlhJc0lFbHVZeTR4RkRBU0JnTlZCQXNUQzBWdVoybHVaV1Z5YVc1bk1TQXdIZ1lEVlFRREV4ZEViMk5yWlhJc0lFbHVZeTRnUlc1bklFcFhWQ0JEUVRDQ0FTSXdEUVlKS29aSWh2Y05BUUVCQlFBRGdnRVBBRENDQVFvQ2dnRUJBTWI4eHR6ZDQ1UWdYekV0bWMxUEJsdWNGUnlzSUF4UUJCN3lSNjdJemdMd05IS24rbUdKTzV5alh6amtLZm5zWm1JRURnZFlraEpBbGNYYTdQa1BFaCtqcTRGNWNaaWtkTmFUQmM3alNkTFJzTVlVa3dwWTl4WUVqYitCYnVGUWVxa0R2RXNqbFJJTzRQK0FsRlhNMDhMYlpIZ3hFWUdkbFk3WFlhT1BLMmE1aUd2eVFRb09GVmZjZDd2ekhaREVBMHZqVmU1M0xLdjVMYmh6TzcxZHRxS0RwNEhnVWR5N1pENDFNN3I1bTd5eE1LeFNpQmJHZTFvem5Wamh1ck5GNHdGSml5bVU4YkhTV2tVTXVLQ3JTbEd4d1NCZFVZNDRyaEh2UW5zYmgzUFF2TUZTWTQ4REdoNFhUUldjSzFWUVlSTnA2ZWFXUVg1RUpJSXVJbjJQOVBzQ0F3RUFBYU43TUhrd0RnWURWUjBQQVFIL0JBUURBZ0dtTUJNR0ExVWRKUVFNTUFvR0NDc0dBUVVGQndNQk1CSUdBMVVkRXdFQi93UUlNQVlCQWY4Q0FRQXdIUVlEVlIwT0JCWUVGSnVRYXZTZHVScm5kRXhLTTAwV2Z2czh5T0RaTUI4R0ExVWRJd1FZTUJhQUZGSGVwRE9ZQ0Y5Qnc5dXNsY0RVUW5CalU3MS9NQTBHQ1NxR1NJYjNEUUVCQ3dVQUE0SUJBUUNDWW0xUVorUUZ1RVhkSWpiNkg4bXNyVFBRSlNnR0JpWDFXSC9QRnpqZlJGeHc3dTdDazBRb0FXZVNqV3JWQWtlVlZQN3J2REpwZ0ZoeUljdzNzMXRPVjN0OGp3cXJTUmc2R285dUd2OG9IWUlLTm9YaDErUFVDTG44b0JwYUJsOUxzSWtsL2FHMG9lY0hrcDVLYmtBNjN6eTFxSUVXNFAzWVJLSk9hSGoxYWFiOXJLc3lRSHF6SUl4TnlDRVVINTMwU1B4RUNMbE53YWVKTDVmNXIxUW5wSi9GM3Q5Vk8xZ0Y2RFpiNitPczdTV29ocGhWZlRCOERkL1VjSk1VOGp2YlF3MWRVREkwelNEdXo2aHNJbGdITk0yak04M0lOS1VqNjNaRDMwRG15ejQvczFFdGgyQmlKK2RHdnFpQkRzaWhaR0tyQnJzUzhWVkRBd3hDeDVRMyJdfQ.eyJhY2Nlc3MiOlt7ImFjdGlvbnMiOlsicHVsbCJdLCJuYW1lIjoibGlicmFyeS9hbHBpbmUiLCJwYXJhbWV0ZXJzIjp7InB1bGxfbGltaXQiOiIxMDAiLCJwdWxsX2xpbWl0X2ludGVydmFsIjoiMjE2MDAifSwidHlwZSI6InJlcG9zaXRvcnkifV0sImF1ZCI6InJlZ2lzdHJ5LmRvY2tlci5pbyIsImV4cCI6MTcyNTQzNzIwMSwiaWF0IjoxNzI1NDM2OTAxLCJpc3MiOiJhdXRoLmRvY2tlci5pbyIsImp0aSI6ImRja3JfanRpX1dVVnQ2RldmdGRyNUU4UUNTTmwxb1M3NG1LUT0iLCJuYmYiOjE3MjU0MzY2MDEsInN1YiI6IiJ9.SMcP5s3bLnOmC0X3pG4b9N8PD3Dtpk5xovCzdcje-79O-lRx0Tqa2zruCauMENetmmq2CoxZOsY1ostvrAfZaDp2f_KkjaftymHhvgx60ppxIZuHIwZJjt9v7XprJuK8OZ1QxHL-HKp5nw8dRwa-IyScs-YevTz7PMD5B7uy36AZs1db2FwT_6Ygzhkvuq3OKgr70Mie2WHAoyShEJs7V6b9tywiSXPEgu2Nf7VSrj6PeZOi__IQl0wXo7w4j7C8o6ijUAotPRfMoQUXN1j9_4ql-yJZpp5jVmLsEOaOVKPjoIvnyDWYs9KVluUF4NKgvOydgIP-MeccXXIFOg4bhA","access_token":"eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsIng1YyI6WyJNSUlFRmpDQ0F2NmdBd0lCQWdJVVZOajJRbU1JWnUzeGl0NUJ1RTlvRWdoVU5KUXdEUVlKS29aSWh2Y05BUUVMQlFBd2dZWXhDekFKQmdOVkJBWVRBbFZUTVJNd0VRWURWUVFJRXdwRFlXeHBabTl5Ym1saE1SSXdFQVlEVlFRSEV3bFFZV3h2SUVGc2RHOHhGVEFUQmdOVkJBb1RERVJ2WTJ0bGNpd2dTVzVqTGpFVU1CSUdBMVVFQ3hNTFJXNW5hVzVsWlhKcGJtY3hJVEFmQmdOVkJBTVRHRVJ2WTJ0bGNpd2dTVzVqTGlCRmJtY2dVbTl2ZENCRFFUQWVGdzB5TkRBeE1UWXdOak0yTURCYUZ3MHlOVEF4TVRVd05qTTJNREJhTUlHRk1Rc3dDUVlEVlFRR0V3SlZVekVUTUJFR0ExVUVDQk1LUTJGc2FXWnZjbTVwWVRFU01CQUdBMVVFQnhNSlVHRnNieUJCYkhSdk1SVXdFd1lEVlFRS0V3eEViMk5yWlhJc0lFbHVZeTR4RkRBU0JnTlZCQXNUQzBWdVoybHVaV1Z5YVc1bk1TQXdIZ1lEVlFRREV4ZEViMk5yWlhJc0lFbHVZeTRnUlc1bklFcFhWQ0JEUVRDQ0FTSXdEUVlKS29aSWh2Y05BUUVCQlFBRGdnRVBBRENDQVFvQ2dnRUJBTWI4eHR6ZDQ1UWdYekV0bWMxUEJsdWNGUnlzSUF4UUJCN3lSNjdJemdMd05IS24rbUdKTzV5alh6amtLZm5zWm1JRURnZFlraEpBbGNYYTdQa1BFaCtqcTRGNWNaaWtkTmFUQmM3alNkTFJzTVlVa3dwWTl4WUVqYitCYnVGUWVxa0R2RXNqbFJJTzRQK0FsRlhNMDhMYlpIZ3hFWUdkbFk3WFlhT1BLMmE1aUd2eVFRb09GVmZjZDd2ekhaREVBMHZqVmU1M0xLdjVMYmh6TzcxZHRxS0RwNEhnVWR5N1pENDFNN3I1bTd5eE1LeFNpQmJHZTFvem5Wamh1ck5GNHdGSml5bVU4YkhTV2tVTXVLQ3JTbEd4d1NCZFVZNDRyaEh2UW5zYmgzUFF2TUZTWTQ4REdoNFhUUldjSzFWUVlSTnA2ZWFXUVg1RUpJSXVJbjJQOVBzQ0F3RUFBYU43TUhrd0RnWURWUjBQQVFIL0JBUURBZ0dtTUJNR0ExVWRKUVFNTUFvR0NDc0dBUVVGQndNQk1CSUdBMVVkRXdFQi93UUlNQVlCQWY4Q0FRQXdIUVlEVlIwT0JCWUVGSnVRYXZTZHVScm5kRXhLTTAwV2Z2czh5T0RaTUI4R0ExVWRJd1FZTUJhQUZGSGVwRE9ZQ0Y5Qnc5dXNsY0RVUW5CalU3MS9NQTBHQ1NxR1NJYjNEUUVCQ3dVQUE0SUJBUUNDWW0xUVorUUZ1RVhkSWpiNkg4bXNyVFBRSlNnR0JpWDFXSC9QRnpqZlJGeHc3dTdDazBRb0FXZVNqV3JWQWtlVlZQN3J2REpwZ0ZoeUljdzNzMXRPVjN0OGp3cXJTUmc2R285dUd2OG9IWUlLTm9YaDErUFVDTG44b0JwYUJsOUxzSWtsL2FHMG9lY0hrcDVLYmtBNjN6eTFxSUVXNFAzWVJLSk9hSGoxYWFiOXJLc3lRSHF6SUl4TnlDRVVINTMwU1B4RUNMbE53YWVKTDVmNXIxUW5wSi9GM3Q5Vk8xZ0Y2RFpiNitPczdTV29ocGhWZlRCOERkL1VjSk1VOGp2YlF3MWRVREkwelNEdXo2aHNJbGdITk0yak04M0lOS1VqNjNaRDMwRG15ejQvczFFdGgyQmlKK2RHdnFpQkRzaWhaR0tyQnJzUzhWVkRBd3hDeDVRMyJdfQ.eyJhY2Nlc3MiOlt7ImFjdGlvbnMiOlsicHVsbCJdLCJuYW1lIjoibGlicmFyeS9hbHBpbmUiLCJwYXJhbWV0ZXJzIjp7InB1bGxfbGltaXQiOiIxMDAiLCJwdWxsX2xpbWl0X2ludGVydmFsIjoiMjE2MDAifSwidHlwZSI6InJlcG9zaXRvcnkifV0sImF1ZCI6InJlZ2lzdHJ5LmRvY2tlci5pbyIsImV4cCI6MTcyNTQzNzIwMSwiaWF0IjoxNzI1NDM2OTAxLCJpc3MiOiJhdXRoLmRvY2tlci5pbyIsImp0aSI6ImRja3JfanRpX1dVVnQ2RldmdGRyNUU4UUNTTmwxb1M3NG1LUT0iLCJuYmYiOjE3MjU0MzY2MDEsInN1YiI6IiJ9.SMcP5s3bLnOmC0X3pG4b9N8PD3Dtpk5xovCzdcje-79O-lRx0Tqa2zruCauMENetmmq2CoxZOsY1ostvrAfZaDp2f_KkjaftymHhvgx60ppxIZuHIwZJjt9v7XprJuK8OZ1QxHL-HKp5nw8dRwa-IyScs-YevTz7PMD5B7uy36AZs1db2FwT_6Ygzhkvuq3OKgr70Mie2WHAoyShEJs7V6b9tywiSXPEgu2Nf7VSrj6PeZOi__IQl0wXo7w4j7C8o6ijUAotPRfMoQUXN1j9_4ql-yJZpp5jVmLsEOaOVKPjoIvnyDWYs9KVluUF4NKgvOydgIP-MeccXXIFOg4bhA","expires_in":300,"issued_at":"2024-09-04T08:01:41.048681016Z"}"#;

                let got = serde_json::from_str::<Token>(INPUT).unwrap();
                insta::assert_json_snapshot!(got);
            }
        }
    }
}
