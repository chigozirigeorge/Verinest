use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserWallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub wallet_address: String,
    pub wallet_type: Option<String>, // Make optional if can be NULL
    pub blockchain: Option<String>,  // Make optional if can be NULL
    pub is_verified: Option<bool>,   // Make optional if can be NULL
    pub created_at: Option<DateTime<Utc>>, // Make optional if can be NULL
    pub updated_at: Option<DateTime<Utc>>, // Make optional if can be NULL
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct WalletUpdateRequest {
    pub wallet_address: String,
    pub wallet_type: Option<String>,
    pub blockchain: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WalletVerificationRequest {
    pub signature: String,
    pub message: String,
    pub wallet_address: String,
}