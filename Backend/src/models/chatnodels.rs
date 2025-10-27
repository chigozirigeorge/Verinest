// models/chatmodels.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "message_type", rename_all = "snake_case")]
pub enum MessageType {
    Text,
    Image,
    File,
    ContractProposal,
    JobReference,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "chat_status", rename_all = "snake_case")]
pub enum ChatStatus {
    Active,
    Archived,
    Blocked,
}

#[derive(Debug, Serialize, Clone, Deserialize, sqlx::FromRow)]
pub struct Chat {
    pub id: Uuid,
    pub participant_one_id: Uuid,
    pub participant_two_id: Uuid,
    pub job_id: Option<Uuid>,
    pub status: Option<ChatStatus>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Message {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub sender_id: Uuid,
    pub message_type: MessageType,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub is_read: Option<bool>,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContractProposal {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub message_id: Uuid,
    pub job_id: Uuid,
    pub proposed_by: Uuid,
    pub worker_id: Uuid,
    pub employer_id: Uuid,
    pub agreed_rate: f64,
    pub agreed_timeline: i32,
    pub terms: String,
    pub status: Option<String>, // pending, accepted, rejected
    pub created_at: Option<DateTime<Utc>>,
    pub responded_at: Option<DateTime<Utc>>,
}
