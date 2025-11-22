// Rate limiting middleware for wallet operations
use std::time::Duration;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use axum::{
    extract::{Request, State},
    http::{StatusCode},
    middleware::Next,
    response::Response,
};

// Simple in-memory rate limiter (for production, use Redis)
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<std::time::Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    pub fn is_allowed(&self, key: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = std::time::Instant::now();
        
        let entry = requests.entry(key.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests outside the window
        entry.retain(|&timestamp| now.duration_since(timestamp) < self.window);
        
        // Check if under limit
        if entry.len() < self.max_requests {
            entry.push(now);
            true
        } else {
            false
        }
    }
}

// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get client identifier (IP address or user ID for authenticated requests)
    let client_id = get_client_id(&request);
    
    if !limiter.is_allowed(&client_id) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    Ok(next.run(request).await)
}

fn get_client_id(request: &Request) -> String {
    // Try to get user ID from JWT if available
    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                // Extract user ID from JWT token
                let token = &auth_str[7..];
                if let Ok(user_id) = extract_user_id_from_jwt(token) {
                    return format!("user:{}", user_id);
                }
            }
        }
    }
    
    // Fallback to IP address
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .unwrap_or("unknown")
        .to_string()
}

// Extract user ID from JWT token
fn extract_user_id_from_jwt(token: &str) -> Result<String, Box<dyn std::error::Error>> {
    use jsonwebtoken::{decode, Validation, DecodingKey};
    use serde_json::Value;
    
    // Decode token without verification (just to get user ID for rate limiting)
    let token_data = decode::<Value>(
        token,
        &DecodingKey::from_secret(b"your-secret-key"), // This won't be used for validation
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )?;
    
    if let Some(claims) = token_data.claims.get("sub") {
        if let Some(user_id) = claims.as_str() {
            return Ok(user_id.to_string());
        }
    }
    
    Err("User ID not found in token".into())
}

// Rate limiting configurations
pub fn wallet_rate_limiter() -> RateLimiter {
    RateLimiter::new(10, Duration::from_secs(60)) // 10 requests per minute
}

pub fn deposit_rate_limiter() -> RateLimiter {
    RateLimiter::new(5, Duration::from_secs(60)) // 5 deposits per minute
}

pub fn webhook_rate_limiter() -> RateLimiter {
    RateLimiter::new(100, Duration::from_secs(60)) // 100 webhooks per minute
}
