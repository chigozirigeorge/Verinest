use std::sync::Arc;
use uuid::Uuid;
use sqlx::Error as SqlxError;
use crate::db::db::DBClient;
use redis::AsyncCommands;
use chrono::{Utc, Duration};

/// Simple segmentation service used by the recommendation engine.
///
/// This module implements a lightweight, conservative segmentation strategy
/// suitable as a first-pass for personalized feeds. It relies on counting
/// user interactions stored in `recommendation_interactions` and caches
/// computed segments in Redis for quick lookup.

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Segment {
    New,
    Active,
    PowerUser,
    Dormant,
}

#[derive(Clone)]
pub struct SegmentationService {
    db_client: Arc<DBClient>,
    /// cache ttl in seconds
    cache_ttl: usize,
}

impl SegmentationService {
    pub fn new(db_client: Arc<DBClient>, cache_ttl_seconds: usize) -> Self {
        Self { db_client, cache_ttl: cache_ttl_seconds }
    }

    /// Compute a simple segment for a single user.
    /// Heuristics (configurable later):
    /// - PowerUser: >= 50 interactions in last 30 days
    /// - Active: >= 5 interactions in last 30 days
    /// - New: total interactions < 5 and account created within 7 days
    /// - Dormant: otherwise
    pub async fn compute_user_segment(&self, user_id: Uuid) -> Result<Segment, SqlxError> {
        // Count interactions in the last 30 days
        let recent_count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)::bigint FROM recommendation_interactions WHERE user_id = $1 AND created_at > (NOW() - INTERVAL '30 days')"#
        )
        .bind(user_id)
        .fetch_one(&self.db_client.pool)
        .await?;

        // Total interactions
        let total_count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)::bigint FROM recommendation_interactions WHERE user_id = $1"#
        )
        .bind(user_id)
        .fetch_one(&self.db_client.pool)
        .await?;

        // Account age (if users table exists)
        let created_at_opt: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
            r#"SELECT created_at FROM users WHERE id = $1 LIMIT 1"#
        )
        .bind(user_id)
        .fetch_optional(&self.db_client.pool)
        .await?;

        let segment = if recent_count.0 >= 50 {
            Segment::PowerUser
        } else if recent_count.0 >= 5 {
            Segment::Active
        } else if total_count.0 < 5 {
            if let Some(created) = created_at_opt {
                let age = Utc::now() - created;
                if age < Duration::days(7) {
                    Segment::New
                } else {
                    Segment::Dormant
                }
            } else {
                // If we can't find the user creation time, treat by counts only
                Segment::New
            }
        } else {
            Segment::Dormant
        };

        // Cache the computed segment for quick lookup
        let _ = self.cache_segment(user_id, &segment).await;

        Ok(segment)
    }

    /// Compute segments for a batch of users. Returns a map of user_id -> Segment.
    /// This will use compute_user_segment per-user; can be optimized later to a single query.
    pub async fn compute_batch_segments(&self, user_ids: &[Uuid]) -> Result<Vec<(Uuid, Segment)>, SqlxError> {
        let mut out = Vec::with_capacity(user_ids.len());
        for uid in user_ids {
            let seg = self.compute_user_segment(*uid).await?;
            out.push((*uid, seg));
        }
        Ok(out)
    }

    fn redis_key_for(user_id: Uuid) -> String {
        format!("reco:segment:{}", user_id)
    }

    /// Cache a segment in Redis (no-op if Redis not configured)
    pub async fn cache_segment(&self, user_id: Uuid, segment: &Segment) -> Result<(), redis::RedisError> {
        if let Some(rc) = &self.db_client.redis_client {
            let mut conn = rc.lock().await;
            let key = Self::redis_key_for(user_id);
            let payload = serde_json::to_string(segment).unwrap_or_else(|_| "\"Dormant\"".to_string());
            let _: () = conn.set_ex(key, payload, self.cache_ttl).await?;
        }
        Ok(())
    }

    /// Get cached segment if present
    pub async fn get_cached_segment(&self, user_id: Uuid) -> Result<Option<Segment>, redis::RedisError> {
        if let Some(rc) = &self.db_client.redis_client {
            let mut conn = rc.lock().await;
            let key = Self::redis_key_for(user_id);
            let raw: Option<String> = conn.get(key).await?;
            if let Some(s) = raw {
                if let Ok(seg) = serde_json::from_str::<Segment>(&s) {
                    return Ok(Some(seg));
                }
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::db::DBClient;
    use sqlx::PgPool;

    #[tokio::test]
    async fn segmentation_compiles() {
        let pool = PgPool::connect_lazy("postgres://localhost/verinest").unwrap();
        let db_client = Arc::new(DBClient::new(pool));
        let svc = SegmentationService::new(db_client, 86400);

        // We can't run DB queries here, but ensure the API is callable
        let _ = svc.compute_batch_segments(&[]);
    }
}
