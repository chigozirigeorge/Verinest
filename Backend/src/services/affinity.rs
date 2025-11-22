use std::sync::Arc;
use redis::aio::ConnectionManager;
use uuid::Uuid;
use sqlx::Error as SqlxError;
use sqlx::Row;
use crate::db::db::DBClient;
use crate::services::reco_db::RecoDB;
use redis::AsyncCommands;

/// Simple affinity scoring service.
///
/// This implements a lightweight collaborative filtering approximation:
/// 1. Gather items the user has recently interacted with.
/// 2. Find other items that users who touched those items also touched.
/// 3. Rank by co-occurrence count (or fall back to global popularity).
///
/// Results are cached in Redis under `reco:affinity:{user_id}` to avoid expensive
/// queries on every request.

#[derive(Clone)]
pub struct AffinityService {
    db_client: Arc<DBClient>,
    reco_db: RecoDB,
    cache_ttl: usize,
}

impl AffinityService {
    pub fn new(db_client: Arc<DBClient>, cache_ttl_seconds: usize) -> Self {
        let reco_db = RecoDB::new(db_client.clone());
        Self { db_client, reco_db, cache_ttl: cache_ttl_seconds }
    }

    fn redis_key(user_id: Uuid) -> String {
        format!("reco:affinity:{}", user_id)
    }

    /// Compute top-N item recommendations for a user using collaborative co-occurrence.
    /// Returns a Vec of (item_id, score) where score is a non-negative float (higher = better).
    pub async fn compute_affinity_recommendations(&self, user_id: Uuid, limit: i64) -> Result<Vec<(Uuid, f32)>, SqlxError> {
        // 1. Get the user's distinct recent items (last 365 days)
        let user_items_rows = sqlx::query(
            r#"SELECT DISTINCT item_id FROM recommendation_interactions WHERE user_id = $1 AND created_at > (NOW() - INTERVAL '365 days')"#
        )
        .bind(user_id)
        .fetch_all(&self.db_client.pool)
        .await?;

        let mut user_items: Vec<Uuid> = Vec::new();
        for row in user_items_rows {
            if let Ok(id) = row.try_get::<Uuid, _>("item_id") {
                user_items.push(id);
            }
        }

        // If the user has no history, fall back to global popularity in last 30 days (optimized)
        if user_items.is_empty() {
            // Use a more efficient query with LIMIT early and proper indexing
            let pop_rows = sqlx::query(r#"
                SELECT item_id, interaction_count as cnt 
                FROM (
                    SELECT item_id, COUNT(*) as interaction_count
                    FROM recommendation_interactions 
                    WHERE created_at > (NOW() - INTERVAL '30 days')
                    GROUP BY item_id
                    ORDER BY interaction_count DESC
                    LIMIT $1
                ) popular_items
            "#)
                .bind(limit)
                .fetch_all(&self.db_client.pool)
                .await?;

            let mut out = Vec::new();
            for r in pop_rows {
                let item_id: Uuid = r.try_get("item_id").unwrap_or_else(|_| Uuid::nil());
                let cnt: i64 = r.try_get("cnt").unwrap_or(0);
                if item_id != Uuid::nil() {
                    out.push((item_id, cnt as f32));
                }
            }
            return Ok(out);
        }

        // 2. Find co-occurring items via users who interacted with the user's items (optimized)
        //    This query finds items (not in user's items) and counts distinct users that touched them.
        let rows = sqlx::query(
            r#"
            WITH user_interactions AS (
                SELECT DISTINCT user_id 
                FROM recommendation_interactions 
                WHERE item_id = ANY($1)
                AND created_at > (NOW() - INTERVAL '90 days')
            )
            SELECT ri.item_id, COUNT(DISTINCT ri.user_id) as score
            FROM recommendation_interactions ri
            INNER JOIN user_interactions ui ON ri.user_id = ui.user_id
            WHERE ri.item_id != ALL($1)
              AND ri.created_at > (NOW() - INTERVAL '90 days')
            GROUP BY ri.item_id
            ORDER BY score DESC
            LIMIT $2
            "#
        )
        .bind(&user_items)
        .bind(limit)
        .fetch_all(&self.db_client.pool)
        .await?;

        let mut scored: Vec<(Uuid, f32)> = Vec::new();
        for r in rows {
            let item_id: Uuid = match r.try_get("item_id") {
                Ok(id) => id,
                Err(_) => continue,
            };
            let score_i: i64 = r.try_get("score").unwrap_or(0);
            scored.push((item_id, score_i as f32));
        }

        // Cache result in Redis for quick access
        let _ = self.cache_affinity(user_id, &scored).await;

        Ok(scored)
    }

    /// Get cached affinity recommendations if present
    pub async fn get_cached_affinity(&self, user_id: Uuid) -> Result<Option<Vec<(Uuid, f32)>>, redis::RedisError> {
        if let Some(rc) = &self.db_client.redis_client {
            let mut conn = ConnectionManager::clone(rc);
            let key = Self::redis_key(user_id);
            let cached: Result<Option<String>, redis::RedisError> = redis::cmd("GET")
                .arg(&key)
                .query_async(&mut conn)
                .await;
            if let Ok(Some(raw)) = cached {
                if let Ok(vec) = serde_json::from_str::<Vec<(Uuid, f32)>>(&raw) {
                    return Ok(Some(vec));
                }
            }
        }
        Ok(None)
    }

    /// Cache affinity recommendations in Redis
    pub async fn cache_affinity(&self, user_id: Uuid, items: &Vec<(Uuid, f32)>) -> Result<(), redis::RedisError> {
        if let Some(rc) = &self.db_client.redis_client {
            let mut conn = ConnectionManager::clone(rc);
            let key = Self::redis_key(user_id);
            let payload = serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string());
            let _: Result<(), redis::RedisError> = redis::cmd("SETEX")
                .arg(&key)
                .arg(self.cache_ttl)
                .arg(&payload)
                .query_async(&mut conn)
                .await;
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::db::DBClient;
    use sqlx::PgPool;

    #[tokio::test]
    async fn affinity_compiles() {
        let pool = PgPool::connect_lazy("postgres://localhost/verinest").unwrap();
        let db_client = Arc::new(DBClient::new(pool));
        let svc = AffinityService::new(db_client, 3600);

        let _ = svc.compute_affinity_recommendations(Uuid::nil(), 10);
    }
}
