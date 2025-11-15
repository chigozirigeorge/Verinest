//13
use std::{env, sync::Arc};

use axum::{extract::{Query, Path}, middleware, response::IntoResponse, routing::{get, post, put}, Extension, Json, Router};
use chrono::Utc;
use validator::Validate;
use uuid::Uuid;



use crate::{
    db::{userdb::UserExt, naira_walletdb::NairaWalletExt, subscriptiondb::SubscriptionExt}, 
    dtos::userdtos::*, 
    error::{ErrorMessage, HttpError}, 
    handler::{
        google_oauth::get_google_user, 
        // wallet::{
        //     generate_verification_message, get_wallet, 
        //     get_wallet_verification_status, update_wallet, 
        //     verify_wallet
        // }
        }, middleware::{role_check, JWTAuthMiddeware}, 
        models::{usermodel::*, subscriptionmodels::SubscriptionTier}, 
        service::{referral::generate_referral_link, subscription_service::SubscriptionService}, 
        utils::password, AppState};


pub fn users_handler() -> Router {
    Router::new()
        .route(
            "/me", 
            get(get_me)
            .layer(middleware::from_fn(|state, req, next| {
                role_check(state, req, next, vec![UserRole::Admin, UserRole::User, UserRole::Moderator, UserRole::Verifier, UserRole::Lawyer,
                    UserRole::Agent, UserRole::Landlord, UserRole::Whistleblower, UserRole::CustomerCare, UserRole::Dev, UserRole::Worker, UserRole::Employer ])
            }))
    )
    .route(
        "/users", 
        get(get_users)
        .layer(middleware::from_fn(|state, req, next| {
            role_check(state, req, next, vec![UserRole::User, UserRole::Admin])
        }))
    )
    .route("/check-username", get(check_username_availability))
     .route("/avatar", put(update_user_avatar))
    .route("/name", put(update_user_name))
    .route("/role", put(update_user_role))
    .route("/role/upgrade", put(upgrade_user_role)) // Self-upgrade route
    .route("/role/available", get(get_available_roles)) // Get available roles
    .route("/password", put(update_user_password))
    .route(
        "/trust_point", 
        put(update_trust_point)
        .layer(middleware::from_fn(|state, req, next| {
            role_check(state, req, next, vec![UserRole::User, UserRole::Admin])
        }))
    )
    .route(
            "/admin/users", 
            get(get_users_admin)
            .layer(middleware::from_fn(|state, req, next| {
                role_check(state, req, next, vec![UserRole::Admin, UserRole::SuperAdmin])
            }))
        )
        .route(
            "/admin/users/:user_id", 
            get(get_user_admin)
            .layer(middleware::from_fn(|state, req, next| {
                role_check(state, req, next, vec![UserRole::Admin, UserRole::SuperAdmin])
            }))
        )
    .route(
        "/leaderboard", 
        get(get_leaderboard)
    )
    .route(
        "/referral-link", 
        get(get_referral_link)
    )
    .route(
        "/referral-stats", 
        get(get_referral_stats)
    )
    .route(
        "/referral-status", 
        get(check_referral_status)
    )
    // .route("/wallet", put(update_wallet).get(get_wallet))
    // .route("/wallet/verify", post(verify_wallet))
    // .route("/wallet/verification-message", get(generate_verification_message))
    // .route("/wallet/verification-status", get(get_wallet_verification_status))
    //.route("/update_transaction-pin", put(update_transaction_pin))
    .route("/oauth/google", get(get_google_user))
    .route("/transaction-pin/verify", post(verify_transaction_pin))
    .route("/transaction-pin", put(set_transaction_pin))
    .route("/verify-password", post(verify_password))
    .route("/subscription/premium", 
            post(subscribe_premium)
            .get(get_subscription_status)
    )
    .route("/subscription/premium/initiate", post(initiate_premium_payment))
    .route("/subscription/role-change-stats", get(get_role_change_stats))
    .route("/subscription/benefits", get(get_subscription_benefits))
}




