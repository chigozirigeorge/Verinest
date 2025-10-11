use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct Referral {
    pub id: Uuid,
    pub referrer_id: Uuid,
    pub referee_id: Uuid,
    pub points_awarded: i32,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReferralStats {
    pub total_referrals: i64,
    pub total_points_earned: i64,
    pub successful_referrals: Vec<ReferralUser>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReferralUser {
    pub id: Uuid,
    pub name: String,
    pub username: String,
    pub email: String,
    pub joined_at: DateTime<Utc>,
}