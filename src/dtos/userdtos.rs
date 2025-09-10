//3
use core::str;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;
use serde_json;

use crate::models::usermodel::{User, UserRole};

#[derive(Validate, Debug, Default, Clone, Serialize, Deserialize)]
pub struct RegisterUserWithReferralDto {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,

     #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,

    #[validate(
        length(min = 1, message = "Email is required"),
        email(message = "Email is invalid")
    )]
    pub email: String,
    #[validate(
        length(min = 1, message = "Password is required"),
        length(min = 6, message = "Password must be at least 6 characters")
    )]
    pub password: String,

    #[validate(
        length(min = 1, message = "Confirm Password is required"),
        must_match(other = "password", message="passwords do not match")
    )]
    #[serde(rename = "passwordConfirm")]
    pub password_confirm: String,

    pub referral_code: Option<String>, //added
}

#[derive(Validate, Debug, Default, Clone, Serialize, Deserialize)]
pub struct LoginUserDto {
    #[validate(length(min = 1, message = "Email is required"), email(message = "Email is invalid"))]
    pub email: String,
    #[validate(
        length(min = 1, message = "Password is required"),
        length(min = 6, message = "Password must be at least 6 characters")
    )]
    pub password: String,
}

#[derive(Serialize, Deserialize, Validate)]
pub struct RequestQueryDto {
    #[validate(range(min = 1))]
    pub page: Option<usize>,
    #[validate(range(min = 1, max = 50))]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilterUserDto {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub trust_score: i32,
    pub dob: DateTime<Utc>,
    pub nationality: Option<String>,
    pub lga: String,
    pub verified: bool,
    pub wallet_address: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

impl FilterUserDto {
    pub fn filter_user(user: &User) -> Self {
        FilterUserDto {
            id: user.id.to_string(),
            name: user.name.to_owned(),
            email: user.email.to_owned(),
            trust_score: user.trust_score,
            verified: user.verified,
            nationality: user.nationality.clone(),
            lga: user.lga.clone().unwrap_or_default(),
            dob: user.dob.unwrap_or_else(|| chrono::Utc::now()),
            wallet_address: user.wallet_address.clone(),
            role: user.role.to_str().to_string(),
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }

    pub fn filter_users(user: &[User]) -> Vec<FilterUserDto> {
        user.iter().map(FilterUserDto::filter_user).collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    pub user: FilterUserDto,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponseDto {
    pub status: String,
    pub data: UserData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserListResponseDto {
    pub status: String,
    pub users: Vec<FilterUserBoard>, //Added something here
    pub results: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLoginResponseDto {
    pub status: String,
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub status: &'static str,
    pub message: String,
}

#[derive(Validate, Debug, Default, Clone, Serialize, Deserialize)]
pub struct NameUpdateDto {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RoleUpdateDto {
    #[validate(custom = "validate_user_role")]
    pub role: UserRole,
    pub target_user_id: Uuid, 
}

fn validate_user_role(role: &UserRole) -> Result<(), validator::ValidationError> {
    match role {
        UserRole::Admin | UserRole::User => Ok(()),
        _ => Err(validator::ValidationError::new("invalid_role")),
    }
}

#[derive(Debug, Validate, Default, Clone, Serialize, Deserialize)]
pub struct UserPasswordUpdateDto {
    #[validate(
        length(min = 1, message = "New password is required."),
        length(min = 6, message = "new password must be at least 6 characters")
    )]
    pub new_password: String,

    #[validate(
        length(min = 1, message = "New password confirm is required."),
        length(min = 6, message = "new password confirm must be at least 6 characters"),
        must_match(other = "new_password", message="new passwords do not match")
    )]
    pub new_password_confirm: String,

    #[validate(
        length(min = 1, message = "Old password is required."),
        length(min = 6, message = "Old password must be at least 6 characters")
    )]
    pub old_password: String,
}

#[derive(Serialize, Deserialize, Validate)]
pub struct VerifyEmailQueryDto {
    #[validate(length(min = 1, message = "Token is required."),)]
    pub token: String,
}

#[derive(Deserialize, Serialize, Validate, Debug, Clone)]
pub struct ForgotPasswordRequestDto {
    #[validate(length(min = 1, message = "Email is required"), email(message = "Email is invalid"))]
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct ResetPasswordRequestDto {
    #[validate(length(min = 1, message = "Token is required."),)]
    pub token: String,

    #[validate(
        length(min = 1, message = "New password is required."),
        length(min = 6, message = "new password must be at least 6 characters")
    )]
    pub new_password: String,

    #[validate(
        length(min = 1, message = "New password confirm is required."),
        length(min = 6, message = "new password confirm must be at least 6 characters"),
        must_match(other = "new_password", message="new passwords do not match")
    )]
    pub new_password_confirm: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct UpdatePointRequestDto {
    pub user_id: Uuid,
    pub points: i32,
    pub category: String,
    pub reason: Option<serde_json::Value>
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct TrustPointRequestDto {
    pub user_id: Uuid,
    pub score_to_add: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LeaderboardQueryDto {
    pub status: String,
    pub user: Vec<FilterUserDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilterUserBoard {
    pub id: String,
    pub username: String,
    pub trust_score: i32,
    pub wallet_address: Option<String>,
}

impl FilterUserBoard {
    pub fn filter_user(user: &User) -> Self {
        FilterUserBoard { 
            id: user.id.to_string(), 
            username: user.username.to_owned(),
            trust_score: user.trust_score,
            wallet_address: user.wallet_address.clone()
        }
    }

    pub fn filter_users(user: &[User]) -> Vec<FilterUserBoard> {
        user.iter().map(FilterUserBoard::filter_user).collect()
    }
}
