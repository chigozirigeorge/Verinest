// handler/verification.rs
use std::sync::Arc;

use axum::{
    extract::{Path},
    response::IntoResponse,
    routing::{get, post, put},
    Extension, Json, Router,
};
use chrono::{Duration, Utc};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db::{userdb::UserExt, verificationdb::VerificationExt},
    dtos::verificationdtos::*,
    error::HttpError,
    mail::mails::send_otp_email,
    middleware::JWTAuthMiddeware,
    models::usermodel::{VerificationStatus, VerificationType, UserRole},
    utils::{image_utils, otp_generator},
    AppState,
};

pub fn verification_handler() -> Router {
    Router::new()
        // OTP Routes
        .route("/otp/send", post(send_otp))
        .route("/otp/verify", post(verify_otp))
        
        // Verification Routes
        .route("/nin", post(submit_nin_verification))
        .route("/document", post(submit_document_verification))
        .route("/documents", get(get_user_verifications))
        .route("/status", get(get_verification_status))
        .route("/complete-status", get(get_complete_verification_status))
        
        // Admin Routes
        .route("/admin/pending", get(get_pending_verifications))
        .route("/admin/:verification_id/review", put(review_verification))
}

// OTP Handlers
pub async fn send_otp(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<OtpRequestDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Generate OTP
    let otp_code = otp_generator::generate_otp();
    let expires_at = Utc::now() + Duration::minutes(10); // OTP valid for 10 minutes

    // Store OTP in database
    let otp_record = app_state.db_client
        .create_otp(
            auth.user.id,
            body.email.clone(),
            otp_code.clone(),
            body.purpose.clone(),
            expires_at,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Send OTP via email
    send_otp_email(&body.email, &otp_code, &body.purpose)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "OTP sent successfully",
        "expires_at": expires_at
    })))
}

pub async fn verify_otp(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(body): Json<OtpVerificationDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Verify OTP
    let otp_record = app_state.db_client
        .get_valid_otp(&body.email, &body.otp_code, body.purpose.clone())
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::bad_request("Invalid or expired OTP"))?;

    // Mark OTP as used
    app_state.db_client
        .mark_otp_used(otp_record.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "OTP verified successfully",
        "verified": true
    })))
}

// Verification Handlers
pub async fn submit_nin_verification(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<NinVerificationDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Check if this NIN is already verified by another user
    let existing_user = app_state.db_client
        .get_user_by_verification_number(&body.nin_number)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if let Some(existing_user) = existing_user {
        if existing_user.id != auth.user.id {
            return Err(HttpError::bad_request("This NIN number is already verified by another user"));
        }
    }

    // Create verification document
    let verification = app_state.db_client
        .create_verification_document(
            auth.user.id,
            VerificationType::NationalId,
            body.nin_number.clone(),
            body.document_url.clone(),
            body.selfie_url.clone(),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Update user with basic verification data (pending status)
    app_state.db_client
        .update_user_verification_data(
            auth.user.id,
            VerificationStatus::Submitted,
            Some(body.nin_number.clone()), // Store NIN as verification_number
            VerificationType::NationalId,
            Some(body.document_url.clone()), // Store document URL
            Some(body.selfie_url.clone()),   // Store selfie URL
            Some(body.nationality.clone()),
            body.dob,
            body.lga.clone(),
            body.nearest_landmark.clone(),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "NIN verification submitted successfully",
        "verification_id": verification.id
    })))
}


pub async fn submit_document_verification(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<DocumentVerificationDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Check if this document ID is already verified by another user
    let existing_user = app_state.db_client
        .get_user_by_verification_number(&body.document_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if let Some(existing_user) = existing_user {
        if existing_user.id != auth.user.id {
            return Err(HttpError::bad_request("This document ID is already verified by another user"));
        }
    }

    // Create verification document
    let verification = app_state.db_client
        .create_verification_document(
            auth.user.id,
            body.verification_type,
            body.document_id.clone(),
            body.document_url.clone(),
            body.selfie_url.clone(),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Update user with basic verification data (pending status)
    app_state.db_client
        .update_user_verification_data(
            auth.user.id,
            VerificationStatus::Submitted,
            Some(body.document_id.clone()), // Store document ID as verification_number
            body.verification_type,
            Some(body.document_url.clone()), // Store document URL
            Some(body.selfie_url.clone()),   // Store selfie URL
            Some(body.nationality.clone()),
            body.dob,
            body.lga.clone(),
            body.nearest_landmark.clone(),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Document verification submitted successfully",
        "verification_id": verification.id
    })))
}

