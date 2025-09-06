use axum::{extract::Query, response::{IntoResponse, Redirect}, routing::get, Extension, Json, Router};
use time::Duration;
use std:: sync::Arc;
use serde::Deserialize;
use oauth2::CsrfToken;
use axum_extra::extract::cookie::{Cookie, CookieJar};

use crate::{
    db::UserExt, 
    error::HttpError, 
    middleware::JWTAuthMiddeware, 
    service::google_oauth::GoogleAuthService, 
    utils::token, AppState
};

#[derive(Debug, Deserialize)]
pub struct GoogleAuthQuery {
    pub code: String,
    pub state: Option<String>,
}

pub fn oauth_handler() -> Router {
    Router::new()
        .route("/google", get(google_login))
        .route("/google/callback", get(google_callback))
        // Add to your routes:
.route("/test-url", get(test_url_generation))
}

pub async fn google_login(
    Extension(app_state): Extension<Arc<AppState>>,
    jar: CookieJar,
) -> Result<(CookieJar, impl IntoResponse), HttpError> {
    println!("=== GOOGLE LOGIN STARTED ===");
    
    let google_oauth = GoogleAuthService::new()
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let redirect_url = format!("https://verinest.vercel.app/api/oauth/google/callback");
    println!("📋 Redirect URL: {}", redirect_url);

    let state = CsrfToken::new_random();
    let state_secret = state.secret().to_string();
    println!("🔑 Generated state: {}", state_secret);

    // google_oauth.store_csrf_state(state_secret.clone()).await;
    //store state in a cookie
    let cookie = Cookie::build(("oauth_state", state_secret.clone()))
        .path("/")
        .max_age(Duration::days(2))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .finish();

    let auth_url = google_oauth.get_authorization_url(&redirect_url, &state_secret);
    println!("🌐 Generated auth URL: {}", auth_url);

    // DEBUG: Test if Redirect::to works
    println!("🔄 Testing redirect creation...");
    let redirect = Redirect::to(&auth_url);
    println!("✅ Redirect object created successfully");
    
    println!("=== REDIRECTING TO GOOGLE ===");
    Ok((jar.add(cookie), redirect))
}


pub async fn google_callback(
    Extension(app_state): Extension<Arc<AppState>>,
    jar: CookieJar,
    Query(query): Query<GoogleAuthQuery>,
) -> Result<impl IntoResponse, HttpError> {
    println!("=== GOOGLE CALLBACK STARTED ===");
    let google_auth = GoogleAuthService::new()
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    //Get state from cookie
    let stored_state = jar.get("oauth_state")
        .map(|cookie| cookie.value().to_string())
        .ok_or_else(|| HttpError::unauthorized("Missing CSRF state cookie".to_string()))?;

    // //addded here 
    // if let Some(state) = &query.state {
    //     google_auth.validate_csrf_state(state)
    //     .await
    //     .map_err(|e| HttpError::unauthorized(e.to_string()))?;
    // } else {
    //     return Err(HttpError::unauthorized("Missing CSRF state parameter".to_string()));
    // } //

    //Validate CSRF state
    if let Some(state) = &query.state {
        if state != &stored_state {
            return Err(HttpError::unauthorized("Invalid CSRF token".to_string()));
        }
    } else {
        return Err(HttpError::unauthorized("Missing CSRF state parameter".to_string()));
    }

    //removing the cookie after use
    let jar = jar.remove(Cookie::named("oauth_state"));

    // let redirect_url = format!("{}/api/auth/google/callback", app_state.env.app_url);
    let redirect_url = "https://verinest.vercel.app/api/oauth/google/callback".to_string();

    //Exchange code for access token
    println!("🔄 Exchanging code for tokens...");
    let (access_token, id_token) = google_auth.exchange_code(&query.code, &redirect_url)
        .await
        .map_err(|e| HttpError::unauthorized(e.to_string()))?;

    //Get user info from Google - you can use either method
     let user_info = if let Some(id_token) = id_token {
        //method 1: Validate ID token directly
        google_auth.validate_id_token(&id_token)
            .await
            .map_err(|e| HttpError::unauthorized(e.to_string()))?
     } else {
        //Method 2: Use access token to get user info
        google_auth.get_user_info_via_access_token(&access_token)
            .await
            .map_err(|e| HttpError::unauthorized(e.to_string()))?
     };

    //Check if user already exists
    let existing_user = app_state.db_client
        .get_user_by_google_id(&user_info.sub)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let user = if let Some(user) = existing_user {
        user
    } else {
        //Create new user
        app_state.db_client.create_oauth_user(user_info.name,
            user_info.email,
            user_info.sub, 
            user_info.picture, 
            100
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
    };

    //Generate JWT token
    let token = token::create_token(
        &user.id.to_string(), 
        app_state.env.jwt_secret.as_bytes(), 
        app_state.env.jwt_maxage
    )
    .map_err(|e| HttpError::server_error(e.to_string()))?;

    //Redirect to Frontend with token

    let redirect_url = format!("{}?token={}", &app_state.env.app_url, token);

    Ok((jar, Redirect::to(&redirect_url)))
}

pub async fn get_google_user(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "user": user.user,
            "is_oauth": user.user.google_id.is_some()
        }
    })))
}

//Deleted after testing
pub async fn test_url_generation() -> Result<impl IntoResponse, HttpError> {
    println!("🧪 Testing URL generation in isolation");
    
    let google_oauth = GoogleAuthService::new()
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    println!("🧪 Testing simple string format...");
    let test_url = format!("https://example.com?test={}", "value");
    println!("   Simple format works: {}", test_url);

    println!("🧪 Testing urlencoding...");
    let encoded = urlencoding::encode("https://verinest.vercel.app/callback");
    println!("   URL encoding works: {}", encoded);

    println!("🧪 Testing get_authorization_url method...");
    let auth_url = google_oauth.get_authorization_url(
        "https://verinest.vercel.app/callback",
        "test_state"
    );

    Ok(Json(serde_json::json!({
        "auth_url": auth_url,
        "status": "success"
    })))
}