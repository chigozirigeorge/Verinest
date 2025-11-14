use std::sync::Arc;
use uuid::Uuid;
use sqlx::Error as SqlxError;
use crate::db::db::DBClient;
use crate::services::reco_db::RecoDB;
use crate::recommendation_models::Interaction;
use redis::AsyncCommands;
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
                let mut conn = rc.lock().await;
                // Use the correct BRPOP signature that returns Option<(String, String)>
                let result: Result<(String, String), redis::RedisError> = 
                    conn.brpop(&self.queue_key, 5).await;
                
                match result {
                    Ok((_key, payload)) => {
                        match from_str::<Interaction>(&payload) {
                            Ok(interaction) => {
                                if let Err(e) = self.reco_db.record_interaction(&interaction).await {
                                    tracing::error!("BehaviorTracker: failed to record interaction: {}", e.to_string());
                                    // On DB error, consider pushing the payload to a dead-letter queue in Redis
                                    let _: Result<(), _> = conn.lpush("reco:dead_letter", &payload).await;
                                }
                            }
                            Err(e) => {
                                tracing::error!("BehaviorTracker: invalid event payload: {}", e.to_string());
                                let _: Result<(), _> = conn.lpush("reco:bad_payloads", &payload).await;
                            }
                        }
                    }
                    Err(e) => {
                        // Check if it's a timeout error (normal case)
                        if e.kind() == redis::ErrorKind::ResponseError && 
                           e.to_string().contains("BRPOP timeout") {
                            // timeout, no data - this is normal, continue
                        } else {
                            tracing::error!("BehaviorTracker: redis brpop error: {}", e.to_string());
                            // backoff a bit to avoid tight error loop
                            sleep(self.idle_sleep).await;
                        }
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
