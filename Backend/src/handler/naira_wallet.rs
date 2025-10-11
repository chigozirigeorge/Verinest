// handler/naira_wallet.rs
use std::sync::Arc;
use std::collections::HashMap;
use axum::{
    http::{ HeaderMap, StatusCode},
    extract::{Path, Query},
    response::IntoResponse,
    routing::{get, post, put},
    Extension, Json, Router,
};
use uuid::Uuid;
use validator::Validate;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use serde_json::Value;
use subtle::ConstantTimeEq;

use crate::{
    db::{
        userdb::UserExt,
        naira_walletdb::NairaWalletExt
    },
    dtos::naira_walletdtos::*,
    error::HttpError,
    middleware::JWTAuthMiddeware,
    models::walletmodels::*,
    service::payment_provider::PaymentProviderService,
    AppState,
};

pub fn naira_wallet_handler() -> Router {
    Router::new()
        // Wallet management - remove /wallet prefix since router will be mounted at /wallet
        .route("/", get(get_wallet))  // ✅ Becomes /api/wallet/wallet when mounted
        .route("/create", post(create_wallet))  // ✅ Becomes /api/wallet/create
        .route("/summary", get(get_wallet_summary))
        
        // Transactions
        .route("/deposit", post(initiate_deposit))
        .route("/deposit/verify", get(handle_paystack_redirect))
        .route("/deposit/verify", post(verify_deposit))
        .route("/withdraw", post(withdraw_funds))
        .route("/transfer", post(transfer_funds))
        
        // Transaction history
        .route("/transactions", get(get_transaction_history))
        .route("/transaction/:reference", get(get_transaction_by_ref))
        
        // Bank accounts
        .route("/bank-accounts", get(get_bank_accounts))
        .route("/bank-accounts", post(add_bank_account))
        .route("/bank-accounts/:account_id/verify", post(verify_bank_account))
        .route("/bank-accounts/:account_id/primary", put(set_primary_account))
        .route("/bank-accounts/resolve", post(resolve_account_number))
        
        // Webhooks
        .route("/webhook/paystack", post(paystack_webhook))
        .route("/webhook/flutterwave", post(flutterwave_webhook))
}

