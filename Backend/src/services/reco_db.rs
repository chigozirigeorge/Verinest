use std::sync::Arc;
use uuid::Uuid;
use sqlx::Error as SqlxError;
use crate::recommendation_models::{Interaction, Job, WorkerProfile, FeedItem};
use crate::db::db::DBClient;
use redis::aio::ConnectionManager;

/// Recommendation DB helper — thin wrapper around PostgreSQL + Redis used by the reco services
#[derive(Clone)]
pub struct RecoDB {
    db_client: Arc<DBClient>,
}

impl RecoDB {
    pub fn new(db_client: Arc<DBClient>) -> Self {
        Self { db_client }
    }

    // Record an interaction in the DB (postgres). This is the canonical store for behavioral events.
    pub async fn record_interaction(&self, interaction: &Interaction) -> Result<(), SqlxError> {
        // Cast the item_type and action parameters to the Postgres enum types so
        // text inputs (e.g. "job", "view") are accepted. This avoids the
        // "expression is of type text" error when inserting.
        let query = r#"
            INSERT INTO recommendation_interactions (id, user_id, item_id, item_type, action, value, created_at)
            VALUES ($1, $2, $3, $4::feed_item_type, $5::interaction_type, $6, NOW())
        "#;

        sqlx::query(query)
            .bind(interaction.id)
            .bind(interaction.user_id)
            .bind(interaction.item_id)
            // Normalize enum casing to lowercase strings to match postgres enum labels (e.g. "job", "view")
            .bind(format!("{:?}", interaction.item_type).to_lowercase())
            .bind(format!("{:?}", interaction.action).to_lowercase())
            .bind(interaction.value)
            .execute(&self.db_client.pool)
            .await?;

        Ok(())
    }

    /// Fetch recent interactions for a user (most recent first)
    pub async fn get_user_interactions(&self, user_id: Uuid, limit: i64) -> Result<Vec<Interaction>, SqlxError> {
        let rows = sqlx::query_as::<_, Interaction>(
            r#"SELECT id, user_id, item_id, item_type::text as "item_type:", action::text as "action:", value, created_at
               FROM recommendation_interactions
               WHERE user_id = $1
               ORDER BY created_at DESC
               LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.db_client.pool)
        .await?;

        Ok(rows)
    }

    /// Batch fetch jobs by ids
    pub async fn get_jobs_by_ids(&self, ids: &[Uuid]) -> Result<Vec<Job>, SqlxError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let rows = sqlx::query_as::<_, Job>(
            r#"SELECT * FROM jobs WHERE id = ANY($1)"#
        )
        .bind(ids)
        .fetch_all(&self.db_client.pool)
        .await?;

        Ok(rows)
    }

    /// Batch fetch worker profiles by ids
    pub async fn get_worker_profiles_by_ids(&self, ids: &[Uuid]) -> Result<Vec<WorkerProfile>, SqlxError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let rows = sqlx::query_as::<_, WorkerProfile>(
            r#"SELECT * FROM worker_profiles WHERE id = ANY($1)"#
        )
        .bind(ids)
        .fetch_all(&self.db_client.pool)
        .await?;

        Ok(rows)
    }

    /// Cache a computed feed for a user in Redis as a JSON string with TTL (seconds)
    /// Stores under key: reco:feed:{user_id}:{role}
    pub async fn cache_feed(&self, user_id: Uuid, role: &str, items: &[FeedItem], ttl_seconds: usize) -> Result<(), redis::RedisError> {
        if let Some(redis_client) = &self.db_client.redis_client {
            let key = format!("reco:feed:{}:{}", user_id, role);
            let mut conn = ConnectionManager::clone(redis_client);
            let json_items = serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string());
            let _: Result<(), redis::RedisError> = redis::cmd("SETEX")
                .arg(&key)
                .arg(ttl_seconds)
                .arg(&json_items)
                .query_async(&mut conn)
                .await;
            Ok(())
        } else {
            // Redis not available — treat as a no-op
            Ok(())
        }
    }

    /// Retrieve cached feed for user if present
    pub async fn get_cached_feed(&self, user_id: Uuid, role: &str) -> Result<Option<Vec<FeedItem>>, redis::RedisError> {
        if let Some(redis_client) = &self.db_client.redis_client {
            let key = format!("reco:feed:{}:{}", user_id, role);
            let mut conn = ConnectionManager::clone(redis_client);
            let raw: Result<Option<String>, redis::RedisError> = redis::cmd("GET")
                .arg(&key)
                .query_async(&mut conn)
                .await;
            if let Ok(Some(json_str)) = raw {
                if let Ok(parsed) = serde_json::from_str::<Vec<FeedItem>>(&json_str) {
                    return Ok(Some(parsed));
                }
            }
            Ok(None)
        } else {
            Ok(None)
        }
    }

    // Push a light-weight event into a Redis stream for real-time processing
    // Stream: reco:events
    pub async fn push_event_stream(&self, interaction: &Interaction) -> Result<(), redis::RedisError> {
        if let Some(redis_client) = &self.db_client.redis_client {
            let mut conn = ConnectionManager::clone(redis_client);
            let key = "reco:events";
            // Serialize the Interaction via serde so enum casing and shapes match the
            // worker's deserializer (lowercase enums, RFC3339 timestamps, etc.)
            let payload_str = serde_json::to_string(&interaction).map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "serialization error")))?;

            // Push into Redis Stream for stream-based consumers
            let _: Result<String, redis::RedisError> = redis::cmd("XADD")
                .arg(key)
                .arg("*")
                .arg("data")
                .arg(payload_str.clone())
                .query_async(&mut conn)
                .await;

            // Also push into a simple list for the BRPOP-based worker (backwards compatible)
            let list_key = "reco:events_list";
            let _: Result<(), redis::RedisError> = redis::cmd("RPUSH")
                .arg(list_key)
                .arg(&payload_str)
                .query_async(&mut conn)
                .await;
            Ok(())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recommendation_models::{FeedItemType, Interaction, InteractionType};
    use uuid::Uuid;

    // Unit-level tests only verify that methods compile and signatures work.
    // Integration tests should run against real DB/Redis and are out-of-scope here.
    #[tokio::test]
    async fn reco_db_compiles() {
        // Create a dummy DBClient with a real pool is not possible in unit tests here.
        // We'll just ensure the wrapper struct can be constructed.
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/verinest").unwrap();
        let db_client = Arc::new(DBClient::new(pool));
        let reco = RecoDB::new(db_client.clone());

        let inter = Interaction::new(
            Uuid::new_v4(), 
            Uuid::new_v4(), 
            FeedItemType::Job, InteractionType::View, Some(1.0));
        // record_interaction can't be executed (no DB), but the method exists and can be called in async context
        let _ = reco.record_interaction(&inter);
    }
}
