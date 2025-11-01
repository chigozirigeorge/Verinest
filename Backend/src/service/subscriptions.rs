use std::sync::Arc;

use tokio::time::{interval, Duration};

use crate::{AppState, models::vendormodels::{VendorProfile, VendorService}};

pub async fn start_vendor_expiry_checker(app_state: Arc<AppState>) {
    let mut interval = interval(Duration::from_secs(86400)); // Check daily
    
    loop {
        interval.tick().await;
        
        // Check expiring subscriptions (7 days warning)
        let expiring_subs = sqlx::query_as::<_, VendorProfile>(
            r#"
            SELECT * FROM vendor_profiles
            WHERE subscription_tier != 'normal'
            AND subscription_expires_at BETWEEN NOW() AND NOW() + INTERVAL '7 days'
            "#
        )
        .fetch_all(&app_state.db_client.pool)
        .await;
        
        if let Ok(profiles) = expiring_subs {
            for profile in profiles {
                if let Some(expires_at) = profile.subscription_expires_at {
                    let days_until = (expires_at - chrono::Utc::now()).num_days() as i32;
                    let _ = app_state.notification_service
                        .notify_subscription_expiring(profile.user_id, days_until)
                        .await;
                }
            }
        }
        
        // Check expiring services (3 days warning)
        let expiring_services = sqlx::query_as::<_, VendorService>(
            r#"
            SELECT s.* FROM vendor_services s
            JOIN vendor_profiles v ON s.vendor_id = v.id
            WHERE s.status = 'active'
            AND s.expires_at BETWEEN NOW() AND NOW() + INTERVAL '3 days'
            "#
        )
        .fetch_all(&app_state.db_client.pool)
        .await;
        
        if let Ok(services) = expiring_services {
            for service in services {
                if let Some(expires_at) = service.expires_at {
                    let vendor = sqlx::query_as::<_, VendorProfile>(
                        "SELECT * FROM vendor_profiles WHERE id = $1"
                    )
                    .bind(service.vendor_id)
                    .fetch_one(&app_state.db_client.pool)
                    .await;
                    
                    if let Ok(vendor) = vendor {
                        let days_until = (expires_at - chrono::Utc::now()).num_days() as i32;
                        let _ = app_state.notification_service
                            .notify_service_expiring(vendor.user_id, &service.title, days_until)
                            .await;
                    }
                }
            }
        }
        
        // Auto-expire services past expiry date
        let _ = sqlx::query(
            r#"
            UPDATE vendor_services
            SET status = 'expired', updated_at = NOW()
            WHERE status = 'active'
            AND expires_at < NOW()
            "#
        )
        .execute(&app_state.db_client.pool)
        .await;
        
        // Downgrade expired subscriptions to Normal tier
        let _ = sqlx::query(
            r#"
            UPDATE vendor_profiles
            SET subscription_tier = 'normal', updated_at = NOW()
            WHERE subscription_tier != 'normal'
            AND subscription_expires_at < NOW()
            "#
        )
        .execute(&app_state.db_client.pool)
        .await;
    }
}