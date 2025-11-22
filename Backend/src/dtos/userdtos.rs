//3
use core::str;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use uuid::Uuid;
use serde_json;
use std::borrow::Cow;


use crate::models::{subscriptionmodels::SubscriptionTier, usermodel::*};

#[derive(Validate, Debug, Default, Clone, Serialize, Deserialize)]
pub struct UpdateUserProfileDto {
    #[validate(
        length(min = 10, max = 20, message = "Phone number must be between 10-20 characters")
    )]
    pub phone_number: Option<String>,

    #[validate(
        length(min = 2, max = 100, message = "LGA must be between 2-100 characters")
    )]
    pub lga: Option<String>,

    #[validate(
        length(min = 2, max = 255, message = "Nearest landmark must be between 2-255 characters")
    )]
    pub nearest_landmark: Option<String>,
}

// Custom validation for phone numbers
impl UpdateUserProfileDto {
    pub fn validate_phone_number(&self) -> Result<(), ValidationError> {
        if let Some(phone) = &self.phone_number {
            // Basic phone number validation - supports international formats
            let phone_regex = regex::Regex::new(r"^(\+?[0-9]{1,3}[- ]?)?[0-9]{3}[- ]?[0-9]{3}[- ]?[0-9]{4}$")
                .map_err(|_| ValidationError::new("Invalid phone regex"))?;
            
            if !phone_regex.is_match(phone) {
                let mut error = ValidationError::new("invalid_phone");
                error.message = Some(Cow::from("Phone number must be in a valid format (e.g., +1234567890 or 123-456-7890)"));
                return Err(error);
            }
        }
        Ok(())
    }
}

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
pub struct ResendVerificationEmailDto {
    #[validate(
        length(min = 1, message = "Email is required"),
        email(message = "Email is invalid")
    )]
    pub email: String,
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
    pub username: String,
    pub role: String,
    pub trust_score: i32,
    pub dob: DateTime<Utc>,
    pub nationality: Option<String>,
    pub lga: String,
    pub email_verified: bool,
    pub document_verified: bool,
    pub subscription_tier: SubscriptionTier,
    pub role_change_count: Option<i32>,
    pub referral_code: Option<String>,
    pub referral_count: Option<i32>,
    pub role_change_reset_at: DateTime<Utc>,
    pub nearest_landmark: Option<String>,
    pub transaction_pin_hash: Option<String>,
    pub transaction_pin: Option<i32>,
    pub verification_status: Option<String>,
    pub wallet_address: Option<String>,
    pub avatar_url: Option<String>,
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
            username: user.username.to_owned(),
            email: user.email.to_owned(),
            trust_score: user.trust_score,
            email_verified: user.verified,
            document_verified: user.verification_status == Some(VerificationStatus::Approved),
            verification_status: user.verification_status.map(|s| s.to_str().to_string()),
            nationality: user.nationality.clone(),
            lga: user.lga.clone().unwrap_or_default(),
            dob: user.dob.unwrap_or_else(|| chrono::Utc::now()),
            subscription_tier: user.subscription_tier.clone(),
            referral_code: user.referral_code.clone(),
            nearest_landmark: user.nearest_landmark.clone(),
            referral_count: user.referral_count,
            role_change_count: user.role_change_count,
            role_change_reset_at: user.role_change_reset_at.unwrap_or_else(|| chrono::Utc::now()),
            transaction_pin_hash: user.transaction_pin_hash.clone(),
            transaction_pin: user.transaction_pin.clone(),
            wallet_address: user.wallet_address.clone(),
            avatar_url: user.avatar_url.clone(),
            role: user.role.to_str().to_string(),
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
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
        UserRole::Admin |
        UserRole::User |
        UserRole::Moderator |
        UserRole::Verifier |
        UserRole::Lawyer |
        UserRole::Agent |
        UserRole::Landlord |
        UserRole::Whistleblower |
        UserRole::CustomerCare |
        UserRole::Dev |
        UserRole::Worker |
        UserRole::Vendor |
        UserRole::Employer => Ok(()),
        
        UserRole::SuperAdmin => {
            let mut error = ValidationError::new("invalid_role");
            error.message = Some("SuperAdmin role cannot be assigned manually".into());
            Err(error)
        }
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

