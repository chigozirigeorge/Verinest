// dtos/naira_wallet_dtos.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::models::walletmodels::*;

// Wallet DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletResponseDto {
    pub id: Uuid,
    pub balance: f64, // In Naira
    pub available_balance: f64,
    pub total_deposits: f64,
    pub total_withdrawals: f64,
    pub status: WalletStatus,
    pub is_verified: bool,
    pub bvn_verified: bool,
    pub daily_limit: f64,
    pub monthly_limit: f64,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct DepositRequestDto {
    #[validate(range(min = 100.0, max = 10000000.0, message = "Amount must be between ₦100 and ₦10,000,000"))]
    pub amount: f64,
    
    pub payment_method: PaymentMethod,
    
    #[validate(length(min = 1, message = "Description is required"))]
    pub description: String,
    
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct WithdrawalRequestDto {
    #[validate(range(min = 100.0, max = 5000000.0, message = "Amount must be between ₦100 and ₦5,000,000"))]
    pub amount: f64,
    
    pub bank_account_id: Uuid,
    
    #[validate(length(min = 1, message = "Description is required"))]
    pub description: String,
    
    pub metadata: Option<serde_json::Value>,
    // Security fields
    #[serde(default)]
    pub transaction_pin: Option<String>,
    #[serde(default)]
    pub email_otp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct TransferRequestDto {
    #[validate(range(min = 10.0, max = 1000000.0, message = "Amount must be between ₦10 and ₦1,000,000"))]
    pub amount: f64,
    
    #[validate(length(min = 1, message = "Recipient is required"))]
    pub recipient_identifier: String, // Email, username, or phone
    
    #[validate(length(min = 1, max = 200, message = "Description must be between 1 and 200 characters"))]
    pub description: String,
    // Security fields
    // #[serde(default)]
    // pub transaction_pin: Option<String>,
    // #[serde(default)]
    // pub email_otp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionResponseDto {
    pub id: Uuid,
    pub transaction_type: TransactionType,
    pub amount: f64,
    pub fee_amount: f64,
    pub balance_before: f64,
    pub balance_after: f64,
    pub status: TransactionStatus,
    pub reference: String,
    pub external_reference: Option<String>,
    pub payment_method: Option<PaymentMethod>,
    pub description: String,
    pub recipient: Option<RecipientInfoDto>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecipientInfoDto {
    pub name: String,
    pub email: String,
    pub username: String,
}

fn validate_account_number(account_number: &str) -> Result<(), validator::ValidationError> {
    if account_number.chars().all(|c| c.is_ascii_digit()) && account_number.len() == 10 {
        Ok(())
    } else {
        Err(validator::ValidationError::new("account_number must be 10 digits"))
    }
}

// Bank Account DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct AddBankAccountDto {
    #[validate(length(min = 1, max = 100, message = "Account name is required"))]
    pub account_name: String,
    
    #[validate(
        length(min = 10, max = 10, message = "Account number must be 10 digits"),
        custom = "validate_account_number"
    )]
    pub account_number: String,
    
    #[validate(length(min = 3, max = 3, message = "Bank code must be 3 digits"))]
    pub bank_code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BankAccountResponseDto {
    pub id: Uuid,
    pub account_name: String,
    pub account_number: String,
    pub bank_code: String,
    pub bank_name: String,
    pub is_verified: bool,
    pub is_primary: bool,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BankVerificationDto {
    pub account_name: String,
    pub account_number: String,
    pub bank_name: String,
    pub bank_code: String,
}

// Transaction History DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct TransactionHistoryQueryDto {
    pub transaction_type: Option<TransactionType>,
    pub status: Option<TransactionStatus>,
    
    #[validate(range(min = 1, max = 100, message = "Limit must be between 1 and 100"))]
    pub limit: Option<i64>,
    
    #[validate(range(min = 0, message = "Offset must be non-negative"))]
    pub offset: Option<i64>,
    
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletSummaryDto {
    pub balance: f64,
    pub available_balance: f64,
    pub total_deposits: f64,
    pub total_withdrawals: f64,
    pub pending_transactions: i64,
    pub active_holds: f64,
    pub daily_spent: f64,
    pub monthly_spent: f64,
    pub daily_limit: f64,
    pub monthly_limit: f64,
}

// Payment Initialization DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentInitializationDto {
    pub payment_url: String,
    pub access_code: String,
    pub reference: String,
    pub amount: f64,
    pub fee: f64,
}

// Webhook DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentWebhookDto {
    pub event: String,
    pub data: PaymentWebhookDataDto,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentWebhookDataDto {
    pub reference: String,
    pub amount: i64,
    pub status: String,
    pub gateway_response: String,
    pub paid_at: Option<String>,
    pub created_at: Option<String>,
    pub channel: Option<String>,
    pub currency: String,
    pub ip_address: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// Response wrappers
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletApiResponse<T> {
    pub status: String,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedTransactionResponse {
    pub status: String,
    pub data: Vec<TransactionResponseDto>,
    pub pagination: PaginationMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationMetadata {
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

impl<T> WalletApiResponse<T> {
    pub fn success(message: &str, data: T) -> Self {
        Self {
            status: "success".to_string(),
            message: message.to_string(),
            data: Some(data),
        }
    }

    pub fn error(message: &str) -> WalletApiResponse<()> {
        WalletApiResponse {
            status: "error".to_string(),
            message: message.to_string(),
            data: None,
        }
    }
}

// Conversion helpers
impl From<NairaWallet> for WalletResponseDto {
    fn from(wallet: NairaWallet) -> Self {
        Self {
            id: wallet.id,
            balance: wallet.balance_in_naira(),
            available_balance: wallet.available_balance_in_naira(),
            total_deposits: wallet.total_deposits_in_naira(),
            total_withdrawals: wallet.total_withdrawals_in_naira(),
            status: wallet.status.unwrap_or(WalletStatus::Active),
            is_verified: wallet.is_verified.unwrap_or(false),
            bvn_verified: wallet.bvn_verified.unwrap_or(false),
            daily_limit: wallet.daily_limit.map(|limit| limit as f64 / 100.0).unwrap_or(0.0),
            monthly_limit: wallet.monthly_limit.map(|limit| limit as f64 / 100.0).unwrap_or(0.0),
            created_at: wallet.created_at.expect("Wallet must have a creation date"),
            last_activity_at: wallet.last_activity_at,
        }
    }
}

impl From<WalletTransaction> for TransactionResponseDto {
    fn from(tx: WalletTransaction) -> Self {
        Self {
            id: tx.id,
            transaction_type: tx.transaction_type.unwrap_or(TransactionType::Transfer),
            amount: tx.amount_in_naira(),
            fee_amount: tx.fee_amount_in_naira(),
            balance_before: kobo_to_naira(tx.balance_before),
            balance_after: kobo_to_naira(tx.balance_after),
            status: tx.status.unwrap_or(TransactionStatus::Pending),
            reference: tx.reference,
            external_reference: tx.external_reference,
            payment_method: tx.payment_method,
            description: tx.description,
            recipient: None, // Would be populated separately
            metadata: tx.metadata,
            created_at: tx.created_at.expect("Transaction must have a creation date"),
            completed_at: tx.completed_at,
        }
    }
}

impl From<BankAccount> for BankAccountResponseDto {
    fn from(account: BankAccount) -> Self {
        Self {
            id: account.id,
            account_name: account.account_name,
            account_number: account.account_number,
            bank_code: account.bank_code,
            bank_name: account.bank_name,
            is_verified: account.is_verified.unwrap_or(false),
            is_primary: account.is_primary.unwrap_or(false),
            created_at: account.created_at,
        }
    }
}