// Separate PIN verification endpoint
pub async fn verify_transaction_pin(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<VerifyTransactionPinDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let user_id = user.user.id;

    // Get fresh user data to ensure we have latest PIN
    let current_user = app_state.db_client
        .get_user(Some(user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    // Check if user has set a transaction PIN

    let stored_hash = current_user.transaction_pin_hash
        .as_deref()
        .ok_or_else(|| HttpError::bad_request("Transaction PIN not set. Please set a PIN first."))?;

    // Verify the provided PIN using Argon2 compare
    let pin_ok = crate::utils::password::compare(&body.transaction_pin, Some(stored_hash))
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if !pin_ok {
        return Ok(Json(TransactionPinResponse {
            status: "error".to_string(),
            message: "Invalid transaction PIN".to_string(),
            verified: false,
            expires_at: None,
        }));
    }

    // Mark verification in Redis for short time so frontend can call sensitive endpoints (like signing) without resending PIN
    let expires_at = Utc::now() + chrono::Duration::minutes(5);
    if let Some(redis_arc) = &app_state.db_client.redis_client {
        let key = format!("pin:verified:{}", user.user.id);
        let mut conn = redis_arc.lock().await;
        // SET key with EXPIRE 300 seconds
        let _ : () = redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("EX")
            .arg(300)
            .query_async(&mut *conn)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
    }

    Ok(Json(TransactionPinResponse {
        status: "success".to_string(),
        message: "Transaction PIN verified successfully".to_string(),
        verified: true,
        expires_at: Some(expires_at),
    }))
}

// Update your existing transaction PIN setup endpoint
pub async fn set_transaction_pin(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<SetTransactionPinDto>, // Renamed from UpdateTransactionPinDto for clarity
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let user_id = user.user.id;

    // Validate new PIN format
    let new_pin_clean = body.new_pin.trim();
    if new_pin_clean.len() != 6 || !new_pin_clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(HttpError::bad_request("Transaction PIN must be exactly 6 digits"));
    }

    // Hash the PIN before storing
    let hashed_pin = crate::utils::password::hash(new_pin_clean.to_string())
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // If user already has a PIN, require current PIN or password
    let current_user = app_state.db_client
        .get_user(Some(user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    if let Some(existing_hash) = current_user.transaction_pin_hash.clone() {
        // Require current PIN for PIN changes - verify against stored hash
        if let Some(provided_pin) = &body.current_pin {
            let current_ok = crate::utils::password::compare(provided_pin, Some(&existing_hash))
                .map_err(|e| HttpError::server_error(e.to_string()))?;

            if !current_ok {
                return Err(HttpError::unauthorized("Invalid current transaction PIN"));
            }
        } else {
            return Err(HttpError::bad_request("Current PIN is required to change existing PIN"));
        }
    } else {
        // No existing PIN - require password to set initial PIN
        if let Some(password) = &body.password {
            let password_match = crate::utils::password::compare(
                password, 
                Some(current_user.password.as_deref().unwrap_or(""))
            ).map_err(|e| HttpError::server_error(e.to_string()))?;
            
            if !password_match {
                return Err(HttpError::unauthorized("Invalid account password"));
            }
        } else {
            return Err(HttpError::bad_request("Account password is required to set transaction PIN"));
        }
    }

    // Update the PIN
    let updated_user = app_state.db_client
        .update_transaction_pin_hash(user_id, &hashed_pin)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let filtered_user = FilterUserDto::filter_user(&updated_user);

    Ok(Json(UserResponseDto {
        status: "success".to_string(),
        data: UserData { user: filtered_user },
    }))
}

pub async fn verify_password(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<VerifyPasswordDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let user_id = user.user.id;

    // Get user from database
    let current_user = app_state.db_client
        .get_user(Some(user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    // Verify password
    let password_match = crate::utils::password::compare(
        &body.password, 
        Some(current_user.password.as_deref().unwrap_or(""))
    ).map_err(|e| HttpError::server_error(e.to_string()))?;

    if !password_match {
        return Ok(Json(VerifyPasswordResponse {
            status: "error".to_string(),
            verified: false,
            message: "Invalid password".to_string(),
        }));
    }

    Ok(Json(VerifyPasswordResponse {
        status: "success".to_string(),
        verified: true,
        message: "Password verified successfully".to_string(),
    }))
}

////

// pub async fn update_transaction_pin(
//     Extension(app_state): Extension<Arc<AppState>>,
//     Extension(user): Extension<JWTAuthMiddeware>,
//     Json(body): Json<UpdateTransactionPinDto>,
// ) -> Result<impl IntoResponse, HttpError> {
//     // Validate new_pin length / format (4-6 digits)
//     let new_pin_clean = body.new_pin.trim();
//     if new_pin_clean.len() < 6 || new_pin_clean.len() > 6 || !new_pin_clean.chars().all(|c| c.is_ascii_digit()) {
//         return Err(HttpError::bad_request("new_pin must be 6 digits"));
//     }

//     let user_id = user.user.id;

//     // If user already has a transaction pin, require current_pin match.
//     if let Some(existing_pin) = user.user.transaction_pin {
//         // current_pin must be provided
//         let provided = body.current_pin.as_ref().ok_or_else(|| HttpError::bad_request("current_pin is required"))?;
//         let provided_pin = provided.parse::<i16>().map_err(|_| HttpError::bad_request("Invalid current_pin format"))?;
//         if provided_pin != existing_pin {
//             return Err(HttpError::unauthorized("Invalid current transaction pin"));
//         }
//     } else {
//         // No existing pin — require account password to set pin
//         let provided_password = body.password.as_ref().ok_or_else(|| HttpError::bad_request("password is required to set transaction pin"))?;
//         // verify password against stored hash
//         let stored_user = app_state.db_client
//             .get_user(Some(user_id), None, None, None)
//             .await
//             .map_err(|e| HttpError::server_error(e.to_string()))?
//             .ok_or_else(|| HttpError::not_found("User not found"))?;

//         let pw_match = crate::utils::password::compare(provided_password, Some(stored_user.password.as_deref().unwrap_or("")))
//             .map_err(|e| HttpError::server_error(e.to_string()))?;
//         if !pw_match {
//             return Err(HttpError::unauthorized("Invalid account password"));
//         }
//     }

//     let new_pin_val = new_pin_clean.parse::<i16>().map_err(|_| HttpError::bad_request("Invalid new_pin format"))?;

//     let updated_user = app_state.db_client
//         .update_transaction_pin(user_id, new_pin_val)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?;

//     let filtered_user = FilterUserDto::filter_user(&updated_user);

//     Ok(Json(UserResponseDto {
//         status: "success".to_string(),
//         data: UserData { user: filtered_user },
//     }))
// }



pub async fn get_me(
    Extension(_app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>
) -> Result<impl IntoResponse, HttpError> {

    let filtered_user = FilterUserDto::filter_user(&user.user);

    let response_data = UserResponseDto {
        status: "success".to_string(),
        data: UserData {
            user: filtered_user,
        }
    };

    Ok(Json(response_data))
}

pub async fn get_users(
    Query(query_params): Query<RequestQueryDto>,
    Extension(app_state): Extension<Arc<AppState>>
) -> Result<impl IntoResponse, HttpError> {
    query_params.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let page = query_params.page.unwrap_or(1);
    let limit = query_params.limit.unwrap_or(10);
    
    let users = app_state.db_client
        .get_users(page as u32, limit)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let user_count = app_state.db_client
        .get_user_count()
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response = UserListResponseDto {
        status: "success".to_string(),
        users: FilterUserBoard::filter_users(&users),
        results: user_count,
    };

    Ok(Json(response))
}

pub async fn update_user_name(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<NameUpdateDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
       .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let user = &user.user;

    let user_id = uuid::Uuid::parse_str(&user.id.to_string()).unwrap();

    let result = app_state.db_client.
        update_user_name(user_id.clone(), &body.name)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let filtered_user = FilterUserDto::filter_user(&result);

    let response = UserResponseDto {
        data: UserData {
            user: filtered_user,
        },
        status: "success".to_string(),
    };

    Ok(Json(response))
}

pub async fn update_user_role(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth_user): Extension<JWTAuthMiddeware>,  // Renamed for clarity
    Json(body): Json<RoleUpdateDto>,
) -> Result<impl IntoResponse, HttpError> {
    // Validate input
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Verify requesting user is admin
    if auth_user.user.role != UserRole::Admin {
        return Err(HttpError::unauthorized("Only Admins can update roles"));
    }

    // Prevent self-demotion
    if auth_user.user.id == body.target_user_id && body.role != UserRole::Admin {
        return Err(HttpError::unauthorized("Admins cannot remove their own admin status"));
    }

    // Update target user
    let updated_user = app_state.db_client
        .update_user_role( body.target_user_id, body.role)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(UserResponseDto {
        status: "success".to_string(),
        data: UserData {
            user: FilterUserDto::filter_user(&updated_user),
        },
    }))
}

pub async fn update_user_password(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<UserPasswordUpdateDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
       .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let user = &user.user;

    let user_id = uuid::Uuid::parse_str(&user.id.to_string()).unwrap();

    let result = app_state.db_client
        .get_user(Some(user_id.clone()), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let user = result.ok_or(HttpError::unauthorized(ErrorMessage::InvalidToken.to_string()))?;

    let password_match = password::compare(&body.old_password, Some(user.password.as_deref().unwrap_or("")))
            .map_err(|e| HttpError::server_error(e.to_string()))?;

    if !password_match {
        return Err(HttpError::bad_request("Old password is incorrect".to_string()));
    }

    let hash_password = password::hash(&body.new_password)
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    app_state.db_client
        .update_user_password(user_id.clone(), hash_password)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response = Response {
        message: "Password updated Successfully".to_string(),
        status: "success",
    };

    Ok(Json(response))

}

pub async fn update_trust_point (
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<TrustPointRequestDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let user_id = user.user.id;

    let updated_user = app_state.db_client
        .update_trust_point(user_id, body.score_to_add)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let filtered_user = FilterUserDto::filter_user(&updated_user);

    let response = UserResponseDto {
        status: "success".to_string(),
        data: UserData {
            user: filtered_user,
        }
    };

    Ok(Json(response))

}

pub async fn get_leaderboard(
    Extension(app_state): Extension<Arc<AppState>>,
    Query(query_params): Query<RequestQueryDto>
) -> Result<impl IntoResponse, HttpError> {
    query_params.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let limit = query_params.limit.unwrap_or(100);

    let users = app_state.db_client
        .get_users_by_trustscore(limit as i64)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let leaderboard = FilterUserBoard::filter_users(&users);

    let response = UserListResponseDto {
        status: "success".to_string(),
        users: leaderboard,
        results: limit as i64
    };

    Ok(Json(response))
}

pub async fn get_referral_link(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>
) -> Result<impl IntoResponse, HttpError> {
    
    let user_id = user.user.id;

    let app_url = env::var("APP_URL").expect("APP_URL is expected");

    let user = app_state.db_client
        .get_user(Some(user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or(HttpError::server_error("User not Found"))?;

    let referral_code = user.referral_code
        .ok_or(HttpError::bad_request("No referral code found"))?;

    let referral_link = generate_referral_link(&app_url, &referral_code);

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "referral_code": referral_code,
            "referral_link": referral_link
        }
    })))
}

pub async fn get_referral_stats(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user_id = user.user.id;
    
    let stats = app_state.db_client
        .get_user_referral_stats(user_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "total_referrals": stats.total_referrals,
            "total_points_earned": stats.total_points_earned,
            "successful_referrals": stats.successful_referrals
        }
    })))
}

