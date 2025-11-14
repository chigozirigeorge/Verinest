use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::types::BigDecimal;
use uuid::Uuid;
use serde_json::Value as JsonValue;

// Core enums used by the recommendation system and other modules
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Worker,
    Employer,
    Vendor,
    Buyer,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "job_status", rename_all = "lowercase")]
pub enum JobStatus {
    Open,
    InProgress,
    UnderReview,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "feed_item_type", rename_all = "lowercase")]
pub enum FeedItemType {
    Job,
    WorkerProfile,
    Service,
    Product,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "interaction_type", rename_all = "lowercase")]
pub enum InteractionType {
    View,
    Click,
    Apply,
    Save,
    Message,
    Purchase,
    Dismiss,
}

// Primary domain models used by services and handlers
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub verified: bool,
    pub trust_score: i32,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Job {
    pub id: Uuid,
    pub employer_id: Uuid,
    pub assigned_worker_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub category: String,
    pub budget: BigDecimal,
    pub estimated_duration_days: i32,
    pub status: Option<JobStatus>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkerProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category: String,
    pub experience_years: i32,
    pub description: String,
    pub hourly_rate: Option<BigDecimal>,
    pub daily_rate: Option<BigDecimal>,
    pub location_state: String,
    pub location_city: String,
    pub is_available: Option<bool>,
    pub rating: Option<f32>,
    pub completed_jobs: Option<i32>,
    pub skills: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmployerProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub company_name: Option<String>,
    pub description: Option<String>,
    pub location_state: Option<String>,
    pub location_city: Option<String>,
    pub rating: Option<f32>,
    pub created_at: Option<DateTime<Utc>>,
}

// Feed item is a generic wrapper used by the recommendation & ranking services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    pub id: Uuid,
    pub item_type: FeedItemType,
    /// payload stores a compact JSON representation of the item (job, profile, etc.)
    pub payload: JsonValue,
    /// affinity score computed by the scoring engine
    pub score: f64,
    pub created_at: DateTime<Utc>,
}

impl FeedItem {
    /// Create a new FeedItem. Payload should be a compact JSON representation
    pub fn new(item_type: FeedItemType, payload: JsonValue, score: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            item_type,
            payload,
            score,
            created_at: Utc::now(),
        }
    }
}

// Interaction event used for behavioral tracking
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Interaction {
    pub id: Uuid,
    pub user_id: Uuid,
    /// id of the referenced item (job id, profile id, product id)
    pub item_id: Uuid,
    pub item_type: FeedItemType,
    pub action: InteractionType,
    /// optional numeric value (e.g. time_spent_seconds, rating)
    pub value: Option<f64>,
    pub created_at: Option<DateTime<Utc>>,
}

// Small DTO used by ranking cache to record metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedItemMeta {
    pub id: Uuid,
    pub score: f64,
    pub reason: Option<String>,
}

// Lightweight recommendation response returned by the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedResponse {
    pub user_id: Uuid,
    pub role: UserRole,
    pub items: Vec<RankedItemMeta>,
    pub generated_at: DateTime<Utc>,
}

// Simple helpers used by services
impl Interaction {
    pub fn new(user_id: Uuid, item_id: Uuid, item_type: FeedItemType, action: InteractionType, value: Option<f64>) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            item_id,
            item_type,
            action,
            value,
            created_at: Some(Utc::now()),
        }
    }
}

// Minimal unit tests to ensure the module compiles and basic constructors work
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn feed_item_new_works() {
        let payload = json!({ "title": "Test job", "budget": 1000 });
        let item = FeedItem::new(FeedItemType::Job, payload, 0.75);
        assert_eq!(item.score, 0.75);
    }

    #[test]
    fn interaction_new_works() {
        let user = Uuid::new_v4();
        let item = Uuid::new_v4();
        let inter = Interaction::new(user, item, FeedItemType::Job, InteractionType::View, Some(12.0));
        assert_eq!(inter.user_id, user);
        assert_eq!(inter.item_id, item);
    }
}
