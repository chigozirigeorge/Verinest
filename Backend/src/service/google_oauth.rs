use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc, time::{Duration, Instant}};
use thiserror::Error;
use reqwest;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: String,
    pub name: String,
    pub given_name: String,
    pub family_name: String,
    pub picture: Option<String>,
    pub email_verified: bool,
    pub locale: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GoogleClaims {
    iss: String,
    aud: String,
    exp: i64,
    iat: i64,
    sub: String,
    email: String,
    email_verified: bool,
    name: String,
    given_name: String,
    family_name: String,
    picture: Option<String>,
    locale: Option<String>,
}

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] env::VarError),
    #[error("HTTP request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JWT validation error: {0}")]
    JwtValidation(String),
    #[error("Token exchange error: {0}")]
    TokenExchange(String),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CSRF validation error: {0}")] // Add this
    CsrfValidation(String),
}

#[derive(Debug, Deserialize)]
struct GoogleCertsResponse {
    keys: Vec<GooglePublicKey>,
}

#[derive(Debug, Deserialize, Clone)]
struct GooglePublicKey {
    kid: String,
    kty: String,
    alg: String,
    r#use: String,
    n: String,
    e: String,
}


pub struct GoogleAuthService {
    client_id: String,
    client_secret: String,
    cached_public_keys: Arc<Mutex<Option<(HashMap<String, GooglePublicKey>, Instant)>>>,
    csrf_states: Arc<Mutex<HashMap<String, Instant>>>, // Store CSRF tokens with expiration
}

impl GoogleAuthService {
    pub fn new() -> Result<Self, OAuthError> {
        println!("🔄 Initializing GoogleAuthService...");
        
        let client_id = env::var("GOOGLE_CLIENT_ID")
            .map_err(|e| {
                println!("❌ GOOGLE_CLIENT_ID error: {}", e);
                OAuthError::EnvVar(e)
            })?;
        println!("✅ GOOGLE_CLIENT_ID loaded");
        
        let client_secret = env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|e| {
                println!("❌ GOOGLE_CLIENT_SECRET error: {}", e);
                OAuthError::EnvVar(e)
            })?;
        println!("✅ GOOGLE_CLIENT_SECRET loaded");
        
        println!("🎉 GoogleAuthService initialized successfully");
        
        Ok(Self { 
            client_id,
            client_secret,
            cached_public_keys: Arc::new(Mutex::new(None)),
            csrf_states: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    //added these 
    // Store CSRF state with expiration (5 minutes)
   pub async fn store_csrf_state(&self, state: String) {
    println!("💾 Starting CSRF state storage...");
    let mut states = self.csrf_states.lock().await;
    println!("💾 Acquired lock, inserting state...");
    states.insert(state, Instant::now());
    println!("💾 State stored successfully");
    
    // Only cleanup occasionally, not on every insert
    if states.len() % 10 == 0 {  // Cleanup every 10th state storage
        println!("💾 Running periodic cleanup...");
        self.cleanup_expired_csrf_states().await;
    } else {
        println!("💾 Skipping cleanup this time");
    }
   }

    // Validate and remove CSRF state
    pub async fn validate_csrf_state(&self, state: &str) -> Result<(), OAuthError> {
        let mut states = self.csrf_states.lock().await;
        
        if let Some(created_at) = states.remove(state) {
            if created_at.elapsed() < Duration::from_secs(300) { // 5 minutes
                Ok(())
            } else {
                Err(OAuthError::CsrfValidation("CSRF token expired".to_string()))
            }
        } else {
            Err(OAuthError::CsrfValidation("Invalid CSRF token".to_string()))
        }
    }

   // Clean up expired CSRF states periodically
pub async fn cleanup_expired_csrf_states(&self) {
    println!("🧹 Starting CSRF cleanup...");
    
    let mut states = match self.csrf_states.try_lock() {
        Ok(lock) => lock,
        Err(_) => {
            println!("⚠️  Couldn't acquire lock for cleanup (already locked)");
            return;
        }
    };
    
    println!("🧹 Acquired lock for cleanup");
    
    let before_count = states.len();
    println!("🧹 States before cleanup: {}", before_count);
    
    // Use a simpler approach for cleanup
    let now = Instant::now();
    states.retain(|_, created_at| {
        let elapsed = now.duration_since(*created_at);
        let should_retain = elapsed < Duration::from_secs(300);
        if !should_retain {
            println!("   - Removing expired state: {:?} old", elapsed);
        }
        should_retain
    });
    
    let after_count = states.len();
    println!("🧹 Cleanup completed: {} -> {} states", before_count, after_count);
  }
    //stopped here

    //Get Google's public keys for JWT verification
    async fn get_google_public_keys(&self) -> Result<HashMap<String, GooglePublicKey>, OAuthError> {
        let client = reqwest::Client::new();
        let response = client
           .get("https://www.googleapis.com/oauth2/v3/certs")
           .send()
           .await?;

        if !response.status().is_success() {
            return Err(OAuthError::JwtValidation(format!("Failed to fetch Google public keys: HTTP {}", response.status())));
        } 

        let certs: GoogleCertsResponse = response.json().await?;

        let mut public_key = HashMap::new();
        for key in certs.keys {
            public_key.insert(key.kid.clone(), key);
        }

        Ok(public_key)
    }


    //Validate a Google ID token using Google's public keys
    pub async fn validate_id_token(&self, id_token: &str) -> Result<GoogleUserInfo, OAuthError> {
        //We get Google's public keys
        let public_keys = self.get_google_public_keys().await?;

        //Decode the token header to get the key ID 
        let header = jsonwebtoken::decode_header(id_token)
            .map_err(|e| OAuthError::JwtValidation(format!("Invalid token header: {}", e)))?;

        let kid = header.kid.ok_or_else(|| 
            OAuthError::JwtValidation("Missing key ID in token header".to_string()))?;

        //Finding the appropriate public key
        let public_key = public_keys.get(&kid).ok_or_else(|| 
            OAuthError::JwtValidation(format!("No public key found for Key ID: {}", kid)))?;

        //setting up the validation
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.client_id]);
        validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);

