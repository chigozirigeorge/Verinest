// db/cache.rs
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::Mutex;
use redis::aio::MultiplexedConnection;
use uuid::Uuid;
use serde::{Serialize, de::DeserializeOwned};

/// Cache TTL constants (in seconds)
pub const CHAT_CACHE_TTL: usize = 3600;        // 1 hour
pub const MESSAGE_CACHE_TTL: usize = 1800;     // 30 minutes  
pub const UNREAD_CACHE_TTL: usize = 300;       // 5 minutes
pub const USER_CACHE_TTL: usize = 1800;        // 30 minutes
pub const JOB_CACHE_TTL: usize = 900;          // 15 minutes
pub const WORKER_PROFILE_TTL: usize = 1800;    // 30 minutes

pub struct CacheHelper;

impl CacheHelper {
    /// Generic get from cache
    pub async fn get<T: DeserializeOwned>(
        redis: &Arc<Mutex<MultiplexedConnection>>,
        key: &str,
    ) -> Result<Option<T>, redis::RedisError> {
        let mut conn = redis.lock().await;
        let cached: Result<String, redis::RedisError> = conn.get(key).await;
        
        match cached {
            Ok(data) => {
                if let Ok(value) = serde_json::from_str::<T>(&data) {
                    tracing::debug!("Cache HIT: {}", key);
                    Ok(Some(value))
                } else {
                    tracing::warn!("Cache deserialization failed for: {}", key);
                    Ok(None)
                }
            }
            Err(_) => {
                tracing::debug!("Cache MISS: {}", key);
                Ok(None)
            }
        }
    }
    
    /// Generic set to cache with TTL
    pub async fn set<T: Serialize>(
        redis: &Arc<Mutex<MultiplexedConnection>>,
        key: &str,
        value: &T,
        ttl_seconds: usize,
    ) -> Result<(), redis::RedisError> {
        if let Ok(json) = serde_json::to_string(value) {
            let mut conn = redis.lock().await;
            let _: () = conn.set_ex(key, json, ttl_seconds).await?;
            tracing::debug!("Cache SET: {} (TTL: {}s)", key, ttl_seconds);
        }
        Ok(())
    }
    
    /// Delete a cache key
    pub async fn delete(
        redis: &Arc<Mutex<MultiplexedConnection>>,
        key: &str,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.lock().await;
        let _: () = conn.del(key).await?;
        tracing::debug!("Cache DELETE: {}", key);
        Ok(())
    }
    
    /// Delete multiple keys matching a pattern
    pub async fn delete_pattern(
        redis: &Arc<Mutex<MultiplexedConnection>>,
        pattern: &str,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.lock().await;
        let keys: Vec<String> = conn.keys(pattern).await?;
        if !keys.is_empty() {
            let _: () = conn.del(&keys).await?;
            tracing::debug!("Cache DELETE pattern: {} ({} keys)", pattern, keys.len());
        }
        Ok(())
    }

    /// Invalidate all chat-related caches for a specific chat
    pub async fn invalidate_chat_caches(
        redis: &Arc<Mutex<MultiplexedConnection>>,
        chat_id: Uuid,
        participant_one_id: Uuid,
        participant_two_id: Uuid,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.lock().await;
        
        // Invalidate chat cache
        let chat_key = format!("chat:{}", chat_id);
        let _: () = conn.del(&chat_key).await?;
        
        // Invalidate messages cache for this chat
        let keys: Vec<String> = conn.keys(format!("messages:{}:*", chat_id)).await?;
        if !keys.is_empty() {
            let _: () = conn.del(&keys).await?;
        }
        
        // Invalidate user chats cache for both participants
        let user1_keys: Vec<String> = conn.keys(format!("user_chats:{}:*", participant_one_id)).await?;
        if !user1_keys.is_empty() {
            let _: () = conn.del(&user1_keys).await?;
        }
        
        let user2_keys: Vec<String> = conn.keys(format!("user_chats:{}:*", participant_two_id)).await?;
        if !user2_keys.is_empty() {
            let _: () = conn.del(&user2_keys).await?;
        }
        
        tracing::debug!("Invalidated all caches for chat: {}", chat_id);
        Ok(())
    }
    
    /// Invalidate unread count cache for a user
    pub async fn invalidate_unread_count(
        redis: &Arc<Mutex<MultiplexedConnection>>,
        user_id: Uuid,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.lock().await;
        let unread_key = format!("unread_count:{}", user_id);
        let _: () = conn.del(&unread_key).await?;
        tracing::debug!("Invalidated unread count for user: {}", user_id);
        Ok(())
    }
    
    /// Invalidate user chats cache
    pub async fn invalidate_user_chats(
        redis: &Arc<Mutex<MultiplexedConnection>>,
        user_id: Uuid,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.lock().await;
        let keys: Vec<String> = conn.keys(format!("user_chats:{}:*", user_id)).await?;
        if !keys.is_empty() {
            let _: () = conn.del(&keys).await?;
            tracing::debug!("Invalidated {} user_chats keys for user: {}", keys.len(), user_id);
        }
        Ok(())
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(
        redis: &Arc<Mutex<MultiplexedConnection>>,
    ) -> Result<CacheStats, redis::RedisError> {
        let mut conn = redis.lock().await;
        
        let info: String = redis::cmd("INFO")
            .arg("stats")
            .query_async(&mut *conn)
            .await?;
        
        let mut hits = 0u64;
        let mut misses = 0u64;
        
        for line in info.lines() {
            if line.starts_with("keyspace_hits:") {
                hits = line.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
            } else if line.starts_with("keyspace_misses:") {
                misses = line.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
            }
        }
        
        Ok(CacheStats { hits, misses })
    }
    
    /// Clear all chat-related caches
    pub async fn clear_all_chat_caches(
        redis: &Arc<Mutex<MultiplexedConnection>>,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.lock().await;
        
        let patterns = vec!["chat:*", "messages:*", "user_chats:*", "unread_count:*", "contract_proposal:*"];
        
        for pattern in patterns {
            let keys: Vec<String> = conn.keys(pattern).await?;
            if !keys.is_empty() {
                let _: () = conn.del(&keys).await?;
                tracing::info!("Cleared {} keys for pattern: {}", keys.len(), pattern);
            }
        }
        
        Ok(())
    }

    /// Clear all caches (use with extreme caution)
    pub async fn clear_all_caches(
        redis: &Arc<Mutex<MultiplexedConnection>>,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.lock().await;
        redis::cmd("FLUSHDB").query_async(&mut *conn).await?;
        tracing::warn!("CLEARED ALL CACHES - This should only happen in development/testing!");
        Ok(())
    }

    /// Check Redis health
    pub async fn health_check(
        redis: &Arc<Mutex<MultiplexedConnection>>,
    ) -> Result<bool, redis::RedisError> {
        let mut conn = redis.lock().await;
        let response: String = redis::cmd("PING").query_async(&mut *conn).await?;
        Ok(response == "PONG")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
    
    pub fn total_requests(&self) -> u64 {
        self.hits + self.misses
    }
}