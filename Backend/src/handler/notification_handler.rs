// // src/handler/notification_handler.rs
// use std::sync::Arc;
// use axum::{
//     extract::{Path, Query},
//     http::StatusCode,
//     response::{IntoResponse, Json},
//     routing::{get, post},
//     Router,
//     Extension,
// };
// use serde::Deserialize;
// use uuid::Uuid;

// use crate::{
//     service::notification_service::NotificationService,
//     error::HttpError,
//     middleware::JWTAuthMiddeware,
//     AppState,
// };

// #[derive(Debug, Deserialize)]
// pub struct PaginationParams {
//     pub page: Option<u32>,
//     pub limit: Option<u32>,
// }

// #[derive(Debug, Deserialize)]
// pub struct MarkReadRequest {
//     pub notification_ids: Option<Vec<Uuid>>,
// }

// pub fn notification_routes() -> Router {
//     Router::new()
//     .route("/", get(get_user_notifications))
//     .route("/read", post(mark_notification_read))
//     .route("/read-all", post(mark_all_notifications_read))
//     .route("/:id/read", post(mark_single_notification_read))
//     .route("/unread-count", get(get_unread_count))
// }

// // FIXED: Use impl IntoResponse and HttpError
// async fn get_user_notifications(
//     Extension(app_state): Extension<Arc<AppState>>,
//     Extension(auth): Extension<JWTAuthMiddeware>,
//     Query(pagination): Query<PaginationParams>,
// ) -> Result<impl IntoResponse, HttpError> {
//     let notification_service = NotificationService::new(app_state.db_client.clone());
    
//     let page = pagination.page.unwrap_or(1);
//     let limit = pagination.limit.unwrap_or(20).min(100) as i64;
//     let offset = (page - 1) * limit as u32;

//     let notifications = notification_service
//         .get_user_notifications(auth.user.id, limit, offset as i64)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?;

//     Ok(Json(notifications))
// }

// // FIXED: Use impl IntoResponse and HttpError
// async fn mark_notification_read(
//     Extension(app_state): Extension<Arc<AppState>>,
//     Extension(auth): Extension<JWTAuthMiddeware>,
//     Json(payload): Json<MarkReadRequest>,
// ) -> Result<impl IntoResponse, HttpError> {
//     let notification_service = NotificationService::new(app_state.db_client.clone());
    
//     if let Some(notification_ids) = payload.notification_ids {
//         for notification_id in notification_ids {
//             notification_service
//                 .mark_notification_read(notification_id, auth.user.id)
//                 .await
//                 .map_err(|e| HttpError::server_error(e.to_string()))?;
//         }
//     }

//     Ok(StatusCode::OK)
// }

// // FIXED: Use impl IntoResponse and HttpError
// async fn mark_all_notifications_read(
//     Extension(app_state): Extension<Arc<AppState>>,
//     Extension(auth): Extension<JWTAuthMiddeware>,
// ) -> Result<impl IntoResponse, HttpError> {
//     let notification_service = NotificationService::new(app_state.db_client.clone());
    
//     notification_service
//         .mark_all_notifications_read(auth.user.id)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?;

//     Ok(StatusCode::OK)
// }

// // FIXED: Use impl IntoResponse and HttpError
// async fn mark_single_notification_read(
//     Extension(app_state): Extension<Arc<AppState>>,
//     Extension(auth): Extension<JWTAuthMiddeware>,
//     Path(notification_id): Path<Uuid>,
// ) -> Result<impl IntoResponse, HttpError> {
//     let notification_service = NotificationService::new(app_state.db_client.clone());
    
//     notification_service
//         .mark_notification_read(notification_id, auth.user.id)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?;

//     Ok(StatusCode::OK)
// }

// // FIXED: Use impl IntoResponse and HttpError
// async fn get_unread_count(
//     Extension(app_state): Extension<Arc<AppState>>,
//     Extension(auth): Extension<JWTAuthMiddeware>,
// ) -> Result<impl IntoResponse, HttpError> {
//     let count: i64 = sqlx::query_scalar(
//         r#"
//         SELECT COUNT(*) 
//         FROM notifications 
//         WHERE user_id = $1 AND is_read = false
//         "#
//     )
//     .bind(auth.user.id)
//     .fetch_one(&app_state.db_client.pool)
//     .await
//     .map_err(|e| HttpError::server_error(e.to_string()))?;