// dtos/userdtos.rs - Add these new DTOs
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpgradeRoleDto {
    pub target_user_id: Uuid,
    
    #[validate(custom = "validate_upgrade_role")]
    pub new_role: UserRole,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleInfo {
    pub role: UserRole,
    pub name: String,
    pub description: String,
    pub requires_verification: bool,
}

// Validation function for self-upgrade roles
fn validate_upgrade_role(role: &UserRole) -> Result<(), ValidationError> {
    match role {
        UserRole::Worker | UserRole::Employer | UserRole::Vendor => Ok(()),
        _ => {
            let mut err = ValidationError::new("invalid_upgrade_role");
            err.add_param(Cow::from("expected"), &"Worker, Employer or Vendor");
            Err(err)
        }
    }
}


#[derive(Validate, Debug, Default, Clone, Serialize, Deserialize)]
pub struct AvatarUpdateDto {
    #[validate(url(message = "Avatar URL must be a valid URL"))]
    pub avatar_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminUserResponseDto {
    pub status: String,
    pub data: AdminUserData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminUserData {
    pub user: AdminUserDto,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminUserDto {
    pub id: String,
    pub name: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub trust_score: i32,
    pub email_verified: bool,
    pub document_verified: bool,
    pub verification_status: Option<String>,
    pub verification_type: Option<String>,
    pub referral_code: Option<String>,
    pub referral_count: Option<i32>,
    pub google_id: Option<String>,
    pub avatar_url: Option<String>,
    pub wallet_address: Option<String>,
    pub nin_number: Option<String>,
    pub verification_number: Option<String>,
    pub nationality: Option<String>,
    pub dob: Option<DateTime<Utc>>,
    pub lga: Option<String>,
    pub nearest_landmark: Option<String>,
    pub next_of_kin: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

impl AdminUserDto {
    pub fn from_user(user: &User) -> Self {
        Self {
            id: user.id.to_string(),
            name: user.name.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            role: user.role.to_str().to_string(),
            trust_score: user.trust_score,
            email_verified: user.verified,
            document_verified: user.verification_status == Some(VerificationStatus::Approved),
            verification_status: user.verification_status.map(|s| s.to_str().to_string()),
            verification_type: Some(user.verification_type.to_str().to_string()),
            referral_code: user.referral_code.clone(),
            referral_count: user.referral_count,
            google_id: user.google_id.clone(),
            avatar_url: user.avatar_url.clone(),
            wallet_address: user.wallet_address.clone(),
            nin_number: user.nin_number.clone(),
            verification_number: user.verification_number.clone(),
            nationality: user.nationality.clone(),
            dob: user.dob,
            lga: user.lga.clone(),
            nearest_landmark: user.nearest_landmark.clone(),
            next_of_kin: user.next_of_kin.clone(),
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CheckUsernameQuery {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct UsernameCheckResponse {
    pub available: bool,
    pub message: String,
}

#[derive(Debug, Validate, Deserialize)]
pub struct VerifyTransactionPinDto {
    #[validate(length(equal = 6, message = "Transaction PIN must be 6 digits"))]
    pub transaction_pin: String,
}

#[derive(Debug, Serialize)]
pub struct TransactionPinResponse {
    pub status: String,
    pub message: String,
    pub verified: bool,
    pub expires_at: Option<DateTime<Utc>>, 
}

#[derive(Debug, serde::Deserialize, Validate)]
pub struct SetTransactionPinDto {
    pub current_pin: Option<String>,  // Required when changing existing PIN
    pub password: Option<String>,     // Required when setting initial PIN
    #[validate(length(equal = 6, message = "Transaction PIN must be 6 digits"))]
    pub new_pin: String,
}

#[derive(Debug, Validate, Deserialize)]
pub struct VerifyPasswordDto {
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyPasswordResponse {
    pub status: String,
    pub verified: bool,
    pub message: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SubscribePremiumDto {
    #[validate(length(min = 1))]
    pub payment_reference: String,
}