//1
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::{subscriptionmodels::SubscriptionTier};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
pub enum UserRole {
    SuperAdmin,
    Admin,
    Moderator,
    Verifier,
    Lawyer,
    Agent,
    Landlord,
    Whistleblower,
    CustomerCare,
    Dev,
    Worker,
    Employer,
    Vendor,
    User
}

impl UserRole {
    pub fn to_str(&self) -> &str {
        match self {
            UserRole::Admin => "admin",
            UserRole::User => "user",
            UserRole::SuperAdmin => "super_admin",
            UserRole::Moderator => "moderator",
            UserRole::Verifier => "verifier",
            UserRole::Lawyer => "lawyer",
            UserRole::Agent => "agent",
            UserRole::Landlord => "landlord",
            UserRole::Whistleblower => "whistleblower",
            UserRole::CustomerCare => "customer_care",
            UserRole::Dev => "dev",
            UserRole::Vendor => "vendor",
            UserRole::Worker => "worker",
            UserRole::Employer => "employer"
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "verification_type", rename_all = "snake_case")]
pub enum VerificationType {
    NationalId,
    DriverLicense,
    Passport
}

impl VerificationType {
    pub fn to_str(&self) -> &str {
        match self {
            VerificationType::NationalId => "national_id",
            VerificationType::DriverLicense => "driver_license",
            VerificationType::Passport => "passport",
        }
    }
}
#[derive(Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "verification_status", rename_all = "snake_case")]
pub enum VerificationStatus {
    Unverified,   ///added here
    Pending,
    Submitted,
    Processing,
    Approved,
    Rejected,
    Expired
}

impl VerificationStatus {
    pub fn to_str(&self) -> &str {
        match self {
            VerificationStatus::Unverified => "unverified",
            VerificationStatus::Pending => "pending",
            VerificationStatus::Submitted => "submitted",
            VerificationStatus::Processing => "processing",
            VerificationStatus::Approved => "approved",
            VerificationStatus::Rejected => "rejected",
            VerificationStatus::Expired => "expired",
        }
    }
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow, sqlx::Type, Clone)]
pub struct User {
    pub id: uuid::Uuid, 
    pub name: String,
    pub username: String,
    pub email: String,
    pub password: Option<String>, 
    pub role: UserRole,
    pub subscription_tier: SubscriptionTier, // Free, Premium
    pub role_change_count: Option<i32>,
    pub trust_score: i32,
    pub verified: bool,
    pub verification_type: VerificationType,
    pub referral_code: Option<String>,
    pub referral_count: Option<i32>,
    pub role_change_reset_at: Option<DateTime<Utc>>,
    
    // OAuth fields
    pub google_id: Option<String>,
    pub avatar_url: Option<String>,
    
    // Wallet field
    pub wallet_address: Option<String>,
    
    // Contact field
    pub phone_number: Option<String>,
    
    // Verification fields
    pub nin_number: Option<String>,
    pub verification_document_id: Option<String>,
    pub facial_verification_id: Option<String>,
    pub nearest_landmark: Option<String>,
    pub verification_status: Option<VerificationStatus>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_number: Option<String>,
    pub nationality: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dob: Option<DateTime<Utc>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lga: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_pin: Option<i32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_pin_hash: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_of_kin: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_token: Option<String>,
    
    pub token_expires_at: Option<DateTime<Utc>>,

    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}



impl User {
    pub fn has_premium_subscription(&self) -> bool {
        self.subscription_tier == SubscriptionTier::Premium
    }

    pub fn get_effective_role(&self) -> UserRole {
        self.role
    }

    pub fn get_monthly_role_changes(self) -> i32 {
        if self.has_premium_subscription() {
            i32::MAX 
        } else {
            5
        }
    }
}