pub async fn check_referral_status(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user_id = user.user.id;
    
    let referral = app_state.db_client
        .get_referral_by_referee(user_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let referrer_info = if let Some(ref referral) = referral {
        let referrer = app_state.db_client
            .get_user(Some(referral.referrer_id), None, None, None)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or(HttpError::server_error("Referrer not found"))?;
        
        Some(serde_json::json!({
            "referrer_name": referrer.name,
            "referrer_username": referrer.username,
            "points_earned": referral.points_awarded,
            "referred_at": referral.created_at
        }))
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "was_referred": referral.is_some(),
            "referral_info": referrer_info
        }
    })))
}


pub async fn upgrade_user_role(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth_user): Extension<JWTAuthMiddeware>,
    Json(body): Json<UpgradeRoleDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let user_id = auth_user.user.id;

    // Verify the user is upgrading themselves
    if body.target_user_id != user_id {
        return Err(HttpError::unauthorized("You can only upgrade your own role"));
    }

    SubscriptionService::check_role_change_limit(app_state.clone(), user_id).await?;


    // Only allow upgrading to Worker or Employer
    let allowed_roles = vec![UserRole::Worker, UserRole::Employer, UserRole::Vendor];
    if !allowed_roles.contains(&body.new_role) {
        return Err(HttpError::bad_request(
            "You can only upgrade to Worker, Employer or Vendor role"
        ));
    }

    let user = app_state.db_client
        .get_user(Some(user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    // Optional: Require email verification
    if !user.verified {
        return Err(HttpError::bad_request(
            "Please verify your email before upgrading your role"
        ));
    }

    // Optional: Require identity verification for certain roles
    if body.new_role == UserRole::Worker {
        if user.verification_status != Some(VerificationStatus::Approved) {
            return Err(HttpError::bad_request(
                "Identity verification required to become a Worker"
            ));
        }
    }

    // Increment role change count
    let _ = app_state.db_client
        .increment_role_change_count(user_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Update the user role
    let updated_user = app_state.db_client
        .update_user_role(user_id, body.new_role)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let filtered_user = FilterUserDto::filter_user(&updated_user);

    Ok(Json(UserResponseDto {
        status: "success".to_string(),
        data: UserData {
            user: filtered_user,
        },
    }))
}

//////////
pub async fn update_user_avatar(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<AvatarUpdateDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let user_id = user.user.id;

    let updated_user = app_state.db_client
        .update_user_avatar(user_id, body.avatar_url.clone())
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let filtered_user = FilterUserDto::filter_user(&updated_user);

    Ok(Json(UserResponseDto {
        status: "success".to_string(),
        data: UserData {
            user: filtered_user,
        },
    }))
}

pub async fn get_users_admin(
    Extension(app_state): Extension<Arc<AppState>>,
    Query(query_params): Query<RequestQueryDto>,
) -> Result<impl IntoResponse, HttpError> {
    query_params.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let page = query_params.page.unwrap_or(1);
    let limit = query_params.limit.unwrap_or(10);
    
    let users_with_docs = app_state.db_client
        .get_users_with_verification_status(page as u32, limit)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let user_count = app_state.db_client
        .get_user_count()
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let admin_users: Vec<AdminUserDto> = users_with_docs
        .into_iter()
        .map(|(user, _documents)| AdminUserDto::from_user(&user))
        .collect();

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "users": admin_users,
            "total_count": user_count,
            "page": page,
            "limit": limit
        }
    })))
}