        //Decoding and Validating the token
        let decoding_key = DecodingKey::from_rsa_components(&public_key.n, &public_key.e)
            .map_err(|e| OAuthError::JwtValidation(format!("Failed to create decoding key: {}", e)))?;
        let token_data = decode::<GoogleClaims>(
           id_token,
           &decoding_key,
           &validation, 
        )
        .map_err(|e| OAuthError::JwtValidation(format!("Token validation failed: {}", e)))?;

        
        //Converting token to GoogleUserInfo
        let claims = token_data.claims;
        let user_info = GoogleUserInfo {
            sub: claims.sub,
            email: claims.email,
            name: claims.name,
            given_name: claims.given_name,
            family_name: claims.family_name,
            picture: claims.picture,
            email_verified: claims.email_verified,
            locale: claims.locale,
        };

        Ok(user_info)
    }


    //Get user info from Google's userinfo endpoint using access token
    pub async fn get_user_info_via_access_token(&self, access_token: &str) -> Result<GoogleUserInfo, OAuthError> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://www.googleapis.com/oauth2/v3/userinfo")
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(OAuthError::JwtValidation(format!("Failed to fetch user info: {} - {}", status, error_text)));
        }

        let user_info = response.json().await?;
        Ok(user_info)
    }


    //Generate the authorization URL for the Frontend
pub fn get_authorization_url(&self, redirect_url: &str, state: &str) -> String {
    println!("🔧 STEP 1: Starting get_authorization_url");
    println!("   Client ID: {}", self.client_id);
    println!("   Redirect URL: {}", redirect_url);
    println!("   State: {}", state);

    // Test if urlencoding is causing the hang
    println!("🔧 STEP 2: Testing urlencoding...");
    let encoded_redirect = urlencoding::encode(redirect_url);
    println!("   Encoded redirect: {}", encoded_redirect);
    
    let encoded_state = urlencoding::encode(state);
    println!("   Encoded state: {}", encoded_state);

    println!("🔧 STEP 3: Building format string...");
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
         client_id={}&\
         response_type=code&\
         scope=openid%20email%20profile&\
         redirect_uri={}&\
         state={}&\
         access_type=offline",
        self.client_id,
        encoded_redirect,
        encoded_state
    );

    println!("🔧 STEP 4: URL generated successfully");
    println!("   Final URL: {}", url);
    
    url
    }


    //Exchanging authorization codes for tokens
    pub async fn exchange_code(&self, code:&str, redirect_url: &str) -> Result<(String, Option<String>), OAuthError> {
        let client = reqwest::Client::new();

        let params = [
            ("code", code),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("redirect_uri", redirect_url),
            ("grant_type", "authorization_code")
        ];

        let response = client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

            return Err(OAuthError::JwtValidation(format!("Token exchange failed: HTTP {} - {}", status, error_text)));
        }

        let token_response: Value = response.json().await?;

        let access_token = token_response["access_token"]
            .as_str()
            .ok_or_else(|| OAuthError::TokenExchange("Access token missing from response".to_string()))?
            .to_string();

        let id_token = token_response["id_token"]
            .as_str()
            .map(|s| s.to_string());

        Ok((access_token, id_token))
    }
}