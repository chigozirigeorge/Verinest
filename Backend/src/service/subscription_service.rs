use std::sync::Arc;
use uuid::Uuid;
use chrono::{Utc, Duration};

use crate::{
    db::{
        userdb::UserExt,
        naira_walletdb::NairaWalletExt,
        subscriptiondb::SubscriptionExt,
    },
    models::{
        subscriptionmodels::{UserSubscription, SubscriptionTier},
        walletmodels::TransactionStatus,
    },
    error::HttpError, 
    AppState
};

#[derive(Debug, serde::Serialize)]
pub struct RoleChangeStats {
    pub current_count: i32,
    pub monthly_limit: i32,
    pub remaining_changes: i32,
    pub reset_at: chrono::DateTime<Utc>,
    pub has_premium: bool,
}

pub struct SubscriptionService;

impl SubscriptionService {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn check_role_change_limit(
        app_state: Arc<AppState>,
        user_id: Uuid,
    ) -> Result<(), HttpError> {
        let stats = Self::get_role_change_stats(app_state.clone(), user_id).await?;
            
        if stats.current_count >= stats.monthly_limit {
            return Err(HttpError::bad_request(
                format!("Monthly role change limit reached ({}/{}). Upgrade to premium for unlimited changes.", 
                    stats.current_count, stats.monthly_limit)
            ));
        }
        
        Ok(())
    }
    
    pub async fn get_role_change_stats(
        app_state: Arc<AppState>,
        user_id: Uuid,
    ) -> Result<RoleChangeStats, HttpError> {
        let user = app_state.db_client
            .get_user(Some(user_id), None, None, None)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::not_found("User not found"))?;
            
        let monthly_limit = user.clone().get_monthly_role_changes();
        let current_count = user.role_change_count.unwrap_or(0);
        let reset_at = user.role_change_reset_at.unwrap_or_else(|| Utc::now() + Duration::days(30));
        let has_premium = user.has_premium_subscription();
        
        Ok(RoleChangeStats {
            current_count,
            monthly_limit,
            remaining_changes: monthly_limit.saturating_sub(current_count),
            reset_at,
            has_premium,
        })
    }
    
    pub async fn create_premium_subscription(
        app_state: Arc<AppState>,
        user_id: Uuid,
        payment_reference: String,
    ) -> Result<UserSubscription, HttpError> {
        // Verify payment using wallet system
        let payment_verified = Self::verify_payment_with_wallet(
            app_state.clone(),
            user_id,
            &payment_reference,
            9000.0, // 9k Naira
        ).await?;
        
        if !payment_verified {
            return Err(HttpError::bad_request("Payment verification failed"));
        }
        
        // Create subscription record
        let subscription = app_state.db_client
            .create_user_subscription(user_id, SubscriptionTier::Premium, 12)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
            
        // Update user subscription tier
        let _user = app_state.db_client
            .update_user_subscription_tier(user_id, SubscriptionTier::Premium)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
            
        Ok(subscription)
    }
    
    async fn verify_payment_with_wallet(
        app_state: Arc<AppState>,
        user_id: Uuid,
        payment_reference: &str,
        amount_required: f64,
    ) -> Result<bool, HttpError> {
        let amount_kobo = (amount_required * 100.0) as i64;
        
        // Check if transaction exists and is valid
        let transaction = app_state.db_client
            .get_transaction_by_reference(payment_reference)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::bad_request("Payment reference not found"))?;
        
        // Verify transaction belongs to user and is sufficient
        if transaction.user_id != user_id {
            return Err(HttpError::unauthorized("Payment reference does not belong to you"));
        }
        
        if transaction.amount != amount_kobo {
            return Err(HttpError::bad_request("Payment amount mismatch"));
        }
        
        if transaction.status != Some(TransactionStatus::Completed) {
            return Err(HttpError::bad_request("Payment not completed"));
        }
        
        Ok(true)
    }
}