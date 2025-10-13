// dtos/verificationdtos.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::{
    usermodel::{VerificationStatus, VerificationType},
    verificationmodels::OtpPurpose,
};

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct OtpRequestDto {
    #[validate(email(message = "Valid email is required"))]
    pub email: String,
    
    pub purpose: OtpPurpose,
}

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct OtpVerificationDto {
    #[validate(email(message = "Valid email is required"))]
    pub email: String,
    
    #[validate(length(min = 6, max = 6, message = "OTP must be 6 digits"))]
    pub otp_code: String,
    
    pub purpose: OtpPurpose,
}

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct NinVerificationDto {
    #[validate(length(min = 11, max = 11, message = "NIN must be 11 digits"))]
    pub nin_number: String,
    
    #[validate(length(min = 3, message = "Nationality is required"))]
    pub nationality: String,

    pub dob: Option<DateTime<Utc>>,
    pub lga: Option<String>,
    pub nearest_landmark: Option<String>,
    
    // Base64 encoded images
    #[validate(length(min = 1, message = "Document image is required"))]
    pub document_url: String,
    
    #[validate(length(min = 1, message = "Selfie image is required"))]
    pub selfie_url: String,
}

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVerificationDto {
    pub verification_type: VerificationType,
    
    #[validate(length(min = 5, message = "Document ID is required"))]
    pub document_id: String,
    
    #[validate(length(min = 3, message = "Nationality is required"))]
    pub nationality: String,

    pub dob: Option<DateTime<Utc>>,
    pub lga: Option<String>,
    pub nearest_landmark: Option<String>,
    
    // Base64 encoded images
    #[validate(length(min = 1, message = "Document image is required"))]
    pub document_url: String,
    
    #[validate(length(min = 1, message = "Selfie image is required"))]
    pub selfie_url: String,
}

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct ReviewVerificationDto {
    pub status: VerificationStatus,
    
    #[validate(length(max = 500, message = "Review notes must be less than 500 characters"))]
    pub review_notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationStatusDto {
    pub verification_status: Option<VerificationStatus>,
    pub is_verified: bool,
    pub pending_verifications: usize,
}

// dtos/verificationdtos.rs (add this struct)
#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteUserVerificationDto {
    pub user: crate::models::usermodel::User,
    pub verification_documents: Vec<VerificationDocument>,
    pub is_fully_verified: bool,
}

impl CompleteUserVerificationDto {
    pub fn from_user_and_documents(user: crate::models::usermodel::User, documents: Vec<VerificationDocument>) -> Self {
        let is_fully_verified = user.verification_status == Some(VerificationStatus::Approved);
        
        Self {
            user,
            verification_documents: documents,
            is_fully_verified,
        }
    }
}