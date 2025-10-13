// service/verification_service.rs
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::db::verificationdb::VerificationExt;

pub struct VerificationService {
    db_client: Arc<DBClient>,
}

impl VerificationService {
    pub fn new(db_client: Arc<DBClient>) -> Self {
        Self { db_client }
    }

    pub async fn start_cleanup_task(&self) {
        let db_client = self.db_client.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // Run every hour
            
            loop {
                interval.tick().await;
                
                match db_client.cleanup_expired_otps().await {
                    Ok(count) => {
                        if count > 0 {
                            info!("Cleaned up {} expired OTPs", count);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to cleanup expired OTPs: {}", e);
                    }
                }
            }
        });
    }
}