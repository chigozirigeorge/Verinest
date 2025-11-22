// db/cache.rs
use redis::{AsyncCommands, aio::ConnectionManager};
use std::sync::Arc;
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
        redis: &Arc<ConnectionManager>,
        key: &str,
    ) -> Result<Option<T>, redis::RedisError> {
        let mut redis = ConnectionManager::clone(&redis);
        let cached: Result<String, redis::RedisError> = redis.get(key).await;
        
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
        redis: &Arc<ConnectionManager>,
        key: &str,
        value: &T,
        ttl_seconds: usize,
    ) -> Result<(), redis::RedisError> {
        if let Ok(json) = serde_json::to_string(value) {
            let mut conn = ConnectionManager::clone(redis);
            let _: () = conn.set_ex(key, json, ttl_seconds).await?;
            tracing::debug!("Cache SET: {} (TTL: {}s)", key, ttl_seconds);
        }
        Ok(())
    }
    
    /// Delete a cache key
    pub async fn delete(
        redis: &Arc<ConnectionManager>,
        key: &str,
    ) -> Result<(), redis::RedisError> {
        let mut conn = ConnectionManager::clone(redis);
        let _: () = redis::AsyncCommands::del(&mut conn, key).await?;
        tracing::debug!("Cache DELETE: {}", key);
        Ok(())
    }
    
    // Delete multiple keys matching a pattern using SCAN (non-blocking)
    pub async fn delete_pattern(
        redis: &Arc<ConnectionManager>,
        pattern: &str,
    ) -> Result<(), redis::RedisError> {
        let mut conn = ConnectionManager::clone(redis);
        let mut cursor: u64 = 0;
        let mut deleted_count = 0;
        
        loop {
            // Use SCAN instead of KEYS to avoid blocking
            // SCAN returns (cursor, keys) tuple
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100)  // Process 100 keys at a time
                .query_async(&mut conn)
                .await?;
            
            // Delete the batch of keys
            if !keys.is_empty() {
                deleted_count += keys.len();
                let _: () = redis::AsyncCommands::del(&mut conn, &keys).await?;
            }
            
            cursor = new_cursor;
            if cursor == 0 {
                break;  // Scan complete
            }
        }
        
        tracing::debug!("Cache DELETE pattern: {} ({} keys deleted using SCAN)", pattern, deleted_count);
        Ok(())
    }

    /// Invalidate all chat-related caches for a specific chat
    pub async fn invalidate_chat_caches(
        redis: &Arc<ConnectionManager>,
        chat_id: Uuid,
        participant_one_id: Uuid,
        participant_two_id: Uuid,
    ) -> Result<(), redis::RedisError> {
        let mut conn = ConnectionManager::clone(redis);
        
        // Invalidate chat cache
        let chat_key = format!("chat:{}", chat_id);
        let _: () = redis::AsyncCommands::del(&mut conn, &chat_key).await?;
        
        // ✅ FIX: Use SCAN instead of KEYS for non-blocking invalidation
        // Invalidate messages cache for this chat
        Self::scan_and_delete(&conn, &format!("messages:{}:*", chat_id), "messages").await?;
        
        // Invalidate user chats cache for both participants
        Self::scan_and_delete(&conn, &format!("user_chats:{}:*", participant_one_id), "user_chats_1").await?;
        Self::scan_and_delete(&conn, &format!("user_chats:{}:*", participant_two_id), "user_chats_2").await?;
        
        tracing::debug!("Invalidated all caches for chat: {}", chat_id);
        Ok(())
    }
    
    /// Helper: Scan and delete keys matching a pattern without blocking Redis
    async fn scan_and_delete(
        conn: &ConnectionManager,
        pattern: &str,
        pattern_name: &str,
    ) -> Result<(), redis::RedisError> {
        let mut cursor: u64 = 0;
        let mut deleted_count = 0;
        let mut conn = ConnectionManager::clone(conn);
        
        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100)  // Process 100 keys per iteration
                .query_async(&mut conn)
                .await?;
            
            if !keys.is_empty() {
                deleted_count += keys.len();
                let _: () = redis::AsyncCommands::del(&mut conn, &keys).await?;
            }
            
            cursor = new_cursor;
            if cursor == 0 {
                break;  // SCAN complete
            }
        }
        
        if deleted_count > 0 {
            tracing::debug!("Cache INVALIDATE {}: {} pattern ({} keys deleted)", pattern_name, pattern, deleted_count);
        }
        Ok(())
    }
    
    /// Invalidate unread count cache for a user
    pub async fn invalidate_unread_count(
        redis: &Arc<ConnectionManager>,
        user_id: Uuid,
    ) -> Result<(), redis::RedisError> {
        let mut conn = ConnectionManager::clone(redis);
        let unread_key = format!("unread_count:{}", user_id);
        let _: () = redis::AsyncCommands::del(&mut conn, &unread_key).await?;
        tracing::debug!("Invalidated unread count for user: {}", user_id);
        Ok(())
    }
    
    /// Invalidate user chats cache
    pub async fn invalidate_user_chats(
        redis: &Arc<ConnectionManager>,
        user_id: Uuid,
    ) -> Result<(), redis::RedisError> {
        let conn = ConnectionManager::clone(redis);
        // ✅ FIX: Use SCAN instead of KEYS for non-blocking invalidation
        Self::scan_and_delete(&conn, &format!("user_chats:{}:*", user_id), "user_chats").await?;
        tracing::debug!("Invalidated user_chats cache for user: {}", user_id);
        Ok(())
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(
        redis: &Arc<ConnectionManager>,
    ) -> Result<CacheStats, redis::RedisError> {
        let mut conn = ConnectionManager::clone(redis);
        
        let info: String = redis::cmd("INFO")
            .arg("stats")
            .query_async(&mut conn)
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
        redis: &Arc<ConnectionManager>,
    ) -> Result<(), redis::RedisError> {
        let conn = ConnectionManager::clone(redis);
        
        let patterns = vec!["chat:*", "messages:*", "user_chats:*", "unread_count:*", "contract_proposal:*"];
        
        for pattern in patterns {
            Self::scan_and_delete(&conn, pattern, pattern).await?;
        }
        
        tracing::info!("Cleared all chat-related cache patterns using SCAN");
        Ok(())
    }

    /// Clear all caches (use with extreme caution)
    pub async fn clear_all_caches(
        redis: &Arc<ConnectionManager>,
    ) -> Result<(), redis::RedisError> {
        let mut conn = ConnectionManager::clone(redis);
        redis::cmd("FLUSHDB").query_async(&mut conn).await?;
        tracing::warn!("CLEARED ALL CACHES - This should only happen in development/testing!");
        Ok(())
    }

    /// Check Redis health
    pub async fn health_check(
        redis: &Arc<ConnectionManager>,
    ) -> Result<bool, redis::RedisError> {
        let mut conn = ConnectionManager::clone(redis);
        let response: String = redis::cmd("PING").query_async(&mut conn).await?;
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