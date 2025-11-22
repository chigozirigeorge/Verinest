use std::sync::Arc;
use crate::db::db::DBClient;
use crate::services::reco_db::RecoDB;
use crate::recommendation_models::Interaction;
use redis::{AsyncCommands, aio::ConnectionManager};
use serde_json::from_str;
use tokio::time::{sleep, Duration};

/// Behavior tracking worker (phase 1)
///
/// This is a simple, resilient worker that reads lightweight interaction events
/// from a Redis list (`reco:events_list`) using `BRPOP` and persists them into
/// Postgres via the `RecoDB` helper. It is intentionally simple so it can be
/// deployed quickly; later iterations can switch this to Redis Streams with
/// consumer groups for at-least-once processing and better visibility.

#[derive(Clone)]
pub struct BehaviorTracker {
    db_client: Arc<DBClient>,
    reco_db: RecoDB,
    /// Redis list key to pop events from
    pub queue_key: String,
    /// Poll/backoff settings
    pub idle_sleep: Duration,
}

impl BehaviorTracker {
    pub fn new(db_client: Arc<DBClient>, queue_key: &str) -> Self {
        let reco_db = RecoDB::new(db_client.clone());
        Self { db_client, reco_db, queue_key: queue_key.to_string(), idle_sleep: Duration::from_secs(2) }
    }

    /// Run the worker loop until the provided shutdown signal triggers.
    /// This will block the current task while polling Redis with BRPOP.
    pub async fn run_forever(&self, shutdown: impl std::future::Future<Output = ()>) {
        let mut shutdown = Box::pin(shutdown);

        loop {
            // Check shutdown first
            if futures::future::poll_immediate(&mut shutdown).await.is_some() {
                tracing::info!("BehaviorTracker: shutdown requested, exiting loop");
                break;
            }

            // If Redis isn't configured, sleep and retry
            if self.db_client.redis_client.is_none() {
                tracing::warn!("BehaviorTracker: Redis not configured; sleeping before retrying");
                sleep(self.idle_sleep).await;
                continue;
            }

            // Try to pop an event from the list with a small timeout
            if let Some(rc) = &self.db_client.redis_client {
                let mut conn = ConnectionManager::clone(rc);
                // Use explicit BRPOP command and map to Option<(String, String)> so nil (timeout) is handled
                match redis::cmd("BRPOP").arg(&self.queue_key).arg(5).query_async::<_, Option<(String, String)>>(&mut conn).await {
                    Ok(Some((_key, payload))) => {
                        match from_str::<Interaction>(&payload) {
                            Ok(interaction) => {
                                if let Err(e) = self.reco_db.record_interaction(&interaction).await {
                                    // Log the error together with the raw payload to aid debugging
                                    tracing::error!("BehaviorTracker: failed to record interaction: {} ; payload: {}", e.to_string(), payload);
                                    // On DB error, push the payload to a dead-letter list in Redis
                                    let _: Result<(), _> = conn.lpush("reco:dead_letter", &payload).await;
                                } else {
                                    tracing::info!("BehaviorTracker: recorded interaction {} for user {}", interaction.id, interaction.user_id);
                                }
                            }
                            Err(e) => {
                                // Include raw payload when deserialization fails to make debugging easier
                                tracing::error!("BehaviorTracker: invalid event payload: {} ; payload: {}", e.to_string(), payload);
                                let _: Result<(), _> = conn.lpush("reco:bad_payloads", &payload).await;
                            }
                        }
                    }
                    Ok(None) => {
                        // timeout, no data
                    }
                    Err(e) => {
                        tracing::error!("BehaviorTracker: redis brpop error: {}", e.to_string());
                        // backoff a bit to avoid tight error loop
                        sleep(self.idle_sleep).await;
                    }
                }
            }
        }

        tracing::info!("BehaviorTracker: stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::db::DBClient;
    use sqlx::PgPool;

    #[tokio::test]
    async fn behavior_tracker_compiles() {
        let pool = PgPool::connect_lazy("postgres://localhost/verinest").unwrap();
        let db_client = Arc::new(DBClient::new(pool));
        let tracker = BehaviorTracker::new(db_client, "reco:events_list");

        // Ensure the API is callable
        let _ = tracker.idle_sleep;
    }
}
