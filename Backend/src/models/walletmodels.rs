// models/walletmodels.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "user_tier", rename_all = "lowercase")]
pub enum UserTier {
    Basic,
    Verified,
    Premium,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "transaction_type", rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Transfer,
    JobPayment,
    JobRefund,
    PlatformFee,
    Bonus,
    Referral,
    Penalty,
    ServiceDelivery,
    ServicePayment,
    Refund,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "transaction_status", rename_all = "snake_case")]
pub enum TransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Reversed,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "payment_method", rename_all = "snake_case")]
pub enum PaymentMethod {
    BankTransfer,
    Card,
    Ussd,
    BankCode,
    Qr,
    MobileMoney,
    Bvn,
    NipSlip,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "wallet_status", rename_all = "snake_case")]
pub enum WalletStatus {
    Active,
    Suspended,
    Frozen,
    Closed,
}

//start

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserWallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub wallet_address: String,
    pub wallet_type: Option<String>,
    pub blockchain: Option<String>,
    pub is_verified: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct WalletUpdateRequest {
    pub wallet_address: String,
    pub wallet_type: Option<String>,
    pub blockchain: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WalletVerificationRequest {
    pub signature: String,
    pub message: String,
    pub wallet_address: String,
}
//end

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NairaWallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub balance: i64,
    pub available_balance: i64,
    pub total_deposits: i64,
    pub total_withdrawals: i64,
    pub status: Option<WalletStatus>,
    pub daily_limit: Option<i64>,
    pub monthly_limit: Option<i64>,
    pub is_verified: Option<bool>,
    pub bvn_verified: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub user_id: Uuid,
    pub transaction_type: Option<TransactionType>,
    pub amount: i64, // in kobo
    pub balance_before: i64,
    pub balance_after: i64,
    pub status: Option<TransactionStatus>,
    pub reference: String, // Unique transaction reference
    pub external_reference: Option<String>, // Payment provider reference
    pub payment_method: Option<PaymentMethod>,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub job_id: Option<Uuid>, // For job-related transactions
    pub recipient_wallet_id: Option<Uuid>, // For transfers
    pub fee_amount: Option<i64>, // Transaction fees
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BankAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub account_name: String,
    pub account_number: String,
    pub bank_code: String,
    pub bank_name: String,
    pub is_verified: Option<bool>,
    pub is_primary: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WalletHold {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub job_id: Option<Uuid>,
    pub amount: i64,
    pub reason: String,
    pub status: Option<String>, // active, released, expired
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub released_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PaymentProvider {
    pub id: Uuid,
    pub name: String,
    pub provider_type: String, // flutterwave, paystack, etc
    pub is_active: bool,
    pub config: serde_json::Value,
    pub supported_methods: Vec<PaymentMethod>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TransactionFee {
    pub id: Uuid,
    pub transaction_type: TransactionType,
    pub min_amount: i64,
    pub max_amount: i64,
    pub fee_type: String, // fixed, percentage
    pub fee_value: i64, // in kobo for fixed, basis points for percentage
    pub is_active: bool,
    pub created_at: Option<DateTime<Utc>>,
}


#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WalletLimit {
    pub id: Uuid,
    pub user_tier: UserTier, // basic, verified, premium
    pub transaction_type: TransactionType,
    pub daily_limit: i64,
    pub monthly_limit: i64,
    pub per_transaction_limit: i64,
    pub is_active: bool,
}

// Helper functions for amount conversion
impl NairaWallet {
    pub fn balance_in_naira(&self) -> f64 {
        self.balance as f64 / 100.0
    }

    pub fn available_balance_in_naira(&self) -> f64 {
        self.available_balance as f64 / 100.0
    }

    pub fn total_deposits_in_naira(&self) -> f64 {
        self.total_deposits as f64 / 100.0
    }

    pub fn total_withdrawals_in_naira(&self) -> f64 {
        self.total_withdrawals as f64 / 100.0
    }
}

impl WalletTransaction {
    pub fn amount_in_naira(&self) -> f64 {
        self.amount as f64 / 100.0
    }

    pub fn fee_amount_in_naira(&self) -> f64 {
        self.fee_amount.unwrap_or(0) as f64 / 100.0
    }
}

// Utility functions
pub fn naira_to_kobo(naira: f64) -> i64 {
    (naira * 100.0).round() as i64
}

pub fn kobo_to_naira(kobo: i64) -> f64 {
    kobo as f64 / 100.0
}

pub fn generate_transaction_reference() -> String {
    format!("VRN_{}", uuid::Uuid::new_v4().to_string().replace("-", "").to_uppercase()[..16].to_string())
}