pub async fn get_user_admin(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let user_with_docs = app_state.db_client
        .get_user_with_verification_status(user_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    let (user, documents) = user_with_docs;
    let admin_user = AdminUserDto::from_user(&user);

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "user": admin_user,
            "verification_documents": documents
        }
    })))
}

pub async fn get_available_roles(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth_user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user = app_state.db_client
        .get_user(Some(auth_user.user.id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    let mut available_roles = vec![];

    // User can always upgrade to Worker or Employer from basic User role
    if user.role == UserRole::User {
        available_roles.push(RoleInfo {
            role: UserRole::Worker,
            name: "Worker".to_string(),
            description: "Find and apply for jobs".to_string(),
            requires_verification: true,
        });

        available_roles.push(RoleInfo {
            role: UserRole::Employer,
            name: "Employer".to_string(),
            description: "Post jobs and hire workers".to_string(),
            requires_verification: false,
        });

        available_roles.push(RoleInfo {
            role: UserRole::Vendor,
            name: "Vendor".to_string(),
            description: "Post services, sell items  and recieve payments".to_string(),
            requires_verification: false,
        });
    }

    // If user is already a Worker, they can become Employer (and vice versa)
    if user.role == UserRole::Worker {
        available_roles.push(RoleInfo {
            role: UserRole::Employer,
            name: "Employer".to_string(),
            description: "Post jobs and hire workers".to_string(),
            requires_verification: false,
        });

        available_roles.push(RoleInfo {
            role: UserRole::Vendor,
            name: "Vendor".to_string(),
            description: "Post services, sell items  and recieve payments".to_string(),
            requires_verification: false,
        });
    }

    if user.role == UserRole::Employer {
        available_roles.push(RoleInfo {
            role: UserRole::Worker,
            name: "Worker".to_string(),
            description: "Find and apply for jobs".to_string(),
            requires_verification: true,
        });

        available_roles.push(RoleInfo {
            role: UserRole::Vendor,
            name: "Vendor".to_string(),
            description: "Post services, sell items  and recieve payments".to_string(),
            requires_verification: false,
        });
    }

    if user.role == UserRole::Vendor {
        available_roles.push(RoleInfo {
            role: UserRole::Worker,
            name: "Worker".to_string(),
            description: "Find and apply for jobs".to_string(),
            requires_verification: true,
        });

        available_roles.push(RoleInfo {
            role: UserRole::Employer,
            name: "Employer".to_string(),
            description: "Post jobs and hire workers".to_string(),
            requires_verification: false,
        });
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "current_role": user.role.to_str(),
            "available_roles": available_roles
        }
    })))
}


