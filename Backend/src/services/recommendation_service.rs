use std::sync::Arc;
use uuid::Uuid;
use sqlx::Error as SqlxError;
use crate::db::db::DBClient;
use crate::recommendation_models::{FeedItem, FeedResponse, RankedItemMeta, FeedItemType, UserRole};
use crate::services::reco_db::RecoDB;
use crate::services::segmentation::SegmentationService;
use crate::services::ranking::RankingService;

/// RecommendationService composes segmentation, ranking, and caching to produce a feed.
#[derive(Clone)]
pub struct RecommendationService {
    db_client: Arc<DBClient>,
    reco_db: RecoDB,
    segmentation: SegmentationService,
    ranking: RankingService,
    cache_ttl: usize,
}

impl RecommendationService {
    pub fn new(db_client: Arc<DBClient>, cache_ttl_seconds: usize) -> Self {
        let reco_db = RecoDB::new(db_client.clone());
        let segmentation = SegmentationService::new(db_client.clone(), 86400);
        let ranking = RankingService::new(db_client.clone(), cache_ttl_seconds);
        Self { db_client, reco_db, segmentation, ranking, cache_ttl: cache_ttl_seconds }
    }

    /// Generate a feed for user_id and role. Returns a FeedResponse with ranked items (id+score)
    /// and also caches a serialized FeedItem vector in Redis for faster subsequent reads.
    pub async fn generate_feed(&self, user_id: Uuid, role: UserRole, limit: i64) -> Result<FeedResponse, SqlxError> {
        // 1. Try cached feed (RecoDB cache stores Vec<FeedItem>)
        if let Ok(Some(cached)) = self.reco_db.get_cached_feed(user_id, &format!("{:?}", role)).await {
            // Convert FeedItem -> RankedItemMeta for Response
            let items = cached.into_iter().map(|fi| RankedItemMeta { id: fi.id, score: fi.score, reason: None }).collect();
            let resp = FeedResponse { user_id, role, items, generated_at: chrono::Utc::now() };
            return Ok(resp);
        }

        // 2. Compute user segment (may warm segment cache)
        let _segment = match self.segmentation.get_cached_segment(user_id).await {
            Ok(Some(s)) => s,
            _ => { let s = self.segmentation.compute_user_segment(user_id).await?; s }
        };

        // 3. Compute ranked items
        let ranked = self.ranking.compute_ranked_items(user_id, limit).await?;
        let ids: Vec<Uuid> = ranked.iter().map(|(id, _)| *id).collect();

        // 4. Enrich items: fetch job objects (or profiles depending on type)
        let jobs = self.reco_db.get_jobs_by_ids(&ids).await?;

        // Build FeedItem vector for caching
        let mut feed_items: Vec<FeedItem> = Vec::new();
        for (id, score) in ranked.into_iter() {
            // Find job payload
            if let Some(job) = jobs.iter().find(|j| j.id == id) {
                if let Ok(payload) = serde_json::to_value(job) {
                    let fi = FeedItem {
                        id,
                        item_type: FeedItemType::Job,
                        payload,
                        score: score as f64,
                        created_at: chrono::Utc::now(),
                    };
                    feed_items.push(fi);
                }
            }
        }

        // Cache feed in Redis (best-effort)
        let _ = self.reco_db.cache_feed(user_id, &format!("{:?}", role), &feed_items, self.cache_ttl).await;

        // Build FeedResponse
        let items = feed_items.into_iter().map(|fi| RankedItemMeta { id: fi.id, score: fi.score, reason: None }).collect();
        let resp = FeedResponse { user_id, role, items, generated_at: chrono::Utc::now() };
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::db::DBClient;
    use sqlx::PgPool;

    #[tokio::test]
    async fn recommendation_service_compiles() {
        let pool = PgPool::connect_lazy("postgres://localhost/verinest").unwrap();
        let db_client = Arc::new(DBClient::new(pool));
        let svc = RecommendationService::new(db_client, 3600);

        let _ = svc;
    }
}
