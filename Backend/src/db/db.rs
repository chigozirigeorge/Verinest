// db/db.rs
use sqlx::{Pool, Postgres};
use redis::aio::{ConnectionManager};
use std::sync::Arc;

#[derive(Clone)]
pub struct DBClient {
    pub pool: Pool<Postgres>,
    pub redis_client: Option<Arc<ConnectionManager>>,
}

impl std::fmt::Debug for DBClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DBClient")
            .field("pool", &"Pool<Postgres>")
            .field("redis_client", &self.redis_client.is_some())
            .finish()
    }
}

impl DBClient {
    /// Create a new DBClient with PostgreSQL pool only 
    pub fn new(pool: Pool<Postgres>) -> Self {
        DBClient { 
            pool,
            redis_client: None,
        }
    }

    /// Create a new DBClient with both PostgreSQL and Redis
    pub async fn with_redis(pool: Pool<Postgres>, redis_url: &str) -> Result<Self, String> {
        match redis::Client::open(redis_url) {
            Ok(client) => {
                match ConnectionManager::new(client).await {
                    Ok(conn) => {
                        tracing::info!("✅ Redis connection established successfully");
                        Ok(DBClient {
                            pool,
                            redis_client: Some(Arc::new(conn)),
                        })
                    }
                    Err(e) => {
                        tracing::warn!("⚠️ Failed to connect to Redis: {}. Continuing without cache.", e);
                        Ok(DBClient {
                            pool,
                            redis_client: None,
                        })
                    }
                }
            }
            Err(e) => {
                tracing::warn!("⚠️ Failed to create Redis client: {}. Continuing without cache.", e);
                Ok(DBClient {
                    pool,
                    redis_client: None,
                })
            }
        }
    }

    /// Check if Redis caching is available
    pub fn is_redis_available(&self) -> bool {
        self.redis_client.is_some()
    }

    /// Get cache status for monitoring
    pub fn cache_status(&self) -> &str {
        if self.redis_client.is_some() {
            "enabled"
        } else {
            "disabled"
        }
    }
}