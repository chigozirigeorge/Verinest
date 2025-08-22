//1
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow, sqlx::Type, Clone)]
pub struct User {
    pub id: uuid::Uuid, 
    pub name: String,
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: UserRole,
    pub trust_score: i32,
    pub verified: bool,
    pub verification_type: VerificationType,
    pub referral_code: Option<String>, //added this line
     pub referral_count: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_number: Option<String>,
    pub wallet_address: Option<String>,
    pub nationality: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dob: Option<DateTime<Utc>>,
    
    /// Local Government Area (for Nigerian users)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lga: Option<String>,
    
    /// 4-digit transaction PIN (hashed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_pin: Option<i16>,
    
    /// Next of kin contact information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_of_kin: Option<String>,
    
    /// Email verification token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_token: Option<String>,
    
    pub token_expires_at: Option<DateTime<Utc>>,

    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}