pub async fn get_user_verifications(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let verifications = app_state.db_client
        .get_user_verification_documents(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": verifications
    })))
}

pub async fn get_verification_status(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let user = app_state.db_client
        .get_user(Some(auth.user.id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "verification_status": user.verification_status,
        "is_verified": user.verification_status == Some(VerificationStatus::Approved)
    })))
}

// Admin Handlers
pub async fn get_pending_verifications(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    // Check if user is admin or verifier
    if auth.user.role != UserRole::Admin 
        && auth.user.role != UserRole::Verifier {
        return Err(HttpError::unauthorized("Insufficient permissions"));
    }

    let pending_verifications = app_state.db_client
        .get_pending_document_verifications()
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": pending_verifications
    })))
}

pub async fn review_verification(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(verification_id): Path<Uuid>,
    Json(body): Json<ReviewVerificationDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Check if user is admin or verifier
    if auth.user.role != UserRole::Admin 
        && auth.user.role != UserRole::Verifier {
        return Err(HttpError::unauthorized("Insufficient permissions"));
    }

    // Get the verification document first
    let verification = app_state.db_client
        .get_verification_by_id(verification_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Verification document not found"))?;

    // Update verification status
    let updated_verification = app_state.db_client
        .update_verification_status(
            verification_id,
            body.status,
            Some(auth.user.id),
            body.review_notes.clone(),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Get user to send email
    let user = app_state.db_client
        .get_user(Some(verification.user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    // Send verification status email
    if let Err(e) = crate::mail::mails::send_verification_status_email(
        &user.email,
        &user.name,
        &body.status,
        body.review_notes.as_deref(),
    ).await {
        tracing::error!("Failed to send verification status email: {}", e);
        // Don't fail the request if email fails
    }

    // If approved, update user verification status AND populate user data
    if body.status == VerificationStatus::Approved {
        match verification.document_type {
            VerificationType::NationalId => {
                app_state.db_client
                    .update_user_verification_data(
                        verification.user_id,
                        VerificationStatus::Approved,
                        Some(verification.document_id.clone()),
                        verification.document_type,
                        Some(verification.document_url.clone()),
                        Some(verification.selfie_url.clone()),
                        None,
                        None,
                        None,
                        None,
                    )
                    .await
                    .map_err(|e| HttpError::server_error(e.to_string()))?;
            }
            VerificationType::DriverLicense | VerificationType::Passport => {
                app_state.db_client
                    .update_user_verification_data(
                        verification.user_id,
                        VerificationStatus::Approved,
                        Some(verification.document_id.clone()),
                        verification.document_type,
                        Some(verification.document_url.clone()),
                        Some(verification.selfie_url.clone()),
                        None,
                        None,
                        None,
                        None,
                    )
                    .await
                    .map_err(|e| HttpError::server_error(e.to_string()))?;
            }
        }

        // Also update the general verification status
        app_state.db_client
            .update_user_verification_status(verification.user_id, VerificationStatus::Approved)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

       match app_state.notification_service.notify_verification_accepted(&verification)
        .await{
            Ok(_) => tracing::info!("verification notification sent to {}", &verification.user_id),
            Err(e) => tracing::error!("Failed to notify verification request: {:?}", e),
        };

        
    }

    if body.status == VerificationStatus::Rejected {
        // Update user with verification data based on document type
                app_state.db_client
                    .update_user_verification_data(
                        verification.user_id,
                        VerificationStatus::Rejected,
                        Some("".to_string()),
                        VerificationType::NationalId,
                        Some("".to_string()),
                        Some("".to_string()),
                        None,
                        None,
                        None,
                        None,
                    )
                    .await
                    .map_err(|e| HttpError::server_error(e.to_string()))?;

            match app_state.notification_service.notify_verification_rejected(
                &verification
            ).await{
            Ok(_) => tracing::info!("verification notification sent to {}", &verification.user_id),
            Err(e) => tracing::error!("Failed to notify verification request: {:?}", e),
            };

        }

        // Also update the general verification status
        app_state.db_client
            .update_user_verification_status(verification.user_id, VerificationStatus::Unverified)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Verification reviewed successfully",
        "data": updated_verification
    })))
}


// handler/verification.rs (add this new handler)
pub async fn get_complete_verification_status(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    // Get user with updated verification data
    let user = app_state.db_client
        .get_user(Some(auth.user.id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    // Get all verification documents
    let verification_documents = app_state.db_client
        .get_user_verification_documents(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response = CompleteUserVerificationDto::from_user_and_documents(user, verification_documents);

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": response
    })))
}