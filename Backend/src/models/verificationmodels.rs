// models/verificationmodels.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;
use validator::Validate;

use crate::models::usermodel::{VerificationStatus, VerificationType};

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct NinVerificationRequest {
    #[validate(length(min = 11, max = 11, message = "NIN must be 11 digits"))]
    pub nin_number: String,
    
    pub verification_type: VerificationType,
    
    #[validate(length(min = 3, message = "Nationality is required"))]
    pub nationality: String,

    pub dob: Option<DateTime<Utc>>,
    pub lga: Option<String>,
    pub nearest_landmark: Option<String>,
    
    pub document_image: String,
    
    pub selfie_image: String,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct DocumentVerificationRequest {
    pub verification_type: VerificationType,
    
    #[validate(length(min = 5, message = "Document ID is required"))]
    pub document_id: String,
    
    #[validate(length(min = 3, message = "Nationality is required"))]
    pub nationality: String,

    pub dob: Option<DateTime<Utc>>,
    pub lga: Option<String>,
    pub nearest_landmark: Option<String>,
    
    pub document_image: String,

    pub selfie_image: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerificationDocument {
    pub id: Uuid,
    pub user_id: Uuid,
    pub document_type: VerificationType,
    pub document_id: String,
    pub document_url: String,
    pub selfie_url: String,
    pub status: Option<VerificationStatus>,
    pub reviewed_by: Option<Uuid>,
    pub review_notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerificationResponse {
    pub status: VerificationStatus,
    pub message: String,
    pub next_steps: Option<Vec<String>>,
    pub estimated_completion_time: Option<i32>, // hours
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct OtpRequest {
    #[validate(email(message = "Valid email is required"))]
    pub email: String,
    pub purpose: OtpPurpose,
}

// Remove the manual Encode implementation - SQLx derives it automatically
#[derive(Debug, Deserialize, Serialize, Clone, sqlx::Type)]
#[sqlx(type_name = "otp_purpose", rename_all = "snake_case")]
pub enum OtpPurpose {
    AccountVerification,
    PasswordReset,
    Transaction,
    VerificationUpdate,
    SensitiveAction,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct OtpVerificationRequest {
    #[validate(email(message = "Valid email is required"))]
    pub email: String,
    
    #[validate(length(min = 6, max = 6, message = "OTP must be 6 digits"))]
    pub otp_code: String,
    
    pub purpose: OtpPurpose,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OtpRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub otp_code: String,
    pub purpose: OtpPurpose,
    pub expires_at: Option<DateTime<Utc>>,
    pub used: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FacialVerificationRequest {
    pub facial_data: String,  // Base64 encoded image
    pub verification_document_id: Uuid,
}