//13
use std::{env, sync::Arc};

use axum::{extract::{Query, Path}, middleware, response::IntoResponse, routing::{get, post, put}, Extension, Json, Router};
use validator::Validate;
use uuid::Uuid;



use crate::{
    db::userdb::UserExt, 
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
        models::usermodel::*, 
        service::referral::generate_referral_link, 
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
     .route("/oauth/google", get(get_google_user))
}



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

// handler/users.rs - Add this new function
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

    // Only allow upgrading to Worker or Employer
    let allowed_roles = vec![UserRole::Worker, UserRole::Employer];
    if !allowed_roles.contains(&body.new_role) {
        return Err(HttpError::bad_request(
            "You can only upgrade to Worker or Employer role"
        ));
    }

    // Check if user is already verified (optional requirement)
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
// Also add a function to get available roles for self-upgrade
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
    }

    // If user is already a Worker, they can become Employer (and vice versa)
    if user.role == UserRole::Worker {
        available_roles.push(RoleInfo {
            role: UserRole::Employer,
            name: "Employer".to_string(),
            description: "Post jobs and hire workers".to_string(),
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
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "current_role": user.role.to_str(),
            "available_roles": available_roles
        }
    })))
}
