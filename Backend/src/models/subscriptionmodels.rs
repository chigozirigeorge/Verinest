use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "subscription_tier", rename_all = "snake_case")]
pub enum SubscriptionTier {
    Free,
    Premium,
}

impl SubscriptionTier {
    pub fn to_str(&self) -> &str {
        match self {
            SubscriptionTier::Free => "free",
            SubscriptionTier::Premium => "premium",
        }
    }
    
    pub fn monthly_role_changes(&self) -> i32 {
        match self {
            SubscriptionTier::Free => 5,
            SubscriptionTier::Premium => i32::MAX, // Unlimited
        }
    }
    
    pub fn annual_price(&self) -> f64 {
        match self {
            SubscriptionTier::Free => 0.0,
            SubscriptionTier::Premium => 9000.0, // 9k Naira per year
        }
    }
    
    pub fn benefits(&self) -> Vec<&str> {
        match self {
            SubscriptionTier::Free => vec![
                "5 role changes per month",
                "Basic support",
            ],
            SubscriptionTier::Premium => vec![
                "Unlimited role changes",
                "Priority support",
                "Early access to new features",
                "Advanced analytics",
            ],
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "subscription_status", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Expired,
    Cancelled,
    Pending,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct UserSubscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tier: SubscriptionTier,
    pub status: SubscriptionStatus,
    pub starts_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub auto_renew: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}