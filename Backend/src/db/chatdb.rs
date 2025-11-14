// db/chatdb.rs
use async_trait::async_trait;
use uuid::Uuid;
use sqlx::Error;
use redis::{
    AsyncCommands,
    aio::MultiplexedConnection,
};
use super::db::DBClient;
use crate::models::chatnodels::*;
use std::sync::Arc;
use tokio::sync::Mutex;

pub const CHAT_CACHE_TTL: usize = 300;        // 5 minutes
pub const MESSAGE_CACHE_TTL: usize = 600;     // 10 minutes  
pub const UNREAD_CACHE_TTL: usize = 30;       // 30 seconds 

#[async_trait]
pub trait ChatExt {
    async fn create_or_get_chat(
        &self,
        user_one_id: Uuid,
        user_two_id: Uuid,
        job_id: Option<Uuid>,
    ) -> Result<Chat, Error>;
    
    async fn get_user_chats(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Chat>, Error>;
    
    async fn get_chat_by_id(
        &self,
        chat_id: Uuid,
    ) -> Result<Option<Chat>, Error>;
    
    async fn update_chat_status(
        &self,
        chat_id: Uuid,
        status: ChatStatus,
    ) -> Result<Chat, Error>;
    
    async fn send_message(
        &self,
        chat_id: Uuid,
        sender_id: Uuid,
        message_type: MessageType,
        content: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<Message, Error>;
    
    async fn get_chat_messages(
        &self,
        chat_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, Error>;
    
    async fn mark_messages_as_read(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), Error>;
    
    async fn get_unread_count(
        &self,
        user_id: Uuid,
    ) -> Result<i64, Error>;
    
    async fn create_contract_proposal(
        &self,
        chat_id: Uuid,
        message_id: Uuid,
        job_id: Uuid,
        proposed_by: Uuid,
        worker_id: Uuid,
        employer_id: Uuid,
        agreed_rate: f64,
        agreed_timeline: i32,
        terms: String,
    ) -> Result<ContractProposal, Error>;
    
    async fn respond_to_contract_proposal(
        &self,
        proposal_id: Uuid,
        status: String,
    ) -> Result<ContractProposal, Error>;
    
    async fn get_contract_proposal_by_message(
        &self,
        message_id: Uuid,
    ) -> Result<Option<ContractProposal>, Error>;
}

// Helper: Scan and delete keys matching a pattern without blocking Redis
// Used by invalidate_chat_caches_async to avoid O(N) KEYS command
async fn scan_and_delete_async(
    conn: &mut MultiplexedConnection,
    pattern: &str,
    pattern_name: &str,
) -> Result<(), redis::RedisError> {
    let mut cursor: u64 = 0;
    let mut deleted_count = 0;
    
    loop {
        let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)  // Process 100 keys per iteration
            .query_async(&mut *conn)
            .await?;
        
        if !keys.is_empty() {
            deleted_count += keys.len();
            let _: Result<usize, redis::RedisError> = conn.del(&keys).await;
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

async fn invalidate_chat_caches_async(
    redis_client: &Arc<Mutex<MultiplexedConnection>>,
    chat_id: Uuid,
    participant_one_id: Uuid,
    participant_two_id: Uuid,
    sender_id: Uuid,
) -> Result<(), redis::RedisError> {
    let mut conn = redis_client.lock().await;
    
    // Invalidate chat cache - ignore the return value (number of keys deleted)
    let _: Result<usize, redis::RedisError> = conn.del(&format!("chat:{}", chat_id)).await;
    
    // ✅ FIX: Use SCAN instead of KEYS for non-blocking invalidation
    // Invalidate messages cache for this chat
    let messages_pattern = format!("messages:{}:*", chat_id);
    scan_and_delete_async(&mut conn, &messages_pattern, "messages").await?;
    
    // Invalidate user chats cache for both participants
    let user1_pattern = format!("user_chats:{}:*", participant_one_id);
    let user2_pattern = format!("user_chats:{}:*", participant_two_id);
    
    scan_and_delete_async(&mut conn, &user1_pattern, "user_chats_1").await?;
    scan_and_delete_async(&mut conn, &user2_pattern, "user_chats_2").await?;
    
    // Invalidate unread count for receiver
    let receiver_id = if participant_one_id == sender_id {
        participant_two_id
    } else {
        participant_one_id
    };
    let unread_key = format!("unread_count:{}", receiver_id);
    let _: Result<usize, redis::RedisError> = conn.del(&unread_key).await;
    
    tracing::debug!("🔄 Invalidated caches for chat: {}", chat_id);
    Ok(())
}

#[async_trait]
impl ChatExt for DBClient {
    async fn create_or_get_chat(
        &self,
        user_one_id: Uuid,
        user_two_id: Uuid,
        job_id: Option<Uuid>,
    ) -> Result<Chat, Error> {
        // Try to find existing chat
        let existing = sqlx::query_as::<_, Chat>(
            r#"
            SELECT id, participant_one_id, participant_two_id, job_id, status, 
                   last_message_at, created_at
            FROM chats
            WHERE (participant_one_id = $1 AND participant_two_id = $2)
               OR (participant_one_id = $2 AND participant_two_id = $1)
            AND ($3::uuid IS NULL OR job_id = $3)
            "#
        )
        .bind(user_one_id)
        .bind(user_two_id)
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(chat) = existing {
            // Cache the chat
            if let Some(redis_client) = &self.redis_client {
                let cache_key = format!("chat:{}", chat.id);
                let chat_json = serde_json::to_string(&chat).unwrap_or_default();
                let _: Result<(), redis::RedisError> = redis_client
                    .lock()
                    .await
                    .set_ex(&cache_key, chat_json, CHAT_CACHE_TTL)
                    .await;
            }
            return Ok(chat);
        }
        
        // Create new chat
        let chat = sqlx::query_as::<_, Chat>(
            r#"
            INSERT INTO chats (participant_one_id, participant_two_id, job_id)
            VALUES ($1, $2, $3)
            RETURNING id, participant_one_id, participant_two_id, job_id, status,
                      last_message_at, created_at
            "#
        )
        .bind(user_one_id)
        .bind(user_two_id)
        .bind(job_id)
        .fetch_one(&self.pool)
        .await?;

        // Cache the new chat
        if let Some(redis_client) = &self.redis_client {
            let cache_key = format!("chat:{}", chat.id);
            let chat_json = serde_json::to_string(&chat).unwrap_or_default();
            let _: Result<(), redis::RedisError> = redis_client
                .lock()
                .await
                .set_ex(&cache_key, chat_json, CHAT_CACHE_TTL)
                .await;
                
            // Invalidate user chats cache
            let user_chats_key1 = format!("user_chats:{}:*", user_one_id);
            let user_chats_key2 = format!("user_chats:{}:*", user_two_id);
            let _: Result<(), redis::RedisError> = redis_client
                .lock()
                .await
                .del(&[user_chats_key1, user_chats_key2])
                .await;
        }

        Ok(chat)
    }
    
    async fn get_user_chats(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Chat>, Error> {
        let cache_key = format!("user_chats:{}:{}:{}", user_id, limit, offset);
        
        // Try cache first
        if let Some(redis_client) = &self.redis_client {
            let cached: Result<String, redis::RedisError> = redis_client
                .lock()
                .await
                .get(&cache_key)
                .await;
                
            if let Ok(cached_data) = cached {
                if let Ok(chats) = serde_json::from_str::<Vec<Chat>>(&cached_data) {
                    return Ok(chats);
                }
            }
        }
        
        // Fetch from database
        let chats = sqlx::query_as::<_, Chat>(
            r#"
            SELECT id, participant_one_id, participant_two_id, job_id, status,
                   last_message_at, created_at
            FROM chats
            WHERE (participant_one_id = $1 OR participant_two_id = $1)
              AND status = 'active'::chat_status
            ORDER BY last_message_at DESC NULLS LAST, created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        
        // Cache the result
        if let Some(redis_client) = &self.redis_client {
            if let Ok(chats_json) = serde_json::to_string(&chats) {
                let _: Result<(), redis::RedisError> = redis_client
                    .lock()
                    .await
                    .set_ex(&cache_key, chats_json, CHAT_CACHE_TTL)
                    .await;
            }
        }
        
        Ok(chats)
    }
    
    async fn get_chat_by_id(
        &self,
        chat_id: Uuid,
    ) -> Result<Option<Chat>, Error> {
        let cache_key = format!("chat:{}", chat_id);
        
        // Try cache first
        if let Some(redis_client) = &self.redis_client {
            let cached: Result<String, redis::RedisError> = redis_client
                .lock()
                .await
                .get(&cache_key)
                .await;
                
            if let Ok(cached_data) = cached {
                if let Ok(chat) = serde_json::from_str::<Chat>(&cached_data) {
                    return Ok(Some(chat));
                }
            }
        }
        
        // Fetch from database
        let chat = sqlx::query_as::<_, Chat>(
            r#"
            SELECT id, participant_one_id, participant_two_id, job_id, status,
                   last_message_at, created_at
            FROM chats
            WHERE id = $1
            "#
        )
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await?;
        
        // Cache if found
        if let Some(ref chat_data) = chat {
            if let Some(redis_client) = &self.redis_client {
                if let Ok(chat_json) = serde_json::to_string(chat_data) {
                    let _: Result<(), redis::RedisError> = redis_client
                        .lock()
                        .await
                        .set_ex(&cache_key, chat_json, CHAT_CACHE_TTL)
                        .await;
                }
            }
        }
        
        Ok(chat)
    }
    
    async fn update_chat_status(
        &self,
        chat_id: Uuid,
        status: ChatStatus,
    ) -> Result<Chat, Error> {
        let chat = sqlx::query_as::<_, Chat>(
            r#"
            UPDATE chats
            SET status = $2
            WHERE id = $1
            RETURNING id, participant_one_id, participant_two_id, job_id, status,
                      last_message_at, created_at
            "#
        )
        .bind(chat_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;
        
        // Invalidate cache
        if let Some(redis_client) = &self.redis_client {
            let cache_key = format!("chat:{}", chat_id);
            let _: Result<(), redis::RedisError> = redis_client
                .lock()
                .await
                .del(&cache_key)
                .await;
                
            // Invalidate user chats cache for both participants
            let user_chats_key1 = format!("user_chats:{}:*", chat.participant_one_id);
            let user_chats_key2 = format!("user_chats:{}:*", chat.participant_two_id);
            let _: Result<(), redis::RedisError> = redis_client
                .lock()
                .await
                .del(&[user_chats_key1, user_chats_key2])
                .await;
        }
        
        Ok(chat)
    }
    
    // async fn send_message(
    //     &self,
    //     chat_id: Uuid,
    //     sender_id: Uuid,
    //     message_type: MessageType,
    //     content: String,
    //     metadata: Option<serde_json::Value>,
    // ) -> Result<Message, Error> {
    //     let mut tx = self.pool.begin().await?;
        
    //     // Insert message
    //     let message = sqlx::query_as::<_, Message>(
    //         r#"
    //         INSERT INTO messages (chat_id, sender_id, message_type, content, metadata)
    //         VALUES ($1, $2, $3, $4, $5)
    //         RETURNING id, chat_id, sender_id, message_type, content, metadata,
    //                   is_read, read_at, created_at
    //         "#
    //     )
    //     .bind(chat_id)
    //     .bind(sender_id)
    //     .bind(message_type)
    //     .bind(content)
    //     .bind(metadata)
    //     .fetch_one(&mut *tx)
    //     .await?;
        
    //     // Update chat's last_message_at
    //     sqlx::query(
    //         r#"
    //         UPDATE chats
    //         SET last_message_at = NOW()
    //         WHERE id = $1
    //         "#
    //     )
    //     .bind(chat_id)
    //     .execute(&mut *tx)
    //     .await?;
        
    //     tx.commit().await?;
        
    //     // Invalidate relevant caches
    //     if let Some(redis_client) = &self.redis_client {
    //         let mut conn = redis_client.lock().await;
            
    //         // Invalidate chat cache
    //         let chat_key = format!("chat:{}", chat_id);
    //         let _: Result<(), redis::RedisError> = conn.del(&chat_key).await;
            
    //         // Invalidate messages cache for this chat
    //         let messages_pattern = format!("messages:{}:*", chat_id);
    //         let _: Result<(), redis::RedisError> = conn.del(&messages_pattern).await;
            
    //         // Get chat to invalidate user chats
    //         if let Ok(Some(chat)) = self.get_chat_by_id(chat_id).await {
    //             let user1_pattern = format!("user_chats:{}:*", chat.participant_one_id);
    //             let user2_pattern = format!("user_chats:{}:*", chat.participant_two_id);
    //             let _: Result<(), redis::RedisError> = conn.del(&[user1_pattern, user2_pattern]).await;
                
    //             // Invalidate unread count for receiver
    //             let receiver_id = if chat.participant_one_id == sender_id {
    //                 chat.participant_two_id
    //             } else {
    //                 chat.participant_one_id
    //             };
    //             let unread_key = format!("unread_count:{}", receiver_id);
    //             let _: Result<(), redis::RedisError> = conn.del(&unread_key).await;
    //         }
    //     }
        
    //     Ok(message)
    // }

    async fn send_message(
    &self,
    chat_id: Uuid,
    sender_id: Uuid,
    message_type: MessageType,
    content: String,
    metadata: Option<serde_json::Value>,
) -> Result<Message, Error> {
    tracing::info!("🗃️ DB: Starting send_message for chat: {}, sender: {}", chat_id, sender_id);
    
    let mut tx = self.pool.begin().await
        .map_err(|e| {
            tracing::error!("❌ DB: Failed to begin transaction: {}", e);
            e
        })?;

    // Insert message
    let message = sqlx::query_as::<_, Message>(
        r#"
        INSERT INTO messages (chat_id, sender_id, message_type, content, metadata)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, chat_id, sender_id, message_type, content, metadata,
                  is_read, read_at, created_at
        "#
    )
    .bind(chat_id)
    .bind(sender_id)
    .bind(message_type)
    .bind(content)
    .bind(metadata)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("❌ DB: Failed to insert message: {}", e);
        e
    })?;

    tracing::debug!("✅ DB: Message inserted: {}", message.id);

    // Update chat's last_message_at
    sqlx::query(
        r#"
        UPDATE chats
        SET last_message_at = NOW()
        WHERE id = $1
        "#
    )
    .bind(chat_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("❌ DB: Failed to update chat last_message_at: {}", e);
        e
    })?;

    tracing::debug!("✅ DB: Chat last_message_at updated");

    // Get participant IDs before committing transaction
    let chat = sqlx::query_as::<_, Chat>(
        r#"
        SELECT id, participant_one_id, participant_two_id, job_id, status,
               last_message_at, created_at
        FROM chats
        WHERE id = $1
        "#
    )
    .bind(chat_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await
        .map_err(|e| {
            tracing::error!("❌ DB: Failed to commit transaction: {}", e);
            e
        })?;

    tracing::debug!("✅ DB: Transaction committed");

    // ASYNC CACHE INVALIDATION - Don't wait for this to complete
    let redis_client = self.redis_client.clone();
    let _message_clone = message.clone();
    let chat_clone = chat.clone();
    
    tokio::spawn(async move {
        if let Some(redis_client) = redis_client {
            tracing::debug!("🔄 Starting async cache invalidation");
            
            let start_time = std::time::Instant::now();
            
            if let Err(e) = invalidate_chat_caches_async(
                &redis_client, 
                chat_clone.id, 
                chat_clone.participant_one_id, 
                chat_clone.participant_two_id,
                sender_id
            ).await {
                tracing::warn!("⚠️ Async cache invalidation failed: {}", e);
            }
            
            let duration = start_time.elapsed();
            tracing::debug!("✅ Async cache invalidation completed in {:?}", duration);
        }
    });

    tracing::info!("✅ DB: send_message completed successfully for message: {}", message.id);
    
    Ok(message)
}

// Separate function for async cache invalidation
    
    async fn get_chat_messages(
        &self,
        chat_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, Error> {
        let cache_key = format!("messages:{}:{}:{}", chat_id, limit, offset);
        
        // Try cache first
        if let Some(redis_client) = &self.redis_client {
            let cached: Result<String, redis::RedisError> = redis_client
                .lock()
                .await
                .get(&cache_key)
                .await;
                
            if let Ok(cached_data) = cached {
                if let Ok(messages) = serde_json::from_str::<Vec<Message>>(&cached_data) {
                    return Ok(messages);
                }
            }
        }
        
        // Fetch from database
        let messages = sqlx::query_as::<_, Message>(
            r#"
            SELECT id, chat_id, sender_id, message_type, content, metadata,
                   is_read, read_at, created_at
            FROM messages
            WHERE chat_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(chat_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        
        // Cache the result
        if let Some(redis_client) = &self.redis_client {
            if let Ok(messages_json) = serde_json::to_string(&messages) {
                let _: Result<(), redis::RedisError> = redis_client
                    .lock()
                    .await
                    .set_ex(&cache_key, messages_json, MESSAGE_CACHE_TTL)
                    .await;
            }
        }
        
        Ok(messages)
    }
    
    async fn mark_messages_as_read(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
            UPDATE messages
            SET is_read = true, read_at = NOW()
            WHERE chat_id = $1
              AND sender_id != $2
              AND is_read = false
            "#
        )
        .bind(chat_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        
        // Invalidate relevant caches
        if let Some(redis_client) = &self.redis_client {
            let mut conn = redis_client.lock().await;
            
            // Invalidate messages cache
            let messages_pattern = format!("messages:{}:*", chat_id);
            let _: Result<(), redis::RedisError> = conn.del(&messages_pattern).await;
            
            // Invalidate unread count
            let unread_key = format!("unread_count:{}", user_id);
            let _: Result<(), redis::RedisError> = conn.del(&unread_key).await;
        }
        
        Ok(())
    }
    
    async fn get_unread_count(
        &self,
        user_id: Uuid,
    ) -> Result<i64, Error> {
        let cache_key = format!("unread_count:{}", user_id);
        
        // Try cache first
        if let Some(redis_client) = &self.redis_client {
            let cached: Result<i64, redis::RedisError> = redis_client
                .lock()
                .await
                .get(&cache_key)
                .await;
                
            if let Ok(count) = cached {
                return Ok(count);
            }
        }
        
        // Fetch from database
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM messages m
            INNER JOIN chats c ON m.chat_id = c.id
            WHERE (c.participant_one_id = $1 OR c.participant_two_id = $1)
              AND m.sender_id != $1
              AND m.is_read = false
            "#
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        
        // Cache the result
        if let Some(redis_client) = &self.redis_client {
            let _: Result<(), redis::RedisError> = redis_client
                .lock()
                .await
                .set_ex(&cache_key, result, UNREAD_CACHE_TTL)
                .await;
        }
        
        Ok(result)
    }
    
    async fn create_contract_proposal(
        &self,
        chat_id: Uuid,
        message_id: Uuid,
        job_id: Uuid,
        proposed_by: Uuid,
        worker_id: Uuid,
        employer_id: Uuid,
        agreed_rate: f64,
        agreed_timeline: i32,
        terms: String,
    ) -> Result<ContractProposal, Error> {
        let proposal = sqlx::query_as::<_, ContractProposal>(
            r#"
            INSERT INTO contract_proposals 
            (chat_id, message_id, job_id, proposed_by, worker_id, employer_id, 
             agreed_rate, agreed_timeline, terms)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, chat_id, message_id, job_id, proposed_by, worker_id,
                      employer_id, agreed_rate, agreed_timeline, terms, status,
                      created_at, responded_at
            "#
        )
        .bind(chat_id)
        .bind(message_id)
        .bind(job_id)
        .bind(proposed_by)
        .bind(worker_id)
        .bind(employer_id)
        .bind(agreed_rate)
        .bind(agreed_timeline)
        .bind(terms)
        .fetch_one(&self.pool)
        .await?;
        
        // Cache the proposal
        if let Some(redis_client) = &self.redis_client {
            let cache_key = format!("contract_proposal:message:{}", message_id);
            if let Ok(proposal_json) = serde_json::to_string(&proposal) {
                let _: Result<(), redis::RedisError> = redis_client
                    .lock()
                    .await
                    .set_ex(&cache_key, proposal_json, CHAT_CACHE_TTL)
                    .await;
            }
        }
        
        Ok(proposal)
    }
    
    async fn respond_to_contract_proposal(
        &self,
        proposal_id: Uuid,
        status: String,
    ) -> Result<ContractProposal, Error> {
        let proposal = sqlx::query_as::<_, ContractProposal>(
            r#"
            UPDATE contract_proposals
            SET status = $2, responded_at = NOW()
            WHERE id = $1
            RETURNING id, chat_id, message_id, job_id, proposed_by, worker_id,
                      employer_id, agreed_rate, agreed_timeline, terms, status,
                      created_at, responded_at
            "#
        )
        .bind(proposal_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;
        
        // Invalidate proposal cache
        if let Some(redis_client) = &self.redis_client {
            let cache_key = format!("contract_proposal:message:{}", proposal.message_id);
            let _: Result<(), redis::RedisError> = redis_client
                .lock()
                .await
                .del(&cache_key)
                .await;
        }
        
        Ok(proposal)
    }
    
    async fn get_contract_proposal_by_message(
        &self,
        message_id: Uuid,
    ) -> Result<Option<ContractProposal>, Error> {
        let cache_key = format!("contract_proposal:message:{}", message_id);
        
        // Try cache first
        if let Some(redis_client) = &self.redis_client {
            let cached: Result<String, redis::RedisError> = redis_client
                .lock()
                .await
                .get(&cache_key)
                .await;
                
            if let Ok(cached_data) = cached {
                if let Ok(proposal) = serde_json::from_str::<ContractProposal>(&cached_data) {
                    return Ok(Some(proposal));
                }
            }
        }
        
        // Fetch from database
        let proposal = sqlx::query_as::<_, ContractProposal>(
            r#"
            SELECT id, chat_id, message_id, job_id, proposed_by, worker_id,
                   employer_id, agreed_rate, agreed_timeline, terms, status,
                   created_at, responded_at
            FROM contract_proposals
            WHERE message_id = $1
            "#
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;
        
        // Cache if found
        if let Some(ref proposal_data) = proposal {
            if let Some(redis_client) = &self.redis_client {
                if let Ok(proposal_json) = serde_json::to_string(proposal_data) {
                    let _: Result<(), redis::RedisError> = redis_client
                        .lock()
                        .await
                        .set_ex(&cache_key, proposal_json, CHAT_CACHE_TTL)
                        .await;
                }
            }
        }
        
        Ok(proposal)
    }
}