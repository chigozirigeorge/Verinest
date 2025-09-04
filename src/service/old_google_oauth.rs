// use oauth2::{
//     basic::BasicClient,
//     AuthUrl,
//     ClientId,
//     ClientSecret,
//     RedirectUrl,
//     TokenUrl,
//     CsrfToken,
//     Scope,
//     AuthorizationCode,
//     TokenResponse,
// };
// use serde::Deserialize;
// use std::env;
// use thiserror::Error;

// #[derive(Debug, Deserialize)]
// pub struct GoogleUserInfo {
//     pub sub: String,
//     pub email: String,
//     pub name: String,
//     pub given_name: String,
//     pub family_name: String,
//     pub picture: Option<String>,
//     pub email_verified: bool,
//     pub locale: Option<String>,
// }

// #[derive(Debug, Error)]
// pub enum OAuthError {
//     #[error("Environment variable error: {0}")]
//     EnvVar(#[from] env::VarError),
//     #[error("OAuth2 configuration error: {0}")]
//     OAuthConfig(String),
//     #[error("HTTP request error: {0}")]
//     Reqwest(#[from] reqwest::Error),
//     #[error("Token exchange error: {0}")]
//     TokenExchange(String),
//     #[error("User info fetch error: {0}")]
//     UserInfoFetch(String),
// }

// pub struct GoogleOAuthService {
//     client: BasicClient,
// }

// impl GoogleOAuthService {
//     pub fn new() -> Result<Self, OAuthError> {
//         let client_id = env::var("GOOGLE_CLIENT_ID")?;
//         let client_secret = env::var("GOOGLE_CLIENT_SECRET")?;
//         let redirect_url = env::var("GOOGLE_REDIRECT_URL")?;

//         let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
//             .map_err(|e| OAuthError::OAuthConfig(format!("Invalid auth URL: {}", e)))?;
            
//         let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
//             .map_err(|e| OAuthError::OAuthConfig(format!("Invalid token URL: {}", e)))?;
            
//         let redirect_uri = RedirectUrl::new(redirect_url)
//             .map_err(|e| OAuthError::OAuthConfig(format!("Invalid redirect URL: {}", e)))?;

//         let client = BasicClient::new(
//             ClientId::new(client_id),
//             // auth_url,
//             // Some(token_url),
//         )
//         .set_client_secret(Some(ClientSecret::new(client_secret)))
//         .set_auth_uri(auth_url)
//         .set_token_uri(token_url)
//         .set_redirect_uri(redirect_uri);

//         Ok(Self { client })
//     }

//     pub fn get_authorization_url(&self) -> (String, CsrfToken) {
//         let (auth_url, csrf_token) = self.client
//             .authorize_url(CsrfToken::new_random)
//             .add_scope(Scope::new("email".to_string()))
//             .add_scope(Scope::new("profile".to_string()))
//             .add_scope(Scope::new("openid".to_string())) // Add openid scope for Google
//             .url();

//         (auth_url.to_string(), csrf_token)
//     }

//     pub async fn exchange_code(&self, code: String) -> Result<String, OAuthError> {
//         let token_result = self.client
//             .exchange_code(AuthorizationCode::new(code))
//             .request_async(async_http_client)
//             .await
//             .map_err(|e| OAuthError::TokenExchange(format!("Token exchange failed: {}", e)))?;

//         Ok(token_result.access_token().secret().to_string())
//     }

//     pub async fn get_user_info(&self, access_token: &str) -> Result<GoogleUserInfo, OAuthError> {
//         let client = reqwest::Client::new();
//         let response = client
//             .get("https://www.googleapis.com/oauth2/v3/userinfo")
//             .bearer_auth(access_token)
//             .send()
//             .await
//             .map_err(|e| OAuthError::Reqwest(e))?;

//         if !response.status().is_success() {
//             let status = response.status();
//             let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
//             return Err(OAuthError::UserInfoFetch(
//                 format!("Failed to fetch user info: HTTP {} - {}", status, error_text)
//             ));
//         }

//         let user_info: GoogleUserInfo = response.json()
//             .await
//             .map_err(|e| OAuthError::UserInfoFetch(format!("Failed to parse user info: {}", e)))?;
            
//         Ok(user_info)
//     }

//     // Optional: Helper method to exchange code and get user info in one call
//     pub async fn exchange_code_and_get_user_info(&self, code: String) -> Result<GoogleUserInfo, OAuthError> {
//         let access_token = self.exchange_code(code).await?;
//         self.get_user_info(&access_token).await
//     }
// }

// // Optional: Implement Default for convenience
// impl Default for GoogleOAuthService {
//     fn default() -> Self {
//         Self::new().expect("Failed to create GoogleOAuthService")
//     }
// }