pub async fn subscribe_premium(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth_user): Extension<JWTAuthMiddeware>,
    Json(body): Json<SubscribePremiumDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let user_id = auth_user.user.id;

    let subscription = SubscriptionService::create_premium_subscription(
        app_state.clone(),
        user_id,
        body.payment_reference,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Premium subscription activated successfully! You now have unlimited role changes.",
        "data": {
            "subscription": {
                "id": subscription.id,
                "tier": subscription.tier.to_str(),
                "status": subscription.status,
                "expires_at": subscription.expires_at,
                "benefits": subscription.tier.benefits()
            },
            "user": {
                "subscription_tier": "premium",
                "role_change_limit": "unlimited"
            }
        }
    })))
}

pub async fn initiate_premium_payment(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth_user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user_id = auth_user.user.id;
    let amount = 9000.0; // 9k Naira
    
    // Check if user already has premium
    let user = app_state.db_client
        .get_user(Some(user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;
    
    if user.subscription_tier == SubscriptionTier::Premium {
        return Err(HttpError::bad_request("You already have a premium subscription"));
    }
    
    // Generate payment reference
    let reference = format!("PREMIUM_{}", Uuid::new_v4().to_string()[..8].to_uppercase());
    
    // Create pending transaction
    let wallet = app_state.db_client
        .get_naira_wallet(user_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Wallet not found"))?;
    
    let amount_kobo = (amount * 100.0) as i64;
    
    // Check wallet balance
    if wallet.balance < amount_kobo {
        return Err(HttpError::bad_request(
            format!("Insufficient wallet balance. Required: ₦{:.2}", amount)
        ));
    }
    
    // Create pending transaction
    let _tx = sqlx::query(
        r#"
        INSERT INTO wallet_transactions 
        (wallet_id, user_id, transaction_type, amount, balance_before, balance_after,
        reference, description, status, metadata)
        VALUES ($1, $2, 'subscription_payment', $3, $4, $4, $5, $6, 'pending', $7)
        "#
    )
    .bind(wallet.id)
    .bind(user_id)
    .bind(amount_kobo)
    .bind(wallet.balance)
    .bind(&reference)
    .bind("Premium subscription payment")
    .bind(serde_json::json!({"subscription_tier": "premium", "duration": "1_year"}))
    .execute(&app_state.db_client.pool)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Payment initiated. Confirm with your transaction PIN.",
        "data": {
            "reference": reference,
            "amount": amount,
            "description": "Premium Subscription (1 Year)",
            "requires_pin": true
        }
    })))
}

