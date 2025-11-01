// service/background_jobs.rs - NEW FILE
use std::sync::Arc;
use chrono::Utc;
use tokio::time::{interval, Duration};

use crate::{
    service::vendor_order_service::VendorOrderService,
    AppState,
};

/// Start background job for auto-confirming deliveries after 7 days
pub async fn start_auto_confirmation_job(app_state: Arc<AppState>) {
    let mut interval = interval(Duration::from_secs(3600)); // Run every hour
    
    loop {
        interval.tick().await;
        
        tracing::info!("Running auto-confirmation job at {}", Utc::now());
        
        let order_service = VendorOrderService::new(
            app_state.db_client.clone(),
            app_state.notification_service.clone(),
        );
        
        match order_service.process_auto_confirmations().await {
            Ok(_) => tracing::info!("Auto-confirmation job completed successfully"),
            Err(e) => tracing::error!("Auto-confirmation job failed: {}", e),
        }
    }
}

/// Start background job for expiring services
pub async fn start_service_expiry_job(app_state: Arc<AppState>) {
    let mut interval = interval(Duration::from_secs(21600)); // Run every 6 hours
    
    loop {
        interval.tick().await;
        
        tracing::info!("Running service expiry job at {}", Utc::now());
        
        // Mark expired services as expired
        match sqlx::query(
            r#"
            UPDATE vendor_services 
            SET status = 'expired'
            WHERE status = 'active' 
            AND expires_at IS NOT NULL 
            AND expires_at < NOW()
            "#
        )
        .execute(&app_state.db_client.pool)
        .await
        {
            Ok(result) => tracing::info!(
                "Service expiry job completed: {} services expired", 
                result.rows_affected()
            ),
            Err(e) => tracing::error!("Service expiry job failed: {}", e),
        }
        
        // Notify vendors of services expiring in 3 days
        match sqlx::query_as::<_, (uuid::Uuid, String, uuid::Uuid)>(
            r#"
            SELECT vs.id, vs.title, vp.user_id
            FROM vendor_services vs
            JOIN vendor_profiles vp ON vs.vendor_id = vp.id
            WHERE vs.status = 'active'
            AND vs.expires_at IS NOT NULL
            AND vs.expires_at BETWEEN NOW() AND NOW() + INTERVAL '3 days'
            "#
        )
        .fetch_all(&app_state.db_client.pool)
        .await
        {
            Ok(expiring_services) => {
                for (service_id, title, vendor_user_id) in expiring_services {
                    let _ = app_state.notification_service
                        .notify_service_expiring(vendor_user_id, &title, 3)
                        .await;
                }
            },
            Err(e) => tracing::error!("Failed to fetch expiring services: {}", e),
        }
    }
}