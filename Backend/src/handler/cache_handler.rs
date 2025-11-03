// handler/cache_handler.rs
use axum::{
    extract::Extension,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::{
    AppState, 
    db::{
        cache::CacheHelper,
        userdb::UserExt,
    }, 
    error::HttpError, 
    middleware::JWTAuthMiddeware, 
    models::usermodel::UserRole
};

pub fn cache_handler() -> Router {
    Router::new()
        .route("/cache/stats", get(get_cache_stats))
        .route("/cache/health", get(cache_health_check))
        .route("/cache/clear", post(clear_cache))
        .route("/cache/clear-chats", post(clear_chat_caches))
}

/// Get cache statistics
pub async fn get_cache_stats(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(_auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    // TODO: Add admin check
    // if !auth.user.is_admin {
    //     return Err(HttpError::unauthorized("Admin access required"));
    // }
    
    if let Some(redis_client) = &app_state.db_client.redis_client {
        match CacheHelper::get_cache_stats(redis_client).await {
            Ok(stats) => {
                Ok(Json(serde_json::json!({
                    "status": "success",
                    "data": {
                        "cache_enabled": true,
                        "hits": stats.hits,
                        "misses": stats.misses,
                        "total_requests": stats.total_requests(),
                        "hit_rate": format!("{:.2}%", stats.hit_rate()),
                        "performance_boost": format!("{}x faster", calculate_speed_boost(stats.hit_rate()))
                    }
                })))
            }
            Err(e) => {
                Err(HttpError::server_error(format!("Failed to get cache stats: {}", e)))
            }
        }
    } else {
        Ok(Json(serde_json::json!({
            "status": "success",
            "data": {
                "cache_enabled": false,
                "message": "Redis caching is not enabled. Set REDIS_URL to enable caching."
            }
        })))
    }
}

/// Health check endpoint for Redis
pub async fn cache_health_check(
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpError> {
    if let Some(redis_client) = &app_state.db_client.redis_client {
        match CacheHelper::health_check(redis_client).await {
            Ok(true) => {
                Ok(Json(serde_json::json!({
                    "status": "healthy",
                    "cache_enabled": true,
                    "redis_status": "connected",
                    "message": "Redis is responding normally"
                })))
            }
            Ok(false) => {
                Ok(Json(serde_json::json!({
                    "status": "degraded",
                    "cache_enabled": true,
                    "redis_status": "unexpected_response",
                    "message": "Redis returned unexpected response"
                })))
            }
            Err(e) => {
                tracing::error!("Redis health check failed: {}", e);
                Ok(Json(serde_json::json!({
                    "status": "unhealthy",
                    "cache_enabled": false,
                    "redis_status": "connection_failed",
                    "error": e.to_string()
                })))
            }
        }
    } else {
        Ok(Json(serde_json::json!({
            "status": "healthy",
            "cache_enabled": false,
            "redis_status": "not_configured",
            "message": "Application is running without Redis caching"
        })))
    }
}

/// Clear all chat-related caches (admin only)
pub async fn clear_chat_caches(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(_auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    // TODO: Add admin check
    // if !auth.user.is_admin {
    //     return Err(HttpError::unauthorized("Admin access required"));
    // }
    
    if let Some(redis_client) = &app_state.db_client.redis_client {
        match CacheHelper::clear_all_chat_caches(redis_client).await {
            Ok(_) => {
                tracing::info!("Chat caches cleared by admin");
                Ok(Json(serde_json::json!({
                    "status": "success",
                    "message": "All chat caches cleared successfully"
                })))
            }
            Err(e) => {
                Err(HttpError::server_error(format!("Failed to clear chat caches: {}", e)))
            }
        }
    } else {
        Err(HttpError::bad_request("Redis caching is not enabled"))
    }
}

/// Clear ALL caches (admin only - use with extreme caution)
pub async fn clear_cache(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user = app_state.db_client
        .get_user(Some(auth.user.id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if auth.user.role != UserRole::Admin {
        return Err(HttpError::unauthorized("Admin access required"));
    }
    
    if let Some(redis_client) = &app_state.db_client.redis_client {
        match CacheHelper::clear_all_caches(redis_client).await {
            Ok(_) => {
                tracing::warn!("ALL caches cleared by admin - this should only happen in dev/testing!");
                Ok(Json(serde_json::json!({
                    "status": "success",
                    "message": "All caches cleared successfully",
                    "warning": "This affects all cached data across the entire application"
                })))
            }
            Err(e) => {
                Err(HttpError::server_error(format!("Failed to clear caches: {}", e)))
            }
        }
    } else {
        Err(HttpError::bad_request("Redis caching is not enabled"))
    }
}

/// Calculate speed boost based on hit rate
fn calculate_speed_boost(hit_rate: f64) -> u32 {
    // Assuming cached requests are 20-50x faster on average
    // This is a conservative estimate
    if hit_rate >= 90.0 {
        40
    } else if hit_rate >= 80.0 {
        30
    } else if hit_rate >= 70.0 {
        20
    } else if hit_rate >= 60.0 {
        15
    } else if hit_rate >= 50.0 {
        10
    } else {
        5
    }
}