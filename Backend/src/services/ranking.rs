use std::sync::Arc;
use uuid::Uuid;
use sqlx::Error as SqlxError;
use sqlx::Row;
use crate::db::db::DBClient;
use crate::services::reco_db::RecoDB;
use crate::services::affinity::AffinityService;
use redis::AsyncCommands;

/// Ranking service: blends affinity scores with popularity to produce a final ranked list.
///
/// Score composition (configurable):
/// score = w_affinity * norm_affinity + w_pop * norm_popularity
///
/// Results are cached in Redis under `reco:ranked:{user_id}` for `cache_ttl` seconds.

#[derive(Clone)]
pub struct RankingService {
    db_client: Arc<DBClient>,
    reco_db: RecoDB,
    affinity: AffinityService,
    cache_ttl: usize,
    w_affinity: f32,
    w_pop: f32,
}

impl RankingService {
    pub fn new(db_client: Arc<DBClient>, cache_ttl_seconds: usize) -> Self {
        let reco_db = RecoDB::new(db_client.clone());
        let affinity = AffinityService::new(db_client.clone(), cache_ttl_seconds);
        Self {
            db_client,
            reco_db,
            affinity,
            cache_ttl: cache_ttl_seconds,
            w_affinity: 0.7,
            w_pop: 0.3,
        }
    }

    fn redis_key(user_id: Uuid) -> String {
        format!("reco:ranked:{}", user_id)
    }

    /// Compute ranked item ids with blended scores.
    pub async fn compute_ranked_items(&self, user_id: Uuid, limit: i64) -> Result<Vec<(Uuid, f32)>, SqlxError> {
        // 1. Try to get affinity candidates (cached or compute)
        let affinity_vec = match self.affinity.get_cached_affinity(user_id).await {
            Ok(Some(v)) => v,
            _ => {
                // compute and return
                match self.affinity.compute_affinity_recommendations(user_id, limit).await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("RankingService: affinity compute error: {:?}", e.to_string());
                        Vec::new()
                    }
                }
            }
        };

        // Build candidate item list
        let mut candidates: Vec<Uuid> = affinity_vec.iter().map(|(id, _)| *id).collect();

        // If no candidates, fall back to top-popular items
        if candidates.is_empty() {
            let pop_rows = sqlx::query(r#"SELECT item_id FROM recommendation_interactions WHERE created_at > (NOW() - INTERVAL '30 days') GROUP BY item_id ORDER BY COUNT(*) DESC LIMIT $1"#)
                .bind(limit)
                .fetch_all(&self.db_client.pool)
                .await?;
            for r in pop_rows {
                if let Ok(id) = r.try_get::<Uuid, _>("item_id") {
                    candidates.push(id);
                }
            }
        }

        if candidates.is_empty() {
            return Ok(vec![]);
        }

        // 2. Fetch popularity counts for candidates in last 90 days
        let rows = sqlx::query(r#"SELECT item_id, COUNT(*) as cnt FROM recommendation_interactions WHERE item_id = ANY($1) AND created_at > (NOW() - INTERVAL '90 days') GROUP BY item_id"#)
            .bind(&candidates)
            .fetch_all(&self.db_client.pool)
            .await?;

        use std::collections::HashMap;
        let mut pop_map: HashMap<Uuid, i64> = HashMap::new();
        let mut max_pop: i64 = 0;
        for r in rows {
            if let Ok(id) = r.try_get::<Uuid, _>("item_id") {
                let cnt: i64 = r.try_get("cnt").unwrap_or(0);
                pop_map.insert(id, cnt);
                if cnt > max_pop { max_pop = cnt; }
            }
        }

        // Map affinity scores for candidates
        let mut aff_map: HashMap<Uuid, f32> = HashMap::new();
        let mut max_aff: f32 = 0.0;
        for (id, score) in affinity_vec.iter() {
            aff_map.insert(*id, *score);
            if *score > max_aff { max_aff = *score; }
        }

        // Normalize and combine
        let mut scored: Vec<(Uuid, f32)> = Vec::new();
        for id in candidates.iter() {
            let aff = *aff_map.get(id).unwrap_or(&0.0);
            let pop = *pop_map.get(id).unwrap_or(&0) as f32;

            let norm_aff = if max_aff > 0.0 { aff / max_aff } else { 0.0 };
            let norm_pop = if max_pop > 0 { pop / (max_pop as f32) } else { 0.0 };

            let score = self.w_affinity * norm_aff + self.w_pop * norm_pop;
            scored.push((*id, score));
        }

        // Sort by score desc and take top N
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top = scored.into_iter().take(limit as usize).collect::<Vec<_>>();

        // Cache in Redis
        let _ = self.cache_ranked(user_id, &top).await;

        Ok(top)
    }

    /// Cache ranked results in Redis
    pub async fn cache_ranked(&self, user_id: Uuid, items: &Vec<(Uuid, f32)>) -> Result<(), redis::RedisError> {
        if let Some(rc) = &self.db_client.redis_client {
            let mut conn = rc.lock().await;
            let key = Self::redis_key(user_id);
            let payload = serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string());
            let _: () = conn.set_ex(key, payload, self.cache_ttl).await?;
        }
        Ok(())
    }

    /// Get cached ranked results
    pub async fn get_cached_ranked(&self, user_id: Uuid) -> Result<Option<Vec<(Uuid, f32)>>, redis::RedisError> {
        if let Some(rc) = &self.db_client.redis_client {
            let mut conn = rc.lock().await;
            let key = Self::redis_key(user_id);
            if let Ok(Some(raw)) = conn.get::<_, Option<String>>(key).await {
                if let Ok(vec) = serde_json::from_str::<Vec<(Uuid, f32)>>(&raw) {
                    return Ok(Some(vec));
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
    async fn ranking_compiles() {
        let pool = PgPool::connect_lazy("postgres://localhost/verinest").unwrap();
        let db_client = Arc::new(DBClient::new(pool));
        let svc = RankingService::new(db_client, 3600);

        let _ = svc.compute_ranked_items(Uuid::nil(), 10);
    }
}
