use async_trait::async_trait;
use uuid::Uuid;

use crate::models::subscriptionmodels::{UserSubscription, SubscriptionTier};

#[async_trait]
pub trait SubscriptionExt {
    async fn create_user_subscription(
        &self,
        user_id: Uuid,
        tier: SubscriptionTier,
        duration_months: i32,
    ) -> Result<UserSubscription, sqlx::Error>;
    
    async fn get_user_subscription(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserSubscription>, sqlx::Error>;
    
    async fn update_user_subscription_tier(
        &self,
        user_id: Uuid,
        tier: SubscriptionTier,
    ) -> Result<crate::models::usermodel::User, sqlx::Error>;
}

#[async_trait]
impl SubscriptionExt for super::db::DBClient {
    async fn create_user_subscription(
        &self,
        user_id: Uuid,
        tier: SubscriptionTier,
        duration_months: i32,
    ) -> Result<UserSubscription, sqlx::Error> {
        let starts_at = chrono::Utc::now();
        let expires_at = starts_at + chrono::Duration::days(30 * duration_months as i64);
        
        sqlx::query_as::<_, UserSubscription>(
            r#"
            INSERT INTO user_subscriptions 
            (user_id, tier, status, starts_at, expires_at, auto_renew)
            VALUES ($1, $2, 'active', $3, $4, true)
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(tier)
        .bind(starts_at)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn get_user_subscription(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserSubscription>, sqlx::Error> {
        sqlx::query_as::<_, UserSubscription>(
            r#"
            SELECT * FROM user_subscriptions 
            WHERE user_id = $1 AND status = 'active' AND expires_at > NOW()
            ORDER BY created_at DESC LIMIT 1
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
    
    async fn update_user_subscription_tier(
        &self,
        user_id: Uuid,
        tier: SubscriptionTier,
    ) -> Result<crate::models::usermodel::User, sqlx::Error> {
        sqlx::query_as::<_, crate::models::usermodel::User>(
            r#"
            UPDATE users
            SET subscription_tier = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#
        )
        .bind(tier)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }
}