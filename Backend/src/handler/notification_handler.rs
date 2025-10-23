// src/handler/notification_handler.rs
use std::sync::Arc;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
    Extension,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    service::notification_service::NotificationService,
    error::HttpError,
    middleware::JWTAuthMiddeware,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct MarkReadRequest {
    pub notification_ids: Option<Vec<Uuid>>,
}

pub fn notification_routes() -> Router {
    Router::new()
    .route("/", get(get_user_notifications))
    .route("/read", post(mark_notification_read))
    .route("/read-all", post(mark_all_notifications_read))
    .route("/:id/read", post(mark_single_notification_read))
    .route("/unread-count", get(get_unread_count))
}

// FIXED: Use impl IntoResponse and HttpError
async fn get_user_notifications(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, HttpError> {
    let notification_service = NotificationService::new(app_state.db_client.clone());
    
    let page = pagination.page.unwrap_or(1);
    let limit = pagination.limit.unwrap_or(20).min(100) as i64;
    let offset = (page - 1) * limit as u32;

    let notifications = notification_service
        .get_user_notifications(auth.user.id, limit, offset as i64)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(notifications))
}

// FIXED: Use impl IntoResponse and HttpError
async fn mark_notification_read(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(payload): Json<MarkReadRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let notification_service = NotificationService::new(app_state.db_client.clone());
    
    if let Some(notification_ids) = payload.notification_ids {
        for notification_id in notification_ids {
            notification_service
                .mark_notification_read(notification_id, auth.user.id)
                .await
                .map_err(|e| HttpError::server_error(e.to_string()))?;
        }
    }

    Ok(StatusCode::OK)
}

// FIXED: Use impl IntoResponse and HttpError
async fn mark_all_notifications_read(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let notification_service = NotificationService::new(app_state.db_client.clone());
    
    notification_service
        .mark_all_notifications_read(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(StatusCode::OK)
}

// FIXED: Use impl IntoResponse and HttpError
async fn mark_single_notification_read(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(notification_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let notification_service = NotificationService::new(app_state.db_client.clone());
    
    notification_service
        .mark_notification_read(notification_id, auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(StatusCode::OK)
}

// FIXED: Use impl IntoResponse and HttpError
async fn get_unread_count(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) 
        FROM notifications 
        WHERE user_id = $1 AND is_read = false
        "#
    )
    .bind(auth.user.id)
    .fetch_one(&app_state.db_client.pool)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "unread_count": count
    })))
}

// Additional handler for notification preferences (if you add them later)
#[derive(Debug, Deserialize)]
pub struct NotificationPreferences {
    pub email_enabled: Option<bool>,
    pub push_enabled: Option<bool>,
    pub job_alerts: Option<bool>,
    pub payment_alerts: Option<bool>,
    pub dispute_alerts: Option<bool>,
}

// POST /notifications/preferences - Update user notification preferences
async fn update_notification_preferences(
    Extension(_app_state): Extension<Arc<AppState>>,
    Extension(auth_user): Extension<JWTAuthMiddeware>,
    Json(preferences): Json<NotificationPreferences>,
) -> Result<impl IntoResponse, HttpError> {
    // This would update user notification preferences in the database
    // For now, we'll just log and return success
    
    tracing::info!(
        "User {} updated notification preferences: {:?}",
        auth_user.user.id,
        preferences
    );

    // In a real implementation, you'd update a user_preferences table
    // sqlx::query!(
    //     r#"
    //     INSERT INTO user_notification_preferences 
    //     ...
    //     "#,
    //     ...
    // )
    // .execute(&db_client.pool)
    // .await?;

    Ok(StatusCode::OK)
}