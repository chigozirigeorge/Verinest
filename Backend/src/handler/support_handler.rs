// src/handler/support_handler.rs
use std::sync::Arc;
use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    routing::{get, post, put},
    Extension, Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    db::supportdb::SupportExt,
    error::HttpError,
    middleware::JWTAuthMiddeware,
    models::{
        usermodel::UserRole,
        supportmodel::*
    },

    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTicketDto {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(min = 1, max = 2000))]
    pub description: String,
    pub category: TicketCategory,
    pub priority: TicketPriority,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateMessageDto {
    #[validate(length(min = 1, max = 2000))]
    pub message: String,
    pub is_internal: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTicketStatusDto {
    pub status: TicketStatus,
}

#[derive(Debug, Deserialize)]
pub struct AssignTicketDto {
    pub assigned_to: Uuid,
}

pub fn support_handler() -> Router {
    Router::new()
        .route("/tickets", get(get_tickets).post(create_ticket))
        .route("/tickets/:ticket_id", get(get_ticket))
        .route("/tickets/:ticket_id/status", put(update_ticket_status))
        .route("/tickets/:ticket_id/assign", put(assign_ticket))
        .route("/tickets/:ticket_id/messages", get(get_ticket_messages).post(add_message))
        .route("/my-tickets", get(get_my_tickets))
}

// Create support ticket (for users)
pub async fn create_ticket(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateTicketDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let ticket = app_state.db_client
        .create_support_ticket(
            auth.user.id,
            body.title,
            body.description,
            body.category,
            body.priority,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": ticket
    })))
}

// Get tickets (for customer care)
pub async fn get_tickets(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Query(params): Query<SupportQueryParams>,
) -> Result<impl IntoResponse, HttpError> {
    // Only customer care and admin can access all tickets
    if auth.user.role != UserRole::CustomerCare 
        && auth.user.role != UserRole::Admin {
        return Err(HttpError::unauthorized("Not authorized"));
    }

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let offset = ((page - 1) * limit) as i64;
    let limit = limit as i64;

    let tickets = app_state.db_client
        .get_support_tickets(limit, offset, params.status)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "tickets": tickets,
            "page": page,
            "limit": limit
        }
    })))
}

// Get user's own tickets
pub async fn get_my_tickets(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let tickets = app_state.db_client
        .get_user_support_tickets(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": tickets
    })))
}

// Add message to ticket
pub async fn add_message(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(ticket_id): Path<Uuid>,
    Json(body): Json<CreateMessageDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Verify user has access to the ticket
    let ticket = app_state.db_client
        .get_support_ticket(ticket_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Ticket not found"))?;

    let is_customer_care = auth.user.role == UserRole::CustomerCare 
        || auth.user.role == UserRole::Admin;

    if !is_customer_care && ticket.user_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to access this ticket"));
    }

    let message = app_state.db_client
        .add_ticket_message(
            ticket_id,
            auth.user.id,
            body.message,
            body.is_internal.unwrap_or(false),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Update ticket status if it's a customer care response
    if is_customer_care && ticket.status == TicketStatus::Open {
        app_state.db_client
            .update_ticket_status(ticket_id, TicketStatus::InProgress)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": message
    })))
}

// Update ticket status
pub async fn update_ticket_status(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(ticket_id): Path<Uuid>,
    Json(body): Json<UpdateTicketStatusDto>,
) -> Result<impl IntoResponse, HttpError> {
    // Only customer care and admin can update status
    if auth.user.role != UserRole::CustomerCare 
        && auth.user.role != UserRole::Admin {
        return Err(HttpError::unauthorized("Not authorized"));
    }

    let ticket = app_state.db_client
        .update_ticket_status(ticket_id, body.status)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": ticket
    })))
}

// Assign ticket
pub async fn assign_ticket(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(ticket_id): Path<Uuid>,
    Json(body): Json<AssignTicketDto>,
) -> Result<impl IntoResponse, HttpError> {
    // Only customer care and admin can assign tickets
    if auth.user.role != UserRole::CustomerCare 
        && auth.user.role != UserRole::Admin {
        return Err(HttpError::unauthorized("Not authorized"));
    }

    let ticket = app_state.db_client
        .assign_ticket(ticket_id, body.assigned_to)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": ticket
    })))
}

// Get ticket details
pub async fn get_ticket(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(ticket_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let ticket = app_state.db_client
        .get_support_ticket_with_messages(ticket_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Ticket not found"))?;

    let is_customer_care = auth.user.role == UserRole::CustomerCare 
        || auth.user.role == UserRole::Admin;

    if !is_customer_care && ticket.ticket.user_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to access this ticket"));
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": ticket
    })))
}

// Get ticket messages
pub async fn get_ticket_messages(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(ticket_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let ticket = app_state.db_client
        .get_support_ticket(ticket_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Ticket not found"))?;

    let is_customer_care = auth.user.role == UserRole::CustomerCare 
        || auth.user.role == UserRole::Admin;

    if !is_customer_care && ticket.user_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to access this ticket"));
    }

    let messages = app_state.db_client
        .get_ticket_messages(ticket_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": messages
    })))
}