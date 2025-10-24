use std::{
    collections::HashMap,
    sync::Arc,
};

use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{
    Instrument,
    info_span,
};

use crate::docker::token::{
    CacheKey,
    Token,
};

#[cfg(feature = "redis_cache")]
use redis::AsyncCommands;

#[cfg(feature = "redis_cache")]
const REDIS_PREFIX: &str = "docker-registry-client:token";

#[derive(Debug)]
pub enum FetchError {
    CheckExists(redis::RedisError),
    DeserializeToken(serde_json::Error),
    GetConnection(redis::RedisError),
    GetValue(redis::RedisError),
}

#[derive(Debug)]
pub enum StoreError {
    GetConnection(redis::RedisError),
    SerializeToken(serde_json::Error),
    SetExpiration(redis::RedisError),
    SetValue(redis::RedisError),
}

#[async_trait::async_trait]
pub(super) trait Cache: std::fmt::Debug + Send + Sync + dyn_clone::DynClone {
    async fn fetch(&self, key: &CacheKey) -> Result<Option<Token>, FetchError>;
    async fn store(&self, key: CacheKey, token: Token) -> Result<(), StoreError>;
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

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CheckExists(e) => write!(f, "failed to check if key exists: {e}"),
            Self::DeserializeToken(e) => write!(f, "failed to deserialize token: {e}"),
            Self::GetConnection(e) => write!(f, "failed to get redis connection: {e}"),
            Self::GetValue(e) => write!(f, "failed to get value from redis: {e}"),
        }
    }
}

impl std::error::Error for FetchError {}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GetConnection(e) => write!(f, "failed to get redis connection: {e}"),
            Self::SerializeToken(e) => write!(f, "failed to serialize token: {e}"),
            Self::SetExpiration(e) => write!(f, "failed to set expiration: {e}"),
            Self::SetValue(e) => write!(f, "failed to set value in redis: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

#[async_trait::async_trait]
impl Cache for NoCache {
    async fn fetch(&self, _key: &CacheKey) -> Result<Option<Token>, FetchError> {
        Ok(None)
    }

    async fn store(&self, _key: CacheKey, _token: Token) -> Result<(), StoreError> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl Cache for MemoryTokenCache {
    #[tracing::instrument]
    async fn fetch(&self, key: &CacheKey) -> Result<Option<Token>, FetchError> {
        let result = self.cache.read().await.get(key).cloned().and_then(|token| {
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
        });

        Ok(result)
    }

    #[tracing::instrument]
    async fn store(&self, key: CacheKey, token: Token) -> Result<(), StoreError> {
        self.cache.write().await.insert(key, token);

        Ok(())
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
    async fn fetch(&self, key: &CacheKey) -> Result<Option<Token>, FetchError> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .instrument(info_span!("get redis connection"))
            .await
            .map_err(FetchError::GetConnection)?;

        let key = format!("{REDIS_PREFIX}:{key}");

        let exists: bool = connection
            .exists(&key)
            .instrument(info_span!("check if key exists"))
            .await
            .map_err(FetchError::CheckExists)?;

        if !exists {
            return Ok(None);
        }

        let value: String = connection
            .get(&key)
            .instrument(info_span!("get value"))
            .await
            .map_err(FetchError::GetValue)?;

        let token = serde_json::from_str(&value).map_err(FetchError::DeserializeToken)?;

        Ok(Some(token))
    }

    #[tracing::instrument]
    async fn store(&self, key: CacheKey, token: Token) -> Result<(), StoreError> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .instrument(info_span!("get redis connection"))
            .await
            .map_err(StoreError::GetConnection)?;

        let key = format!("{REDIS_PREFIX}:{key}");

        let value = serde_json::to_string(&token).map_err(StoreError::SerializeToken)?;

        connection
            .set::<&String, String, String>(&key, value)
            .instrument(info_span!("set value"))
            .await
            .map_err(StoreError::SetValue)?;

        if let Some(expires_in) = token.expires_in {
            connection
                .expire::<&String, String>(&key, expires_in)
                .instrument(info_span!("set expire"))
                .await
                .map_err(StoreError::SetExpiration)?;
        }

        Ok(())
    }
}