// Wallet Management Handlers
pub async fn get_wallet(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let wallet = app_state
        .db_client
        .get_naira_wallet(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Wallet not found"))?;

    let response: WalletResponseDto = wallet.into();
    Ok(Json(WalletApiResponse::success("Wallet retrieved successfully", response)))
}

pub async fn create_wallet(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    // Check if wallet already exists
    let existing = app_state
        .db_client
        .get_naira_wallet(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if existing.is_some() {
        return Err(HttpError::bad_request("Wallet already exists"));
    }

    let wallet = app_state
        .db_client
        .create_naira_wallet(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response: WalletResponseDto = wallet.into();
    Ok(Json(WalletApiResponse::success("Wallet created successfully", response)))
}

pub async fn get_wallet_summary(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let summary = app_state
        .db_client
        .get_wallet_summary(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response = WalletSummaryDto {
        balance: kobo_to_naira(summary.balance),
        available_balance: kobo_to_naira(summary.available_balance),
        total_deposits: kobo_to_naira(summary.total_deposits),
        total_withdrawals: kobo_to_naira(summary.total_withdrawals),
        pending_transactions: summary.pending_transactions,
        active_holds: kobo_to_naira(summary.active_holds),
        daily_spent: 0.0, // Calculate from today's transactions
        monthly_spent: 0.0, // Calculate from this month's transactions
        daily_limit: 1_000_000.0, // Get from wallet limits
        monthly_limit: 20_000_000.0, // Get from wallet limits
    };

    Ok(Json(WalletApiResponse::success("Wallet summary retrieved successfully", response)))
}

// Deposit Handlers
pub async fn initiate_deposit(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<DepositRequestDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Check transaction limits
    let amount_kobo = naira_to_kobo(body.amount);
    let can_transact = app_state
        .db_client
        .check_transaction_limits(auth.user.id, TransactionType::Deposit, amount_kobo)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if !can_transact {
        return Err(HttpError::bad_request("Transaction exceeds your limits"));
    }

    // Generate transaction reference
    let reference = generate_transaction_reference();

    // Initialize payment with provider (Paystack/Flutterwave)
    let payment_service = PaymentProviderService::new(&app_state.env);
    let payment_init = payment_service
        .initialize_payment(
            auth.user.email.clone(),
            body.amount,
            reference.clone(),
            body.payment_method,
            body.metadata.clone(),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Create pending transaction
    let wallet = app_state
        .db_client
        .get_naira_wallet(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Wallet not found"))?;

    // Create transaction record with pending status
    let tx_result = sqlx::query!(
        r#"
        INSERT INTO wallet_transactions 
        (wallet_id, user_id, transaction_type, amount, balance_before, balance_after,
         reference, description, status, payment_method)
        VALUES ($1, $2, 'deposit', $3, $4, $4, $5, $6, 'pending', $7)
        RETURNING id
        "#,
        wallet.id,
        auth.user.id,
        amount_kobo,
        wallet.balance,
        reference,
        body.description,
        body.payment_method as PaymentMethod
    )
    .fetch_one(&app_state.db_client.pool)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(WalletApiResponse::success(
        "Payment initialized successfully",
        payment_init,
    )))
}

pub async fn handle_paystack_redirect(
    Extension(app_state): Extension<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, HttpError> {
    let reference = params.get("trxref")
        .or_else(|| params.get("reference"))
        .ok_or_else(|| HttpError::bad_request("No reference provided"))?;

    tracing::info!("Paystack redirect received for reference: {}", reference);

    // Verify the payment
    let payment_service = PaymentProviderService::new(&app_state.env);
    let verification = payment_service
        .verify_payment(reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if verification.status == "success" {
        // Get transaction by reference
        let transaction = app_state
            .db_client
            .get_transaction_by_reference(reference)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::not_found("Transaction not found"))?;

        // Process payment if still pending
        if transaction.status == Some(TransactionStatus::Pending) {
            let _ = app_state
                .db_client
                .credit_wallet(
                    transaction.user_id,
                    verification.amount,
                    TransactionType::Deposit,
                    "Deposit via Paystack redirect".to_string(),
                    reference.to_string(),
                    Some(verification.gateway_reference),
                    verification.metadata,
                )
                .await
                .map_err(|e| HttpError::server_error(e.to_string()))?;
        }

            let app_url = &app_state.env.app_url;
        // Redirect to frontend success page
        let frontend_url = format!(
            "{}/payment/success?reference={}",
            app_url,
            reference
        );
        
        Ok(axum::response::Redirect::to(&frontend_url))
    } else {
        let app_url = &app_state.env.app_url;
        // Redirect to frontend failure page
        let frontend_url = format!(
            "{}/payment/failed?reference={}&error={}",
            app_url,
            reference,
            urlencoding::encode(&verification.gateway_reference)
        );
        
        Ok(axum::response::Redirect::to(&frontend_url))
    }
}

pub async fn verify_deposit(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let reference = body["reference"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Reference is required"))?;

    // Verify payment with provider
    let payment_service = PaymentProviderService::new(&app_state.env);
    let verification = payment_service
        .verify_payment(reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if verification.status == "success" {
        // Credit wallet
        let transaction = app_state
            .db_client
            .credit_wallet(
                auth.user.id,
                verification.amount,
                TransactionType::Deposit,
                "Deposit via payment gateway".to_string(),
                reference.to_string(),
                Some(verification.gateway_reference),
                verification.metadata,
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        let response: TransactionResponseDto = transaction.into();
        Ok(Json(WalletApiResponse::success(
            "Deposit successful",
            response,
        )))
    } else {
        Err(HttpError::bad_request("Payment verification failed"))
    }
}

// Withdrawal Handler
pub async fn withdraw_funds(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<WithdrawalRequestDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let amount_kobo = naira_to_kobo(body.amount);

    // Check transaction limits
    let can_transact = app_state
        .db_client
        .check_transaction_limits(auth.user.id, TransactionType::Withdrawal, amount_kobo)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if !can_transact {
        return Err(HttpError::bad_request("Transaction exceeds your limits"));
    }

    // Get bank account details
     // Get bank account details - FIX: Use manual query
    let bank_account = sqlx::query_as::<_, BankAccount>(
        "SELECT * FROM bank_accounts WHERE id = $1 AND user_id = $2 AND is_verified = true"
    )
    .bind(body.bank_account_id)
    .bind(auth.user.id)
    .fetch_optional(&app_state.db_client.pool)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?
    .ok_or_else(|| HttpError::not_found("Bank account not found or not verified"))?;


    // Calculate fee
    let fee = app_state
        .db_client
        .calculate_transaction_fee(TransactionType::Withdrawal, amount_kobo)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let total_deduction = amount_kobo + fee;

    // Check balance
    let balance = app_state
        .db_client
        .get_wallet_balance(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if balance < total_deduction {
        return Err(HttpError::bad_request("Insufficient balance"));
    }

    // Generate reference
    let reference = generate_transaction_reference();

    // Initiate transfer with payment provider
    let payment_service = PaymentProviderService::new(&app_state.env);
    let transfer_result = payment_service
        .initiate_transfer(
            bank_account.account_number.clone(),
            bank_account.bank_code.clone(),
            body.amount,
            reference.clone(),
            body.description.clone(),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Debit wallet
    let mut metadata = serde_json::json!({
        "bank_account": bank_account.account_number,
        "bank_name": bank_account.bank_name,
        "transfer_code": transfer_result.transfer_code,
    });

    if let Some(meta) = body.metadata {
        if let Some(obj) = metadata.as_object_mut() {
            if let Some(body_obj) = meta.as_object() {
                for (k, v) in body_obj {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
    }

    let transaction = app_state
        .db_client
        .debit_wallet(
            auth.user.id,
            total_deduction,
            TransactionType::Withdrawal,
            body.description,
            reference,
            Some(transfer_result.reference),
            Some(metadata),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response: TransactionResponseDto = transaction.into();
    Ok(Json(WalletApiResponse::success(
        "Withdrawal initiated successfully",
        response,
    )))
}

// Transfer Handler
pub async fn transfer_funds(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<TransferRequestDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Find recipient by email, username, or phone
    let recipient = app_state
        .db_client
        .get_user(None, Some(&body.recipient_identifier), Some(&body.recipient_identifier), None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Recipient not found"))?;

    if recipient.id == auth.user.id {
        return Err(HttpError::bad_request("Cannot transfer to yourself"));
    }

    let amount_kobo = naira_to_kobo(body.amount);

    // Check transaction limits
    let can_transact = app_state
        .db_client
        .check_transaction_limits(auth.user.id, TransactionType::Transfer, amount_kobo)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if !can_transact {
        return Err(HttpError::bad_request("Transaction exceeds your limits"));
    }

    // Generate reference
    let reference = generate_transaction_reference();

    // Execute transfer
    let (sender_tx, recipient_tx) = app_state
        .db_client
        .transfer_funds(
            auth.user.id,
            recipient.id,
            amount_kobo,
            body.description,
            reference,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response: TransactionResponseDto = sender_tx.into();
    Ok(Json(WalletApiResponse::success(
        "Transfer successful",
        response,
    )))
}

// Transaction History
pub async fn get_transaction_history(
    Query(params): Query<TransactionHistoryQueryDto>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    params.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    let transactions = app_state
        .db_client
        .get_wallet_transactions(
            auth.user.id,
            params.transaction_type,
            params.status,
            limit,
            offset,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response: Vec<TransactionResponseDto> = transactions
        .into_iter()
        .map(|tx| tx.into())
        .collect();

    Ok(Json(PaginatedTransactionResponse {
        status: "success".to_string(),
        data: response,
        pagination: PaginationMetadata {
            total: 0, // Would calculate total count
            page: offset / limit,
            limit,
            total_pages: 0,
        },
    }))
}

pub async fn get_transaction_by_ref(
    Path(reference): Path<String>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let transaction = app_state
        .db_client
        .get_transaction_by_reference(&reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Transaction not found"))?;

    // Verify transaction belongs to user
    if transaction.user_id != auth.user.id {
        return Err(HttpError::unauthorized("Unauthorized access"));
    }

    let response: TransactionResponseDto = transaction.into();
    Ok(Json(WalletApiResponse::success(
        "Transaction retrieved successfully",
        response,
    )))
}

// Bank Account Handlers
pub async fn get_bank_accounts(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let accounts = app_state
        .db_client
        .get_user_bank_accounts(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response: Vec<BankAccountResponseDto> = accounts
        .into_iter()
        .map(|acc| acc.into())
        .collect();

    Ok(Json(WalletApiResponse::success(
        "Bank accounts retrieved successfully",
        response,
    )))
}

pub async fn verify_bank_account(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(account_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    // Verify bank account
    let verified_account = app_state
        .db_client
        .verify_bank_account(account_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Verify the account belongs to the user
    if verified_account.user_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to verify this account"));
    }

    let response: BankAccountResponseDto = verified_account.into();
    Ok(Json(WalletApiResponse::success(
        "Bank account verified successfully",
        response,
    )))
}

pub async fn add_bank_account(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<AddBankAccountDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Resolve account number to get account name
    let payment_service = PaymentProviderService::new(&app_state.env);
    let verification = payment_service
        .resolve_account_number(&body.account_number, &body.bank_code)
        .await
        .map_err(|e| HttpError::server_error(format!("Could not verify account: {}", e)))?;

    // Add bank account
    let account = app_state
        .db_client
        .add_bank_account(
            auth.user.id,
            verification.account_name.clone(),
            body.account_number,
            body.bank_code,
            verification.bank_name,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Mark as verified since we resolved it
    let verified_account = app_state
        .db_client
        .verify_bank_account(account.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response: BankAccountResponseDto = verified_account.into();
    Ok(Json(WalletApiResponse::success(
        "Bank account added successfully",
        response,
    )))
}

pub async fn set_primary_account(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(account_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    // Set the bank account as primary
    let primary_account = app_state
        .db_client
        .set_primary_bank_account(auth.user.id, account_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response: BankAccountResponseDto = primary_account.into();
    Ok(Json(WalletApiResponse::success(
        "Primary bank account set successfully",
        response,
    )))
}


pub async fn resolve_account_number(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(_auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let account_number = body["account_number"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Account number is required"))?;
    let bank_code = body["bank_code"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Bank code is required"))?;

    let payment_service = PaymentProviderService::new(&app_state.env);
    let verification = payment_service
        .resolve_account_number(account_number, bank_code)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(WalletApiResponse::success(
        "Account resolved successfully",
        verification,
    )))
}


// In naira_wallet.rs - Add secure public verification
pub async fn public_verify_deposit(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let reference = body["reference"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Reference is required"))?;

    // SECURITY: Validate reference format to prevent random guessing
    if !reference.starts_with("VRN_") || reference.len() < 10 {
        return Err(HttpError::bad_request("Invalid reference format"));
    }

    // Get transaction by reference first
    let transaction = app_state
        .db_client
        .get_transaction_by_reference(reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Transaction not found"))?;

    // SECURITY: Check if transaction is already completed
    if transaction.status == Some(TransactionStatus::Completed) {
        let response: TransactionResponseDto = transaction.into();
        return Ok(Json(WalletApiResponse::success(
            "Payment already verified",
            response,
        )));
    }

    // SECURITY: Verify payment with provider (Paystack)
    let payment_service = PaymentProviderService::new(&app_state.env);
    let verification = payment_service
        .verify_payment(reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // SECURITY: Only process if payment provider confirms success
    if verification.status == "success" {
        // Credit wallet using user_id from transaction
        let updated_transaction = app_state
            .db_client
            .credit_wallet(
                transaction.user_id,
                verification.amount,
                TransactionType::Deposit,
                "Deposit via payment gateway".to_string(),
                reference.to_string(),
                Some(verification.gateway_reference),
                verification.metadata,
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        let response: TransactionResponseDto = updated_transaction.into();
        Ok(Json(WalletApiResponse::success(
            "Deposit verified successfully",
            response,
        )))
    } else {
        // SECURITY: Log failed verification attempts
        tracing::warn!("Failed payment verification for reference: {}", reference);
        Err(HttpError::bad_request("Payment verification failed"))
    }
}


// Paystack Webhook Handler
pub async fn paystack_webhook(
    Extension(app_state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    // Verify webhook signature
    let signature = headers
        .get("x-paystack-signature")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            HttpError::new(
                "Missing or invalid Paystack signature".to_string(),
                StatusCode::BAD_REQUEST,
            )
        })?;

    let webhook_secret: &String = &app_state.env.paystack_secret_key;

    // Verify HMAC signature
    if !verify_paystack_signature(&body, signature, webhook_secret) {
        tracing::warn!("Invalid Paystack webhook signature received");
        return Err(HttpError::new(
            "Invalid webhook signature".to_string(),
            StatusCode::UNAUTHORIZED,
        ));
    }

    // Process webhook event
    let event_type = body["event"]
        .as_str()
        .ok_or_else(|| {
            HttpError::new(
                "Missing event type in webhook payload".to_string(),
                StatusCode::BAD_REQUEST,
            )
        })?;

    let data = &body["data"];

    match event_type {
        "charge.success" => {
            process_paystack_successful_payment(&app_state, data).await?;
        }
        "transfer.success" => {
            process_paystack_successful_transfer(&app_state, data).await?;
        }
        "transfer.failed" => {
            process_paystack_failed_transfer(&app_state, data).await?;
        }
        "transfer.reversed" => {
            process_paystack_reversed_transfer(&app_state, data).await?;
        }
        _ => {
            tracing::info!("Unhandled Paystack webhook event: {}", event_type);
        }
    }

    Ok(Json(serde_json::json!({"status": "success"})))
}

// Flutterwave Webhook Handler
pub async fn flutterwave_webhook(
    Extension(app_state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    // Verify webhook signature
    let signature = headers
        .get("verif-hash")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            HttpError::new(
                "Missing or invalid Flutterwave signature".to_string(),
                StatusCode::BAD_REQUEST,
            )
        })?;

    let webhook_secret: &String = &app_state.env.flutterwave_secret_key;

    // Verify signature
    if signature != webhook_secret {
        tracing::warn!("Invalid Flutterwave webhook signature received");
        return Err(HttpError::new(
            "Invalid webhook signature".to_string(),
            StatusCode::UNAUTHORIZED,
        ));
    }

    // Process webhook event
    let event_type = body["event"]
        .as_str()
        .ok_or_else(|| {
            HttpError::new(
                "Missing event type in webhook payload".to_string(),
                StatusCode::BAD_REQUEST,
            )
        })?;

    let data = &body["data"];

    match event_type {
        "charge.completed" => {
            process_flutterwave_completed_charge(&app_state, data).await?;
        }
        "transfer.completed" => {
            process_flutterwave_completed_transfer(&app_state, data).await?;
        }
        "transfer.failed" => {
            process_flutterwave_failed_transfer(&app_state, data).await?;
        }
        "transfer.reversed" => {
            process_flutterwave_reversed_transfer(&app_state, data).await?;
        }
        _ => {
            tracing::info!("Unhandled Flutterwave webhook event: {}", event_type);
        }
    }

    Ok(Json(serde_json::json!({"status": "success"})))
}

// Paystack Helper Functions
fn verify_paystack_signature(payload: &Value, signature: &str, secret: &str) -> bool {
    let payload_string = payload.to_string();
    
    let mut mac = Hmac::<Sha512>::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(payload_string.as_bytes());
    
    let expected_signature = mac.finalize().into_bytes();
    let expected_signature_hex = hex::encode(expected_signature);
    
    // Compare signatures in constant time to prevent timing attacks
    ConstantTimeEq::ct_eq(
        signature.as_bytes(),
        expected_signature_hex.as_bytes(),
    ).into()
}

async fn process_paystack_successful_payment(
    app_state: &Arc<AppState>,
    data: &Value,
) -> Result<(), HttpError> {
    let reference = data["reference"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Missing reference in webhook data"))?;

    let amount = data["amount"]
        .as_f64()
        .ok_or_else(|| HttpError::bad_request("Missing amount in webhook data"))?;

    let gateway_reference = data["id"]
        .as_str()
        .map(|s| s.to_string());

    // Convert amount from kobo to your base unit (assuming amount is in kobo from Paystack)
    let amount_kobo = amount as i64;

    // Find transaction by reference
    let transaction = app_state.db_client
        .get_transaction_by_reference(reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| {
            tracing::warn!("Transaction not found for reference: {}", reference);
            HttpError::not_found("Transaction not found")
        })?;

    // Only process if transaction is still pending
    if transaction.status != Some(TransactionStatus::Pending) {
        tracing::info!("Transaction {} already processed with status: {:?}", reference, transaction.status);
        return Ok(());
    }

    // Update transaction status first
    let updated_transaction = app_state.db_client
        .update_transaction_status(
            transaction.id,
            TransactionStatus::Completed,
            gateway_reference,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Credit the wallet
    let wallet_transaction = app_state.db_client
        .credit_wallet(
            transaction.user_id,
            amount_kobo,
            transaction.transaction_type,
            "Payment confirmed via Paystack webhook".to_string(),
            reference.to_string(),
            updated_transaction.external_reference.clone(),
            Some(serde_json::json!(data)),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    tracing::info!(
        "Successfully processed Paystack payment for reference: {}, user: {}, amount: {}",
        reference,
        transaction.user_id,
        amount_kobo
    );

    Ok(())
}

async fn process_paystack_successful_transfer(
    app_state: &Arc<AppState>,
    data: &Value,
) -> Result<(), HttpError> {
    let transfer_reference = data["reference"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Missing reference in transfer data"))?;

    let status = data["status"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Missing status in transfer data"))?;

    if status == "success" {
        tracing::info!("Paystack transfer {} completed successfully", transfer_reference);
        
        // Update withdrawal transaction status if it exists
        if let Some(transaction) = app_state.db_client
            .get_transaction_by_reference(transfer_reference)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
        {
            if transaction.status == Some(TransactionStatus::Pending) {
                let _ = app_state.db_client
                    .update_transaction_status(
                        transaction.id,
                        TransactionStatus::Completed,
                        Some(transfer_reference.to_string()),
                    )
                    .await
                    .map_err(|e| HttpError::server_error(e.to_string()))?;
                
                tracing::info!("Updated withdrawal transaction {} to success", transfer_reference);
            }
        }
    }

    Ok(())
}

async fn process_paystack_failed_transfer(
    app_state: &Arc<AppState>,
    data: &Value,
) -> Result<(), HttpError> {
    let transfer_reference = data["reference"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Missing reference in transfer data"))?;

    let reason = data["reason"]
        .as_str()
        .unwrap_or("Unknown reason");

    tracing::warn!("Paystack transfer {} failed: {}", transfer_reference, reason);

    // Find the withdrawal transaction and mark it as failed
    if let Some(transaction) = app_state.db_client
        .get_transaction_by_reference(transfer_reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
    {
        if transaction.status == Some(TransactionStatus::Pending) {
            let _ = app_state.db_client
                .update_transaction_status(
                    transaction.id,
                    TransactionStatus::Failed,
                    Some(format!("Transfer failed: {}", reason)),
                )
                .await
                .map_err(|e| HttpError::server_error(e.to_string()))?;

            // Refund the amount back to available balance
            let _ = app_state.db_client
                .refund_transaction(transaction.id)
                .await
                .map_err(|e| HttpError::server_error(e.to_string()))?;

            tracing::info!("Refunded failed transfer {} for user {}", transfer_reference, transaction.user_id);
        }
    }

    Ok(())
}

async fn process_paystack_reversed_transfer(
    app_state: &Arc<AppState>,
    data: &Value,
) -> Result<(), HttpError> {
    let transfer_reference = data["reference"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Missing reference in transfer data"))?;

    tracing::info!("Paystack transfer {} was reversed", transfer_reference);

    // Handle transfer reversal - refund the user
    if let Some(transaction) = app_state.db_client
        .get_transaction_by_reference(transfer_reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
    {
        let _ = app_state.db_client
            .refund_transaction(transaction.id)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        tracing::info!("Refunded reversed transfer {} for user {}", transfer_reference, transaction.user_id);
    }

    Ok(())
}

// Flutterwave Helper Functions
async fn process_flutterwave_completed_charge(
    app_state: &Arc<AppState>,
    data: &Value,
) -> Result<(), HttpError> {
    let tx_ref = data["tx_ref"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Missing tx_ref in webhook data"))?;

    let amount = data["amount"]
        .as_f64()
        .ok_or_else(|| HttpError::bad_request("Missing amount in webhook data"))?;

    let flw_ref = data["flw_ref"]
        .as_str()
        .map(|s| s.to_string());

    let status = data["status"]
        .as_str()
        .unwrap_or("");

    // Convert amount from naira to kobo (Flutterwave sends amount in Naira)
    let amount_kobo = (amount * 100.0) as i64;

    if status == "successful" {
        // Find transaction by reference
        let transaction = app_state.db_client
            .get_transaction_by_reference(tx_ref)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| {
                tracing::warn!("Transaction not found for reference: {}", tx_ref);
                HttpError::not_found("Transaction not found")
            })?;

        // Only process if transaction is still pending
        if transaction.status != Some(TransactionStatus::Pending) {
            tracing::info!("Transaction {} already processed", tx_ref);
            return Ok(());
        }

        // Update transaction status and credit wallet
        let updated_transaction = app_state.db_client
            .update_transaction_status(
                transaction.id,
                TransactionStatus::Completed,
                flw_ref,
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        // Credit the wallet
        let _ = app_state.db_client
            .credit_wallet(
                transaction.user_id,
                amount_kobo,
                transaction.transaction_type,
                "Payment confirmed via Flutterwave webhook".to_string(),
                tx_ref.to_string(),
                updated_transaction.external_reference.clone(),
                Some(serde_json::json!(data)),
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        tracing::info!(
            "Successfully processed Flutterwave payment for reference: {}, user: {}, amount: {}",
            tx_ref,
            transaction.user_id,
            amount_kobo
        );
    }

    Ok(())
}

async fn process_flutterwave_completed_transfer(
    app_state: &Arc<AppState>,
    data: &Value,
) -> Result<(), HttpError> {
    let transfer_reference = data["ref"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Missing ref in transfer data"))?;

    let status = data["status"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Missing status in transfer data"))?;

    if status == "successful" {
        tracing::info!("Flutterwave transfer {} completed successfully", transfer_reference);
        
        // Update withdrawal transaction status if it exists
        if let Some(transaction) = app_state.db_client
            .get_transaction_by_reference(transfer_reference)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
        {
            if transaction.status == Some(TransactionStatus::Pending) {
                let _ = app_state.db_client
                    .update_transaction_status(
                        transaction.id,
                        TransactionStatus::Completed,
                        Some(transfer_reference.to_string()),
                    )
                    .await
                    .map_err(|e| HttpError::server_error(e.to_string()))?;
                
                tracing::info!("Updated withdrawal transaction {} to success", transfer_reference);
            }
        }
    }

    Ok(())
}

async fn process_flutterwave_failed_transfer(
    app_state: &Arc<AppState>,
    data: &Value,
) -> Result<(), HttpError> {
    let transfer_reference = data["ref"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Missing ref in transfer data"))?;

    let reason = data["complete_message"]
        .as_str()
        .unwrap_or("Unknown reason");

    tracing::warn!("Flutterwave transfer {} failed: {}", transfer_reference, reason);

    // Find the withdrawal transaction and mark it as failed
    if let Some(transaction) = app_state.db_client
        .get_transaction_by_reference(transfer_reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
    {
        if transaction.status == Some(TransactionStatus::Pending) {
            let _ = app_state.db_client
                .update_transaction_status(
                    transaction.id,
                    TransactionStatus::Failed,
                    Some(format!("Transfer failed: {}", reason)),
                )
                .await
                .map_err(|e| HttpError::server_error(e.to_string()))?;

            // Refund the amount back to available balance
            let _ = app_state.db_client
                .refund_transaction(transaction.id)
                .await
                .map_err(|e| HttpError::server_error(e.to_string()))?;

            tracing::info!("Refunded failed transfer {} for user {}", transfer_reference, transaction.user_id);
        }
    }

    Ok(())
}

async fn process_flutterwave_reversed_transfer(
    app_state: &Arc<AppState>,
    data: &Value,
) -> Result<(), HttpError> {
    let transfer_reference = data["ref"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Missing ref in transfer data"))?;

    tracing::info!("Flutterwave transfer {} was reversed", transfer_reference);

    // Handle transfer reversal - refund the user
    if let Some(transaction) = app_state.db_client
        .get_transaction_by_reference(transfer_reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
    {
        let _ = app_state.db_client
            .refund_transaction(transaction.id)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        tracing::info!("Refunded reversed transfer {} for user {}", transfer_reference, transaction.user_id);
    }

    Ok(())
}
