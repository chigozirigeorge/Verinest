use axum::{response::IntoResponse, Extension, Json};
use std::sync::Arc;
use validator::Validate;
use web3::{
    signing::{recover},
};
use hex;
use tiny_keccak::{Keccak, Hasher};

use crate::{
    db::UserExt,
    error::HttpError,
    middleware::JWTAuthMiddeware,
    models::walletmodels::{WalletUpdateRequest, WalletVerificationRequest},
    AppState
};

async fn verify_wallet_signature(
    signature: &str,
    message: &str,
    expected_address: &str,
) -> Result<String, String> {
    //Removing the "0x" prefix if present
    let signature_clean = signature.strip_prefix("0x").unwrap_or(signature);

    //parse the signature
    let signature_bytes = hex::decode(signature_clean)
        .map_err(|e| format!("Invalid signature format: {}", e))?;

    if signature_bytes.len() != 65 {
        return Err("Signature must be 65 bytes long".to_string());
    }

    //Extract recovery id (27 or 28) from the last bytes
    let recovery_id = signature_bytes[64].into();
    let recovery_id_32 = match recovery_id {
        27  => 0, 
        28 => 1,
        v @ 0 | v @ 1 => v as i32, 
        _ => return Err("Invalid recovery id".to_string()),
    };

    //creating a signature struct
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&signature_bytes[0..64]);

    //Hash the message with (EIP-191 compliant)
    let message_hash = hash_message(message);

    //Recover the address
    let recovered = recover(&message_hash, &sig, recovery_id_32)
        .map_err(|e| format!("Recovery failed: {}", e))?;

    Ok(format!("0x{}", hex::encode(recovered.as_bytes())))
}

fn hash_message(message: &str) -> [u8; 32] {
    //EIP-191 compliant message hashing
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut eth_message = Vec::new();
    eth_message.extend_from_slice(prefix.as_bytes());
    eth_message.extend_from_slice(message.as_bytes());

    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(&eth_message);
    hasher.finalize(&mut output);
    output
}

fn validate_ethereum_address(address: &str) -> Result<(), String> {
    let address_clean = address.strip_prefix("0x").unwrap_or(address);

    if address_clean.len() != 40 {
        return Err("Invalid Ethereum address length".to_string());
    }

    if !address_clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid Ethereum address format".to_string());
    }

    Ok(())
}

pub async fn update_wallet(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<WalletUpdateRequest>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let user_id = user.user.id;

    //Update primary wallet
    let updated_user = app_state.db_client
        .update_user_wallet(user_id, body.wallet_address.clone())
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    //Add to wallet table
    let wallet = app_state.db_client
        .add_user_wallet(user_id, body)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Wallet address updated successfully",
        "data": {
            "user": updated_user,
            "wallet": wallet
        }
    })))
}

pub async fn get_wallet(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user_id = user.user.id;

    let wallets = app_state.db_client
        .get_user_wallets(user_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "succesful",
        "data": {
            "wallets": wallets
        }
    })))
}

pub async fn verify_wallet(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<WalletVerificationRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let user_id = user.user.id;

    //Validate wallet address format
    let recovered_address = verify_wallet_signature(
        &body.signature, 
        &body.message, 
        &body.wallet_address
    )
    .await
    .map_err(|e| HttpError::bad_request(e))?;

    //checking to see if recovered address matches the claimed wallet address
    if recovered_address.to_lowercase() != body.wallet_address.to_lowercase() {
        return Err(HttpError::bad_request("Signature verification failed: address does not match"));
    }

    //verify the wallet belongs to the user
    let user_wallets = app_state.db_client
        .get_user_wallets(user_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let wallet_exists = user_wallets.iter()
        .any(|w| w.wallet_address.to_lowercase() == body.wallet_address.to_lowercase());

    if !wallet_exists {
        return Err(HttpError::bad_request("wallet address not associated with this user"));
    }

    let verified_wallet = app_state.db_client
        .verify_wallet(user_id, body.wallet_address)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "wallet verified successfully",
        "data": {
            "wallet": verified_wallet
        }
    })))
}

//Generate a verification message for the frontend to sign
pub async fn generate_verification_message(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user_id = user.user.id;

    //Generate a unique nonce to prevent replay attacks
    let nonce = chrono::Utc::now().timestamp();
    let message = format!("Please sign this message to verify your wallet.Nonce: {}", nonce);

    //Store the nonce in the database associated with the user
    // app_state.db_client
    //     .store_wallet_verification_nonce(user_id, nonce)
    //     .await
    //     .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "message": message,
            "nonce": nonce,
            "expires_in": 300
        }
    })))
}

//Additional endpoint to get wallet verification status
pub async fn get_wallet_verification_status(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user_id = user.user.id;

    let wallets = app_state.db_client
        .get_user_wallets(user_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let primary_wallet = app_state.db_client
        .get_primary_wallet(user_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "wallets": wallets,
            "primary_wallet": primary_wallet,
            "has_verified_wallet": wallets.iter().any(|w| w.is_verified.unwrap_or(false)),
        }
    })))
}