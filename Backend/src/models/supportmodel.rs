// src/models/supportmodel.rs
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "ticket_status", rename_all = "snake_case")]
pub enum TicketStatus {
    Open,
    InProgress,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "ticket_priority", rename_all = "snake_case")]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "ticket_category", rename_all = "snake_case")]
pub enum TicketCategory {
    General,
    Technical,
    Billing,
    Account,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SupportTicket {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: String,
    pub category: TicketCategory,
    pub priority: TicketPriority,
    pub status: TicketStatus,
    pub assigned_to: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SupportTicketWithUser {
    #[sqlx(flatten)]
    pub ticket: SupportTicket,
    pub user_name: String,
    pub user_email: String,
    pub user_username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SupportMessage {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SupportMessageWithUser {
    #[sqlx(flatten)]
    pub message: SupportMessage,
    pub user_name: String,
    pub user_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicketWithMessages {
    pub ticket: SupportTicket,
    pub messages: Vec<SupportMessageWithUser>,
}

#[derive(Debug, Deserialize)]
pub struct SupportQueryParams {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub status: Option<TicketStatus>,
}