//     Ok(Json(serde_json::json!({
//         "unread_count": count
//     })))
// }

// // Additional handler for notification preferences (if you add them later)
// #[derive(Debug, Deserialize)]
// pub struct NotificationPreferences {
//     pub email_enabled: Option<bool>,
//     pub push_enabled: Option<bool>,
//     pub job_alerts: Option<bool>,
//     pub payment_alerts: Option<bool>,
//     pub dispute_alerts: Option<bool>,
// }

// // POST /notifications/preferences - Update user notification preferences
// async fn update_notification_preferences(
//     Extension(_app_state): Extension<Arc<AppState>>,
//     Extension(auth_user): Extension<JWTAuthMiddeware>,
//     Json(preferences): Json<NotificationPreferences>,
// ) -> Result<impl IntoResponse, HttpError> {
//     // This would update user notification preferences in the database
//     // For now, we'll just log and return success
    
//     tracing::info!(
//         "User {} updated notification preferences: {:?}",
//         auth_user.user.id,
//         preferences
//     );

//     // In a real implementation, you'd update a user_preferences table
//     // sqlx::query!(
//     //     r#"
//     //     INSERT INTO user_notification_preferences 
//     //     ...
//     //     "#,
//     //     ...
//     // )
//     // .execute(&db_client.pool)
//     // .await?;

//     Ok(StatusCode::OK)
// }








// src/handlers/notification_handler.rs - FIXED VERSION
use std::sync::Arc;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{
    db::db::DBClient,
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

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub related_id: Option<Uuid>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub notifications: Vec<Notification>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub unread_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub message: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn success(message: &str, data: T) -> Self {
        Self {
            status: "success".to_string(),
            message: message.to_string(),
            data,
        }
    }
}

pub fn notification_routes() -> Router {
    Router::new()
        .route("/", get(get_user_notifications))
        .route("/unread-count", get(get_unread_count))
        .route("/read", post(mark_notifications_read))
        .route("/read-all", post(mark_all_notifications_read))
        .route("/:id/read", put(mark_single_notification_read))
        .route("/:id", delete(delete_notification))
}

// Get user notifications with pagination
async fn get_user_notifications(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, HttpError> {
    let page = pagination.page.unwrap_or(1);
    let limit = pagination.limit.unwrap_or(20).min(100) as i64;
    let offset = ((page - 1) * limit as u32) as i64;

    println!("📬 [get_user_notifications] Fetching for user: {}", auth.user.id);

    // Get total count
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) 
        FROM notifications 
        WHERE user_id = $1
        "#
    )
    .bind(auth.user.id)
    .fetch_one(&app_state.db_client.pool)
    .await
    .map_err(|e| {
        println!("❌ [get_user_notifications] Count query failed: {}", e);
        HttpError::server_error(format!("Failed to count notifications: {}", e))
    })?;

    // Get unread count
    let unread_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) 
        FROM notifications 
        WHERE user_id = $1 AND is_read = false
        "#
    )
    .bind(auth.user.id)
    .fetch_one(&app_state.db_client.pool)
    .await
    .map_err(|e| {
        println!("❌ [get_user_notifications] Unread count query failed: {}", e);
        HttpError::server_error(format!("Failed to count unread notifications: {}", e))
    })?;

    // Get notifications
    let notifications = sqlx::query_as::<_, Notification>(
        r#"
        SELECT id, user_id, title, message, notification_type, 
               related_id, is_read, created_at
        FROM notifications 
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(auth.user.id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&app_state.db_client.pool)
    .await
    .map_err(|e| {
        println!("❌ [get_user_notifications] Query failed: {}", e);
        HttpError::server_error(format!("Failed to fetch notifications: {}", e))
    })?;

    println!("✅ [get_user_notifications] Found {} notifications", notifications.len());

    let response = NotificationResponse {
        notifications,
        total,
        page,
        limit: limit as u32,
        unread_count,
    };

    Ok(Json(response))
}

