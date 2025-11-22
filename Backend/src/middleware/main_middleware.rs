//9
use std::sync::Arc;

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::IntoResponse,
    Extension
};

use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use redis::aio::ConnectionManager;

use crate::{
    db::userdb::UserExt,
    error::{ErrorMessage, HttpError},
    models::usermodel::{User, UserRole},
    utils::token,
    AppState
};
use crate::db::cache::CacheHelper;
use crate::middleware::cache_invalidation::{CacheInvalidationConfig, invalidate_cache_for_request, should_cache_request};
use axum::body;
use axum::Json;
use axum::http::Method;
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JWTAuthMiddeware {
    pub user: User,
}

pub async fn auth(
    cookie_jar: CookieJar,
    Extension(app_state): Extension<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<impl IntoResponse, HttpError> {
    let cookies = cookie_jar
            .get("token")
            .map(|cookie| cookie.value().to_string())
            .or_else(|| {
                req.headers()
                    .get(header::AUTHORIZATION)
                    .and_then(|auth_header| auth_header.to_str().ok())
                    .and_then(|auth_value| {
                        if auth_value.starts_with("Bearer ") {
                            Some(auth_value[7..].to_owned())
                        } else {
                            None
                        }
                    })  
            });

    let token = cookies.ok_or_else(|| {
        HttpError::unauthorized(ErrorMessage::TokenNotProvided.to_string())
    })?;

    let token_details = 
        match token::decode_token(token, app_state.env.jwt_secret.as_bytes()) {
            Ok(token_details) => token_details,
            Err(_) => {
                return Err(HttpError::unauthorized(ErrorMessage::InvalidToken.to_string()));
            }
        };

    // ✅ FIX #6: Check if token is blacklisted (user logged out)
    if let Some(redis_client) = &app_state.db_client.redis_client {
        let blacklist_key = format!("token_blacklist:{}", token_details);
        let mut conn = ConnectionManager::clone(redis_client);
        
        let is_blacklisted: bool = redis::cmd("EXISTS")
            .arg(&blacklist_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(false);
        
        if is_blacklisted {
            return Err(HttpError::unauthorized("Token has been revoked. Please login again.".to_string()));
        }
    }

    let user_id = uuid::Uuid::parse_str(&token_details)
            .map_err(|_| {
                HttpError::unauthorized(ErrorMessage::InvalidToken.to_string())
            })?;

    let user = app_state.db_client.get_user(Some(user_id), None, None, None)
            .await
            .map_err(|_| {
                HttpError::unauthorized(ErrorMessage::UserNoLongerExist.to_string())
            })?;

    let user = user.ok_or_else(|| {
        HttpError::unauthorized(ErrorMessage::UserNoLongerExist.to_string())
    })?;

    req.extensions_mut().insert(JWTAuthMiddeware {
        user: user.clone(),
    });

    Ok(next.run(req).await)

}


pub async fn role_check(
    Extension(_app_state): Extension<Arc<AppState>>,
    req: Request,
    next: Next,
    required_roles: Vec<UserRole>,
) -> Result<impl IntoResponse, HttpError> {
    let user = req
            .extensions()
            .get::<JWTAuthMiddeware>()
            .ok_or_else(|| {
                HttpError::unauthorized(ErrorMessage::UserNotAuthenticated.to_string())
            })?;
    
    if !required_roles.contains(&user.user.role) {
        return Err(HttpError::new(ErrorMessage::PermissionDenied.to_string(), StatusCode::FORBIDDEN));
    }

    Ok(next.run(req).await)
}

/// Middleware that implements smart caching for GET requests and intelligent cache invalidation
/// for POST/PUT/DELETE requests based on endpoint groupings. It also enforces rate limits
/// for sensitive endpoints.
pub async fn cache_and_rate_limit(mut req: Request, next: Next) -> Result<impl IntoResponse, HttpError> {
    // Initialize cache invalidation config
    let cache_config = CacheInvalidationConfig::new();
    
    // Try to get AppState from request extensions
    let app_state = req
        .extensions()
        .get::<Arc<AppState>>()
        .cloned()
        .ok_or_else(|| HttpError::server_error("AppState missing from request extensions".to_string()))?;

    // Ensure Option<JWTAuthMiddeware> extension exists
    req.extensions_mut().insert::<Option<JWTAuthMiddeware>>(None);

    // Get request info
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    
    tracing::info!("🔍 Request: {} {}", method, path);

    // Rate limiting for sensitive endpoints
    if let Some(ref redis_arc) = app_state.db_client.redis_client {
        let ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        // Apply rate limits for specific endpoints
        if method == Method::POST && path == "/api/auth/login" {
            let key = format!("rl:login:{}", ip);
            let mut conn = ConnectionManager::clone(redis_arc);
            let count: i64 = redis::cmd("INCR").arg(&key).query_async(&mut conn).await.map_err(|e| HttpError::server_error(e.to_string()))?;
            if count == 1 {
                let _ : () = redis::cmd("EXPIRE").arg(&key).arg(3600).query_async(&mut conn).await.map_err(|e| HttpError::server_error(e.to_string()))?;
            }
            if count > 5 {
                return Err(HttpError::new(format!("Too many attempts"), StatusCode::TOO_MANY_REQUESTS));
            }
        }

        if method == Method::PUT && path == "/api/users/password" {
            let key = format!("rl:change_password:{}", ip);
            let mut conn = ConnectionManager::clone(redis_arc);
            let count: i64 = redis::cmd("INCR").arg(&key).query_async(&mut conn).await.map_err(|e| HttpError::server_error(e.to_string()))?;
            if count == 1 {
                let _ : () = redis::cmd("EXPIRE").arg(&key).arg(3600).query_async(&mut conn).await.map_err(|e| HttpError::server_error(e.to_string()))?;
            }
            if count > 3 {
                return Err(HttpError::new(format!("Too many attempts"), StatusCode::TOO_MANY_REQUESTS));
            }
        }

        if method == Method::POST && path == "/api/users/transaction-pin/verify" {
            let key = format!("rl:verify_pin:{}", ip);
            let mut conn = ConnectionManager::clone(redis_arc);
            let count: i64 = redis::cmd("INCR").arg(&key).query_async(&mut conn).await.map_err(|e| HttpError::server_error(e.to_string()))?;
            if count == 1 {
                let _ : () = redis::cmd("EXPIRE").arg(&key).arg(3600).query_async(&mut conn).await.map_err(|e| HttpError::server_error(e.to_string()))?;
            }
            if count > 3 {
                return Err(HttpError::new(format!("Too many attempts"), StatusCode::TOO_MANY_REQUESTS));
            }
        }
    }

    // Handle GET requests - serve from cache if available
    if method == Method::GET {
        // Check if this endpoint should be cached
        if !should_cache_request(&method.to_string(), &path, &cache_config) {
            tracing::info!("🔍 Endpoint {} {} not cacheable, proceeding", method, path);
            return Ok(next.run(req).await);
        }

        if let Some(ref redis_arc) = app_state.db_client.redis_client {
            // Resolve user for cache key
            let mut user_tag = "anon".to_string();
            if let Some(j) = req.extensions().get::<JWTAuthMiddeware>() {
                user_tag = j.user.id.to_string();
            } else if let Some(auth_header) = req.headers().get(header::AUTHORIZATION).and_then(|h| h.to_str().ok()) {
                if auth_header.starts_with("Bearer ") {
                    let token_str = &auth_header[7..];
                    if let Ok(token_details) = crate::utils::token::decode_token(token_str.to_string(), app_state.env.jwt_secret.as_bytes()) {
                        if let Ok(user_uuid) = uuid::Uuid::parse_str(&token_details) {
                            if let Ok(Some(user)) = app_state.db_client.get_user(Some(user_uuid), None, None, None).await {
                                req.extensions_mut().insert(JWTAuthMiddeware { user: user.clone() });
                                req.extensions_mut().insert::<Option<JWTAuthMiddeware>>(Some(JWTAuthMiddeware { user: user.clone() }));
                                user_tag = user_uuid.to_string();
                            }
                        }
                    }
                }
            } else if let Some(cookie_header) = req.headers().get(header::COOKIE).and_then(|h| h.to_str().ok()) {
                if let Some(pair) = cookie_header.split(';').map(|s| s.trim()).find(|s| s.starts_with("token=")) {
                    if let Some(tok) = pair.strip_prefix("token=") {
                        if let Ok(token_details) = crate::utils::token::decode_token(tok.to_string(), app_state.env.jwt_secret.as_bytes()) {
                            if let Ok(user_uuid) = uuid::Uuid::parse_str(&token_details) {
                                if let Ok(Some(user)) = app_state.db_client.get_user(Some(user_uuid), None, None, None).await {
                                    req.extensions_mut().insert(JWTAuthMiddeware { user: user.clone() });
                                    req.extensions_mut().insert::<Option<JWTAuthMiddeware>>(Some(JWTAuthMiddeware { user: user.clone() }));
                                    user_tag = user_uuid.to_string();
                                }
                            }
                        }
                    }
                }
            }

            let cache_key = format!("cache:GET:{}:{}", req.uri().to_string(), user_tag);
            tracing::info!("🔍 Checking cache for key: {}", cache_key);

            if let Ok(Some(cached_value)) = CacheHelper::get::<Value>(redis_arc, &cache_key).await {
                tracing::info!("🎯 Cache HIT for: {}", cache_key);
                return Ok(Json(cached_value).into_response());
            } else {
                tracing::info!("❌ Cache MISS for: {}", cache_key);
            }

            // Not cached -> run downstream and cache successful JSON responses
            let response = next.run(req).await;
            let status = response.status();
            
            if status.is_success() {
                if let Some(ct) = response.headers().get(header::CONTENT_TYPE) {
                    if let Ok(ct_str) = ct.to_str() {
                        if ct_str.contains("application/json") {
                            // Decompose response and buffer body for caching
                            let (parts, body) = response.into_parts();
                            if let Ok(bytes) = axum::body::to_bytes(body, 64 * 1024).await {
                                if let Ok(body_str) = String::from_utf8(bytes.to_vec()) {
                                    if let Ok(json_val) = serde_json::from_str::<Value>(&body_str) {
                                        // Cache the response
                                        let _ = CacheHelper::set(redis_arc, &cache_key, &json_val, 3600).await;
                                        tracing::info!("💾 Cached response for: {}", cache_key);
                                        // Return the JSON response
                                        return Ok(Json(json_val).into_response());
                                    }
                                }
                            }
                            // If buffering or parsing failed, return server error
                            return Err(HttpError::server_error("Failed to buffer/parse JSON response for caching".to_string()));
                        }
                    }
                }
            }

            return Ok(response);
        }
    }

    // For POST/PUT/DELETE: run the handler and then invalidate related cache
    let response = next.run(req).await;
    
    // Only invalidate on successful mutations
    if response.status().is_success() {
        if let Some(ref redis_arc) = app_state.db_client.redis_client {
            if let Err(e) = invalidate_cache_for_request(redis_arc, &cache_config, &method.to_string(), &path).await {
                tracing::error!("🔥 Failed to invalidate cache: {:?}", e);
            }
        }
    }

    Ok(response)
}