// Get role change statistics
pub async fn get_role_change_stats(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth_user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let stats = SubscriptionService::get_role_change_stats(
        app_state.clone(), 
        auth_user.user.id
    ).await?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "current_count": stats.current_count,
            "monthly_limit": stats.monthly_limit,
            "remaining_changes": stats.remaining_changes,
            "reset_at": stats.reset_at,
            "has_premium": stats.has_premium,
            "premium_price": 9000.0
        }
    })))
}

// Get subscription benefits
pub async fn get_subscription_benefits(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth_user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user = app_state.db_client
        .get_user(Some(auth_user.user.id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    let current_tier = user.subscription_tier;
    let benefits = current_tier.benefits();

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "current_tier": current_tier.to_str(),
            "benefits": benefits,
            "premium_tier": {
                "price": 9000.0,
                "duration": "1 year",
                "benefits": SubscriptionTier::Premium.benefits()
            }
        }
    })))
}

// Get subscription status
pub async fn get_subscription_status(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth_user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user = app_state.db_client
        .get_user(Some(auth_user.user.id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    // Get active subscription if exists
    let active_subscription = app_state.db_client
        .get_user_subscription(auth_user.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "user_tier": user.subscription_tier.to_str(),
            "has_premium": user.clone().has_premium_subscription(),
            "active_subscription": active_subscription,
            "role_change_stats": {
                "current_count": user.role_change_count.clone().unwrap_or(0),
                "monthly_limit": user.clone().get_monthly_role_changes(),
                "remaining": user.clone().get_monthly_role_changes() - user.role_change_count.unwrap_or(0)
            }
        }
    })))
}

pub async fn check_username_availability(
    Extension(app_state): Extension<Arc<AppState>>,
    Query(query_params): Query<CheckUsernameQuery>
) -> Result<impl IntoResponse, HttpError> {

    let username = query_params.username.trim().to_lowercase();

    if username.len() < 3 {
        return Ok(Json(UsernameCheckResponse {
            available: false,
            message: "Username must be at least 3 characters long".to_string(),
        }));
    }

    if username.len() > 30 {
        return Ok(Json(UsernameCheckResponse {
            available: false,
            message: "Username must not exceed 30 characters".to_string(),
        }));
    }
    
    // Check if username contains only valid characters
    let valid_username = regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    if !valid_username.is_match(&username) {
        return Ok(Json(UsernameCheckResponse {
            available: false,
            message: "Username can only contain letters, numbers, underscores and hyphens".to_string(),
        }));
    }

     // Check database for existing username
    let existing_user = app_state.db_client
        .get_user(None, Some(&username), None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    if existing_user.is_some() {
        return Ok(Json(UsernameCheckResponse {
            available: false,
            message: "Username is already taken".to_string(),
        }));
    }
    
    Ok(Json(UsernameCheckResponse {
        available: true,
        message: "Username is available".to_string(),
    }))
}