// Get unread count
async fn get_unread_count(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    println!("📬 [get_unread_count] Fetching for user: {}", auth.user.id);

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
    .map_err(|e| {
        println!("❌ [get_unread_count] Query failed: {}", e);
        HttpError::server_error(format!("Failed to count notifications: {}", e))
    })?;

    println!("✅ [get_unread_count] Unread count: {}", count);

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "unread_count": count
        }
    })))
}

// Mark specific notifications as read
async fn mark_notifications_read(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(payload): Json<MarkReadRequest>,
) -> Result<impl IntoResponse, HttpError> {
    println!("📬 [mark_notifications_read] For user: {}", auth.user.id);

    if let Some(notification_ids) = payload.notification_ids {
        for notification_id in &notification_ids {
            sqlx::query(
                r#"
                UPDATE notifications 
                SET is_read = true
                WHERE id = $1 AND user_id = $2
                "#
            )
            .bind(notification_id)
            .bind(auth.user.id)
            .execute(&app_state.db_client.pool)
            .await
            .map_err(|e| {
                println!("❌ [mark_notifications_read] Failed for {}: {}", notification_id, e);
                HttpError::server_error(format!("Failed to mark notification as read: {}", e))
            })?;
        }

        println!("✅ [mark_notifications_read] Marked {} notifications as read", notification_ids.len());
    }

    Ok(Json(ApiResponse::success(
        "Notifications marked as read",
        serde_json::json!({}),
    )))
}

// Mark all notifications as read
async fn mark_all_notifications_read(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    println!("📬 [mark_all_notifications_read] For user: {}", auth.user.id);

    let result = sqlx::query(
        r#"
        UPDATE notifications 
        SET is_read = true
        WHERE user_id = $1 AND is_read = false
        "#
    )
    .bind(auth.user.id)
    .execute(&app_state.db_client.pool)
    .await
    .map_err(|e| {
        println!("❌ [mark_all_notifications_read] Failed: {}", e);
        HttpError::server_error(format!("Failed to mark all notifications as read: {}", e))
    })?;

    println!("✅ [mark_all_notifications_read] Marked {} notifications as read", result.rows_affected());

    Ok(Json(ApiResponse::success(
        "All notifications marked as read",
        serde_json::json!({
            "updated_count": result.rows_affected()
        }),
    )))
}

// Mark single notification as read
async fn mark_single_notification_read(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(notification_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    println!("📬 [mark_single_notification_read] Notification: {}", notification_id);

    sqlx::query(
        r#"
        UPDATE notifications 
        SET is_read = true
        WHERE id = $1 AND user_id = $2
        "#
    )
    .bind(notification_id)
    .bind(auth.user.id)
    .execute(&app_state.db_client.pool)
    .await
    .map_err(|e| {
        println!("❌ [mark_single_notification_read] Failed: {}", e);
        HttpError::server_error(format!("Failed to mark notification as read: {}", e))
    })?;

    println!("✅ [mark_single_notification_read] Marked as read");

    Ok(Json(ApiResponse::success(
        "Notification marked as read",
        serde_json::json!({}),
    )))
}

// Delete notification
async fn delete_notification(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(notification_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    println!("📬 [delete_notification] Notification: {}", notification_id);

    let result = sqlx::query(
        r#"
        DELETE FROM notifications 
        WHERE id = $1 AND user_id = $2
        "#
    )
    .bind(notification_id)
    .bind(auth.user.id)
    .execute(&app_state.db_client.pool)
    .await
    .map_err(|e| {
        println!("❌ [delete_notification] Failed: {}", e);
        HttpError::server_error(format!("Failed to delete notification: {}", e))
    })?;

    if result.rows_affected() == 0 {
        return Err(HttpError::not_found("Notification not found or already deleted"));
    }

    println!("✅ [delete_notification] Deleted successfully");

    Ok(Json(ApiResponse::success(
        "Notification deleted",
        serde_json::json!({}),
    )))
}