// handler/vendor.rs
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    routing::{get, post, put},
    Extension, Json, Router,
};
use uuid::Uuid;
use validator::Validate;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use num_traits::ToPrimitive;
use crate::models::labourmodel::PaymentStatus;

use crate::{
    AppState, db::{labourdb::LaborExt, naira_walletdb::NairaWalletExt, userdb::UserExt, vendordb::VendorExt}, 
    dtos::vendordtos::ConfirmDeliveryDto, error::HttpError, 
    middleware::main_middleware::JWTAuthMiddeware, 
    models::{labourmodel::EscrowTransaction, usermodel::UserRole, vendormodels::*, walletmodels::*}, service::vendor_order_service::VendorOrderService
};
use crate::recommendation_models::{Interaction, FeedItemType, InteractionType};
use crate::services::reco_db::RecoDB;

pub fn vendor_handler() -> Router {
    Router::new()
        // Vendor Profile Management
        .route("/vendor/profile", post(create_vendor_profile).get(get_vendor_profile))
        .route("/vendor/profile", put(update_vendor_profile))
        
        // Subscription Management
        .route("/vendor/subscription/upgrade", post(upgrade_subscription))
        .route("/vendor/subscription/status", get(check_subscription_status))
        
        // Service/Product Management
        .route("/vendor/services", post(create_service).get(get_my_services))
        .route("/vendor/services/:service_id", 
            get(get_service_details)
            .put(update_service)
            .delete(delete_service)
        )
        .route("/vendor/services/:service_id/status", put(update_service_status))
        
        // Public Service Discovery
        .route("/services", get(search_services))
        .route("/services/recommended", get(get_recommended_services))
        .route("/services/:service_id", get(view_service_public))
        .route("/services/:service_id/inquiry", post(create_service_inquiry))
        
        // Vendor Dashboard
        .route("/vendor/inquiries", get(get_vendor_inquiries))
        .route("/vendor/analytics", get(get_vendor_analytics))
        .route("/services/:service_id/purchase", post(initiate_purchase))
        .route("/orders/:order_id", get(get_order_details))
        // .route("/orders/:order_id/confirm", post(confirm_order))
        .route("/orders/:order_id/complete", post(complete_order))
        .route("/orders/:order_id/cancel", post(cancel_order))
        
        // Admin Payment Verification
        .route("/admin/payments/verify", post(verify_service_purchase))
        
        // Buyer Orders
        .route("/orders/my-purchases", get(get_my_purchases))
        
        // Vendor Orders
        .route("/vendor/orders", get(get_vendor_orders_handler))
        .route("/vendor/orders/:order_id/confirm", post(vendor_confirm_order))
        .route("/orders/:order_id/delivery/confirm", post(confirm_delivery))
        
        // Reviews
        .route("/orders/:order_id/review", post(create_order_review))
        .route("/services/:service_id/reviews", get(get_service_reviews_handler))
}

// DTOs
// #[derive(Debug, Deserialize, Validate)]
// pub struct CreateVendorProfileDto {
//     #[validate(length(min = 2, max = 255))]
//     pub business_name: String,
//     pub description: Option<String>,
//     #[validate(length(min = 1))]
//     pub location_state: String,
//     #[validate(length(min = 1))]
//     pub location_city: String,
// }

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateVendorProfileDto {
    pub business_name: Option<String>,
    pub description: Option<String>,
    pub location_state: Option<String>,
    pub location_city: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpgradeSubscriptionDto {
    pub tier: SubscriptionTier,
    #[validate(range(min = 1, max = 12))]
    pub duration_months: i32,
    pub payment_reference: String,
}


#[derive(Debug, Deserialize)]
pub struct SearchServicesQuery {
    pub category: Option<ServiceCategory>,
    pub location_state: Option<String>,
    pub location_city: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub search: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateInquiryDto {
    #[validate(length(min = 10, max = 1000))]
    pub message: String,
}

// DTOs for Purchase Flow
#[derive(Debug, Deserialize, Validate)]
pub struct InitiatePurchaseDto {
    #[validate(range(min = 1))]
    pub quantity: i32,
    
    #[validate(length(min = 1))]
    pub buyer_name: String,
    
    #[validate(email)]
    pub buyer_email: String,
    
    pub buyer_phone: Option<String>,
    pub delivery_address: Option<String>,
    pub delivery_state: Option<String>,
    pub delivery_city: Option<String>,
    pub notes: Option<String>,
    
    // Payment method
    pub use_wallet: bool, // If true, deduct from wallet; if false, generate payment link
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateReviewDto {
    #[validate(range(min = 1, max = 5))]
    pub rating: i32,
    
    #[validate(length(max = 1000))]
    pub comment: Option<String>,
}

// Handler Functions

pub async fn create_vendor_profile(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateVendorProfileDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;
    
    // Check if user already has vendor profile
    if let Some(_) = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))? {
        return Err(HttpError::bad_request("Vendor profile already exists"));
    }
    
    let profile = app_state.db_client
        .create_vendor_profile(
            auth.user.id,
            body.business_name,
            body.description,
            body.location_state,
            body.location_city,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Add Vendor role to user
    let _ = app_state.db_client
        .update_user_role(auth.user.id, UserRole::Vendor)
        .await;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Vendor profile created successfully",
        "data": profile
    })))
}

pub async fn get_vendor_profile(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": profile
    })))
}

pub async fn upgrade_subscription(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<UpgradeSubscriptionDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;
    
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    // Verify payment was made (check wallet transaction)
    let amount = body.tier.monthly_price() * body.duration_months as f64;
    let tx = app_state.db_client
        .get_transaction_by_reference(&body.payment_reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::bad_request("Invalid payment reference"))?;
    
    if tx.user_id != auth.user.id {
        return Err(HttpError::unauthorized("Payment reference does not belong to you"));
    }
    
    let expected_amount = (amount * 100.0) as i64; // Convert to kobo
    if tx.amount != expected_amount {
        return Err(HttpError::bad_request("Payment amount mismatch"));
    }
    
    let (updated_profile, subscription) = app_state.db_client
        .upgrade_subscription(
            profile.id,
            body.tier,
            body.payment_reference,
            body.duration_months,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Send notification
    let _ = app_state.notification_service
        .notify_subscription_upgraded(auth.user.id, body.tier)
        .await;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": format!("Upgraded to {} tier successfully", body.tier.to_str()),
        "data": {
            "profile": updated_profile,
            "subscription": subscription
        }
    })))
}

pub async fn create_service(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateServiceDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Get vendor profile
    let vendor = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found. Create one first."))?;

    // Check subscription limits
    if let Some(max_services) = vendor.subscription_tier.max_services() {
        let current_services = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM vendor_services WHERE vendor_id = $1 AND status = 'active'"
        )
        .bind(vendor.id)
        .fetch_one(&app_state.db_client.pool)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

        if current_services >= max_services as i64 {
            return Err(HttpError::bad_request(
                format!("Service limit reached. Upgrade to list more services. Current limit: {}", max_services)
            ));
        }
    }

    let service = app_state.db_client
        .create_service(
            vendor.id,
            body.title,
            body.description,
            body.category,
            body.price,
            body.images.unwrap_or_default(),
            body.location_state,
            body.location_city,
            body.tags,
            body.stock_quantity.unwrap_or(1),
            body.is_negotiable.unwrap_or(false),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Service created successfully",
        "data": service
    })))
}


pub async fn get_my_services(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Query(params): Query<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    let status = params.get("status")
        .and_then(|s| s.as_str())
        .and_then(|s| match s {
            "active" => Some(ServiceStatus::Active),
            "paused" => Some(ServiceStatus::Paused),
            "sold" => Some(ServiceStatus::Sold),
            "expired" => Some(ServiceStatus::Expired),
            _ => None,
        });
    
    let services = app_state.db_client
        .get_vendor_services(profile.id, status)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "services": services,
            "total": services.len(),
            "subscription_tier": profile.subscription_tier,
            "max_services": profile.subscription_tier.max_services()
        }
    })))
}

pub async fn update_service(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(service_id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let service = app_state.db_client
        .get_service(service_id)
        .await?  // Using ? now works because of From<sqlx::Error>
        .ok_or_else(|| HttpError::not_found("Service not found"))?;
    
    // Verify ownership
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    if service.vendor_id != profile.id {
        return Err(HttpError::unauthorized("Not authorized to update this service"));
    }
    
    // Extract price properly
    let price = body.get("price").and_then(|v| v.as_f64());
    
    // Extract stock_quantity
    let stock_quantity = body.get("stock_quantity")
        .and_then(|v| v.as_i64())
        .map(|n| n as i32);
    
    // Extract is_negotiable
    let is_negotiable = body.get("is_negotiable").and_then(|v| v.as_bool());
    
    // Extract images correctly - convert to Vec<String>
    let images = body.get("images").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
    });
    
    // Extract tags correctly - convert to Vec<String>
    let tags = body.get("tags").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
    });
    
    let updated = app_state.db_client
        .update_service(
            service_id,
            body.get("title").and_then(|v| v.as_str()).map(String::from),
            body.get("description").and_then(|v| v.as_str()).map(String::from),
            price,
            images,
            tags,
            stock_quantity,
            is_negotiable,
        )
        .await?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Service updated successfully",
        "data": updated
    })))
}

pub async fn update_service_status(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(service_id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let service = app_state.db_client
        .get_service(service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Service not found"))?;
    
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    if service.vendor_id != profile.id {
        return Err(HttpError::unauthorized("Not authorized"));
    }
    
    let status_str = body.get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HttpError::bad_request("Status is required"))?;
    
    let status = match status_str {
        "active" => ServiceStatus::Active,
        "paused" => ServiceStatus::Paused,
        "sold" => ServiceStatus::Sold,
        "expired" => ServiceStatus::Expired,
        "removed" => ServiceStatus::Removed,
        _ => return Err(HttpError::bad_request("Invalid status")),
    };
    
    let updated = app_state.db_client
        .update_service_status(service_id, status)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": updated
    })))
}

pub async fn delete_service(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(service_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let service = app_state.db_client
        .get_service(service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Service not found"))?;
    
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    if service.vendor_id != profile.id {
        return Err(HttpError::unauthorized("Not authorized"));
    }
    
    app_state.db_client
        .delete_service(service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Service deleted successfully"
    })))
}

// Public Service Discovery
pub async fn search_services(
    Extension(app_state): Extension<Arc<AppState>>,
    Query(params): Query<SearchServicesQuery>,
) -> Result<impl IntoResponse, HttpError> {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20).min(100) as i64;
    let offset = ((page - 1) * limit as u32) as i64;
    
    let services = app_state.db_client
        .search_services(
            params.category,
            params.location_state,
            params.location_city,
            params.min_price,
            params.max_price,
            params.search,
            limit,
            offset,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": services,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": services.len()
        }
    })))
}

pub async fn get_recommended_services(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<Option<JWTAuthMiddeware>>,
    Query(params): Query<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let limit = params.get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(20)
        .min(50);
    
    let user_id = auth.as_ref().map(|a| a.user.id);
    
    let services = app_state.db_client
        .get_recommended_services(user_id, limit)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": services
    })))
}

pub async fn view_service_public(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<Option<JWTAuthMiddeware>>,
    Path(service_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let service = app_state.db_client
        .get_service(service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Service not found"))?;
    
    // Get vendor profile
    let vendor = sqlx::query_as::<_, VendorProfile>(
        "SELECT * FROM vendor_profiles WHERE id = $1"
    )
    .bind(service.vendor_id)
    .fetch_one(&app_state.db_client.pool)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Get vendor user info
    let vendor_user = app_state.db_client
        .get_user(Some(vendor.user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor user not found"))?;
    
    // Record view
    let viewer_id = auth.as_ref().map(|a| a.user.id);
    let session_id = uuid::Uuid::new_v4().to_string();
    
    let _ = app_state.db_client
        .record_service_view(service_id, viewer_id, session_id)
        .await;

    // Push a lightweight interaction into the recom pipeline for logged-in viewers
    if let Some(auth) = auth {
        let interaction = Interaction::new(auth.user.id, service_id, FeedItemType::Service, InteractionType::View, Some(1.0));
        let _ = RecoDB::new(app_state.db_client.clone()).push_event_stream(&interaction).await;
    }
    
    // Update user preferences if logged in
    if let Some(uid) = viewer_id {
        let _ = app_state.db_client
            .update_user_preferences(uid, service.category)
            .await;
    }
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "service": service,
            "vendor": {
                "id": vendor.id,
                "business_name": vendor.business_name,
                "location": format!("{}, {}", vendor.location_city, vendor.location_state),
                "rating": vendor.rating,
                "total_sales": vendor.total_sales,
                "subscription_tier": vendor.subscription_tier,
                "is_verified": vendor.is_verified,
                "contact": {
                    "name": vendor_user.name,
                    "email": vendor_user.email
                }
            }
        }
    })))
}

pub async fn create_service_inquiry(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(service_id): Path<Uuid>,
    Json(body): Json<CreateInquiryDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;
    
    let service = app_state.db_client
        .get_service(service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Service not found"))?;
    
    let inquiry = app_state.db_client
        .create_inquiry(
            service_id,
            service.vendor_id,
            auth.user.id,
            body.message,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Get vendor to notify
    let vendor = sqlx::query_as::<_, VendorProfile>(
        "SELECT * FROM vendor_profiles WHERE id = $1"
    )
    .bind(service.vendor_id)
    .fetch_one(&app_state.db_client.pool)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Send notification
    let _ = app_state.notification_service
        .notify_service_inquiry(vendor.user_id, &auth.user.name, &service.title)
        .await;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Inquiry sent successfully",
        "data": inquiry
    })))
}

pub async fn get_vendor_inquiries(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Query(params): Query<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    let status = params.get("status").and_then(|v| v.as_str()).map(String::from);
    
    let inquiries = app_state.db_client
        .get_vendor_inquiries(profile.id, status)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": inquiries
    })))
}

pub async fn get_vendor_analytics(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    // Get service stats
    let services = app_state.db_client
        .get_vendor_services(profile.id, Some(ServiceStatus::Active))
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    let total_views: i32 = services.iter()
        .map(|s| s.view_count.unwrap_or(0))
        .sum();
    
    let total_inquiries: i32 = services.iter()
        .map(|s| s.inquiry_count.unwrap_or(0))
        .sum();
    
    // Get subscription info
    let subscription_status = if profile.subscription_tier == SubscriptionTier::Normal {
        "active"
    } else if let Some(expires_at) = profile.subscription_expires_at {
        if expires_at > chrono::Utc::now() {
            "active"
        } else {
            "expired"
        }
    } else {
        "expired"
    };
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "subscription": {
                "tier": profile.subscription_tier,
                "status": subscription_status,
                "expires_at": profile.subscription_expires_at,
                "max_services": profile.subscription_tier.max_services()
            },
            "services": {
                "total": services.len(),
                "active": services.len(),
                "total_views": total_views,
                "total_inquiries": total_inquiries
            },
            "performance": {
                "rating": profile.rating,
                "total_sales": profile.total_sales,
                "is_verified": profile.is_verified
            }
        }
    })))
}

pub async fn get_service_details(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(service_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let service = app_state.db_client
        .get_service(service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Service not found"))?;
    
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    if service.vendor_id != profile.id {
        return Err(HttpError::unauthorized("Not authorized to view this service"));
    }
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": service
    })))
}

pub async fn check_subscription_status(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    let is_active = app_state.db_client
        .check_subscription_active(profile.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "tier": profile.subscription_tier,
            "is_active": is_active,
            "expires_at": profile.subscription_expires_at,
            "max_services": profile.subscription_tier.max_services(),
            "pricing": {
                "normal": { "price": 0.0, "max_services": 2 },
                "pro": { "price": 5000.0, "max_services": 5 },
                "premium": { "price": 12000.0, "max_services": "unlimited" }
            }
        }
    })))
}

pub async fn update_vendor_profile(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<UpdateVendorProfileDto>,
) -> Result<impl IntoResponse, HttpError> {
    let profile = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    let updated = app_state.db_client
        .update_vendor_profile(
            profile.id,
            body.business_name,
            body.description,
            body.location_state,
            body.location_city,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Vendor profile updated successfully",
        "data": updated
    })))
}


pub async fn initiate_purchase(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(service_id): Path<Uuid>,
    Json(body): Json<InitiatePurchaseDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;
    
    // Get service details
    let service = app_state.db_client
        .get_service(service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Service not found"))?;
    
    // Check service is active and in stock
    if service.status != Some(ServiceStatus::Active) {
        return Err(HttpError::bad_request("Service is not available"));
    }
    
    if service.stock_quantity < body.quantity {
        return Err(HttpError::bad_request(format!(
            "Insufficient stock. Only {} available", 
            service.stock_quantity
        )));
    }
    
    // Calculate amounts
    let unit_price = service.price.to_f64().unwrap_or(0.0);
    let subtotal = unit_price * body.quantity as f64;
    let platform_fee = subtotal * 0.05; // 5% platform fee
    let total_amount = subtotal + platform_fee;
    
    // Calculate transportation cost (10% of subtotal for physical delivery)
    let transportation_cost = if body.delivery_address.is_some() {
        subtotal * 0.10 // 10% for transportation
    } else {
        0.0 // No transportation for digital services
    };
    
    // Escrow amount = total amount - transportation cost
    let escrow_amount = total_amount - transportation_cost;
    
    // Generate payment reference
    let payment_reference = format!("SRV_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..16].to_uppercase());
    
    if body.use_wallet {
        // Deduct from wallet immediately
        let wallet = app_state.db_client
            .get_naira_wallet(auth.user.id)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::bad_request("Wallet not found. Please create a wallet first."))?;
        
        let total_kobo = (total_amount * 100.0) as i64;
        
        if wallet.balance < total_kobo {
            return Err(HttpError::bad_request(format!(
                "Insufficient wallet balance. Required: ₦{:.2}, Available: ₦{:.2}",
                total_amount,
                wallet.balance as f64 / 100.0
            )));
        }
        
        // Create order first
        let order = app_state.db_client
            .create_service_order(
                service_id,
                service.vendor_id,
                auth.user.id,
                body.quantity,
                unit_price,
                total_amount,
                platform_fee,
                payment_reference.clone(),
                body.buyer_name,
                body.buyer_email,
                body.buyer_phone,
                body.delivery_address.clone(),
                body.delivery_state.clone(),
                body.delivery_city.clone(),
                body.notes.clone(),
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Create escrow transaction for the main amount
        let escrow = app_state.db_client
            .create_escrow_transaction(
                order.id, // Use order_id as job_id for vendor services
                auth.user.id, // buyer is the employer in escrow context
                Some(service.vendor_id), // vendor is the worker in escrow context
                escrow_amount,
                platform_fee,
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Debit wallet for total amount
        let _ = app_state.db_client
            .debit_wallet(
                auth.user.id,
                total_kobo,
                TransactionType::ServicePayment,
                format!("Purchase: {}", service.title),
                payment_reference.clone(),
                None,
                Some(serde_json::json!({
                    "service_id": service_id,
                    "quantity": body.quantity,
                    "unit_price": unit_price,
                    "escrow_id": escrow.id,
                    "transportation_cost": transportation_cost,
                    "escrow_amount": escrow_amount
                })),
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Release transportation cost to vendor immediately if applicable
        if transportation_cost > 0.0 {
            let transport_kobo = (transportation_cost * 100.0) as i64;
            let _ = app_state.db_client
                .credit_wallet(
                    service.vendor_id,
                    transport_kobo,
                    TransactionType::ServicePayment,
                    format!("Transportation cost for order {}", order.order_number),
                    format!("TRANSPORT_{}", order.id),
                    None,
                    Some(serde_json::json!({
                        "order_id": order.id,
                        "service_id": service_id,
                        "buyer_id": auth.user.id
                    })),
                )
                .await
                .map_err(|e| HttpError::server_error(e.to_string()))?;
        }
        
        // Update order to paid (with escrow)
        let paid_order = app_state.db_client
            .update_order_status_with_escrow(order.id, "paid".to_string(), Some(escrow.id))
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Get vendor info
        let vendor = app_state.db_client
            .get_vendor_profile_by_id(service.vendor_id)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::not_found("Vendor not found"))?;
        
        // Notify vendor
        let _ = app_state.notification_service
            .notify_new_order(vendor.user_id, &service.title, total_amount, &paid_order.order_number)
            .await;
        
        // Notify buyer
        let _ = app_state.notification_service
            .notify_order_placed(auth.user.id, &service.title, total_amount, &paid_order.order_number)
            .await;
        
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Purchase completed with escrow protection",
            "data": {
                "order": paid_order,
                "payment_method": "wallet",
                "escrow_details": {
                    "escrow_id": escrow.id,
                    "escrow_amount": escrow_amount,
                    "transportation_cost": transportation_cost,
                    "total_escrowed": escrow_amount
                }
            }
        })))
        
    } else {
        // Generate payment link (Paystack/Flutterwave)
        let payment_service = crate::service::payment_provider::PaymentProviderService::new(&app_state.env);
        
        let payment_init = payment_service
            .initialize_payment(
                auth.user.email.clone(),
                total_amount,
                payment_reference.clone(),
                PaymentMethod::Card,
                Some(serde_json::json!({
                    "service_id": service_id,
                    "quantity": body.quantity,
                    "type": "service_purchase",
                    "escrow_amount": escrow_amount,
                    "transportation_cost": transportation_cost
                })),
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Create order with 'pending' status
        let order = app_state.db_client
            .create_service_order(
                service_id,
                service.vendor_id,
                auth.user.id,
                body.quantity,
                unit_price,
                total_amount,
                platform_fee,
                payment_reference.clone(),
                body.buyer_name,
                body.buyer_email,
                body.buyer_phone,
                body.delivery_address.clone(),
                body.delivery_state.clone(),
                body.delivery_city.clone(),
                body.notes.clone(),
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Create escrow transaction (will be funded after payment verification)
        let escrow = app_state.db_client
            .create_escrow_transaction(
                order.id, // Use order_id as job_id for vendor services
                auth.user.id, // buyer is the employer in escrow context
                Some(service.vendor_id), // vendor is the worker in escrow context
                escrow_amount,
                platform_fee,
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Update order to include escrow_id
        let updated_order = app_state.db_client
            .update_order_status_with_escrow(order.id, "pending".to_string(), Some(escrow.id))
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Payment initiated with escrow protection. Complete payment to confirm order.",
            "data": {
                "order": updated_order,
                "payment": payment_init,
                "escrow_details": {
                    "escrow_id": escrow.id,
                    "escrow_amount": escrow_amount,
                    "transportation_cost": transportation_cost,
                    "total_escrowed": escrow_amount
                }
            }
        })))
    }
}

// Verify payment and update order (webhook endpoint)
pub async fn verify_service_purchase(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<VerifyPaymentDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;
    
    // Only admins can verify payments (for manual verification)
    if auth.user.role != UserRole::Admin && auth.user.role != UserRole::SuperAdmin {
        return Err(HttpError::unauthorized("Not authorized to verify payments"));
    }
    
    // Get order
    let order = app_state.db_client
        .get_order_by_reference(&body.payment_reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Order not found"))?;
    
    if order.status == Some(OrderStatus::Paid) {
        return Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Order already paid",
            "data": order
        })));
    }
    
    // Verify with payment provider
    let payment_service = crate::service::payment_provider::PaymentProviderService::new(&app_state.env);
    let verification = payment_service
        .verify_payment(&body.payment_reference)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    if verification.status == "success" {
        // Get escrow details from order metadata or get order with escrow
        let escrow = if let Some(escrow_id) = order.escrow_id {
            app_state.db_client
                .get_escrow_transaction(escrow_id)
                .await
                .map_err(|e| HttpError::server_error(e.to_string()))?
                .ok_or_else(|| HttpError::not_found("Escrow transaction not found"))?
        } else {
            return Err(HttpError::bad_request("Order does not have escrow protection"));
        };
        
        // Calculate transportation cost (10% of service subtotal)
        let service = app_state.db_client
            .get_service(order.service_id)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::not_found("Service not found"))?;
        
        let subtotal = service.price.to_f64().unwrap_or(0.0) * order.quantity as f64;
        let transportation_cost = if order.delivery_address.is_some() {
            subtotal * 0.10 // 10% for transportation
        } else {
            0.0
        };
        
        // Update escrow status to funded
        let funded_escrow = app_state.db_client
            .update_escrow_status(escrow.id, crate::models::labourmodel::PaymentStatus::Funded, None)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Release transportation cost to vendor immediately if applicable
        if transportation_cost > 0.0 {
            let transport_kobo = (transportation_cost * 100.0) as i64;
            let _ = app_state.db_client
                .credit_wallet(
                    order.vendor_id,
                    transport_kobo,
                    TransactionType::ServicePayment,
                    format!("Transportation cost for order {}", order.order_number),
                    format!("TRANSPORT_{}", order.id),
                    None,
                    Some(serde_json::json!({
                        "order_id": order.id,
                        "service_id": order.service_id,
                        "buyer_id": order.buyer_id,
                        "escrow_id": escrow.id
                    })),
                )
                .await
                .map_err(|e| HttpError::server_error(e.to_string()))?;
        }
        
        // Update order to paid with escrow
        let paid_order = app_state.db_client
            .update_order_status(order.id, "paid".to_string())
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        let vendor = app_state.db_client
            .get_vendor_profile_by_id(order.vendor_id)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::not_found("Vendor not found"))?;
        
        // Notify vendor about payment and escrow
        let _ = app_state.notification_service
            .notify_new_order(
                vendor.user_id,
                &service.title,
                order.total_amount.to_f64().unwrap_or(0.0),
                &order.order_number,
            )
            .await;
        
        // Notify buyer about payment confirmation
        let _ = app_state.notification_service
            .notify_order_confirmed(
                order.buyer_id,
                &service.title,
                &order.order_number,
            )
            .await;
        
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Payment verified and escrow funded. Transportation cost released to vendor.",
            "data": {
                "order": paid_order,
                "escrow": funded_escrow,
                "transportation_cost": transportation_cost,
                "escrow_amount": escrow.amount
            }
        })))
    } else {
        Ok(Json(serde_json::json!({
            "status": "error",
            "message": "Payment verification failed",
            "details": verification
        })))
    }
}

pub async fn get_order_details(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(order_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let order = app_state.db_client
        .get_order_by_id(order_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Order not found"))?;
    
    // Get vendor profile
    let vendor = app_state.db_client
        .get_vendor_profile_by_user(order.vendor_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor not found"))?;
    
    // Verify access
    if order.buyer_id != auth.user.id && vendor.user_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to view this order"));
    }
    
    // Get service details
    let service = app_state.db_client
        .get_service(order.service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Service not found"))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "order": order,
            "service": {
                "id": service.id,
                "title": service.title,
                "images": service.images
            },
            "vendor": {
                "id": vendor.id,
                "business_name": vendor.business_name,
                "location": format!("{}, {}", vendor.location_city, vendor.location_state)
            }
        }
    })))
}

pub async fn vendor_confirm_order(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(order_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    // Start database transaction for atomic order confirmation
    let mut tx = app_state.db_client.pool.begin().await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Get order with row-level lock to prevent race conditions
    let order = sqlx::query_as::<_, ServiceOrder>(
        "SELECT * FROM service_orders WHERE id = $1 FOR UPDATE"
    )
    .bind(order_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?
    .ok_or_else(|| HttpError::not_found("Order not found"))?;
    
    let vendor = app_state.db_client
        .get_vendor_profile_by_user(order.vendor_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor not found"))?;
    
    if vendor.user_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized"));
    }
    
    if order.status != Some(OrderStatus::Paid) {
        return Err(HttpError::bad_request("Order must be paid before confirmation"));
    }
    
    // Update order status within transaction
    let confirmed_order = sqlx::query_as::<_, ServiceOrder>(
        "UPDATE service_orders SET status = 'confirmed', updated_at = NOW() WHERE id = $1 RETURNING *"
    )
    .bind(order_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Notify buyer (outside of transaction for performance)
    let service = app_state.db_client.get_service(order.service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Service not found"))?;
    let _ = app_state.notification_service
        .notify_order_confirmed(order.buyer_id, &service.title, &order.order_number)
        .await;
    
    // Commit the transaction
    tx.commit().await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Order confirmed",
        "data": confirmed_order
    })))
}

pub async fn complete_order(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(order_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    // Start database transaction for atomic order completion
    let mut tx = app_state.db_client.pool.begin().await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Get order with row-level lock to prevent race conditions
    let order = sqlx::query_as::<_, ServiceOrder>(
        "SELECT * FROM service_orders WHERE id = $1 FOR UPDATE"
    )
    .bind(order_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?
    .ok_or_else(|| HttpError::not_found("Order not found"))?;
    
    // Only buyer can mark as complete
    if order.buyer_id != auth.user.id {
        return Err(HttpError::unauthorized("Only buyer can complete order"));
    }
    
    if order.status != Some(OrderStatus::Confirmed) {
        return Err(HttpError::bad_request("Order must be confirmed before completion"));
    }
    
    // Get escrow details if available
    let escrow_release_amount = if let Some(escrow_id) = order.escrow_id {
        let escrow = sqlx::query_as::<_, crate::models::labourmodel::EscrowTransaction>(
            "SELECT id, job_id, employer_id, worker_id, amount, platform_fee, status, transaction_hash, created_at, released_at FROM escrow_transactions WHERE id = $1"
        )
        .bind(escrow_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Escrow transaction not found"))?;
        
        if escrow.status != Some(crate::models::labourmodel::PaymentStatus::Funded) {
            return Err(HttpError::bad_request("Escrow is not funded yet"));
        }
        
        // Calculate escrow release amount (escrow amount - platform fee)
        let platform_fee = escrow.amount.to_f64().unwrap_or(0.0) * 0.05; // 5% platform fee
        escrow.amount.to_f64().unwrap_or(0.0) - platform_fee
    } else {
        // Fallback to vendor_amount if no escrow
        order.vendor_amount.to_f64().unwrap_or(0.0)
    };
    
    // Update order status to completed within transaction
    let completed_order = sqlx::query_as::<_, ServiceOrder>(
        "UPDATE service_orders SET status = 'completed', completed_at = NOW(), updated_at = NOW() WHERE id = $1 RETURNING *"
    )
    .bind(order_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Release escrow to vendor if applicable
    if let Some(escrow_id) = order.escrow_id {
        // Update escrow status to completed
        sqlx::query(
            "UPDATE escrow_transactions SET status = $2, released_at = NOW() WHERE id = $1"
        )
        .bind(escrow_id)
        .bind(PaymentStatus::Completed)
        .execute(&mut *tx)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    }
    
    // Commit the transaction
    tx.commit().await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Credit vendor wallet with escrow release amount (outside transaction for performance)
    let vendor = app_state.db_client
        .get_vendor_profile_by_id(order.vendor_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor not found"))?;
    
    let vendor_amount_kobo = (escrow_release_amount * 100.0) as i64;
    
    let _ = app_state.db_client
        .credit_wallet(
            vendor.user_id,
            vendor_amount_kobo,
            TransactionType::ServicePayment,
            format!("Escrow release: Order {}", order.order_number),
            format!("ESCROW_RELEASE_{}", order.id),
            None,
            Some(serde_json::json!({
                "order_id": order.id,
                "service_id": order.service_id,
                "buyer_id": order.buyer_id,
                "escrow_id": order.escrow_id
            })),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Notify vendor of payment
    let service = app_state.db_client.get_service(order.service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Service not found"))?;
    
    let _ = app_state.notification_service
        .notify_order_completed(vendor.user_id, &service.title, escrow_release_amount)
        .await;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Order completed and escrow released to vendor",
        "data": {
            "order": completed_order,
            "escrow_release_amount": escrow_release_amount,
            "payment_method": "escrow"
        }
    })))
}

pub async fn cancel_order(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(order_id): Path<Uuid>,
    Json(_body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let order = app_state.db_client
        .get_order_by_id(order_id)
        .await?
        .ok_or_else(|| HttpError::not_found("Order not found"))?;
    
    // Only buyer or vendor can cancel
    let vendor = app_state.db_client
        .get_vendor_profile_by_id(order.vendor_id)
        .await?
        .ok_or_else(|| HttpError::not_found("Vendor not found"))?;
    
    let is_buyer = order.buyer_id == auth.user.id;
    let is_vendor = vendor.user_id == auth.user.id;
    
    if !is_buyer && !is_vendor {
        return Err(HttpError::unauthorized("Not authorized"));
    }
    
    // Can only cancel if not completed
    if order.status == Some(OrderStatus::Completed) {
        return Err(HttpError::bad_request("Cannot cancel completed order"));
    }
    
    let cancelled_order = app_state.db_client
        .update_order_status(order_id, "cancelled".to_string())
        .await?;
    
    // Refund buyer if paid - FIX: Use OrderStatus enum
    if order.status == Some(OrderStatus::Paid) 
        || order.status == Some(OrderStatus::Processing)
        || order.status == Some(OrderStatus::Shipped) {
        let total_kobo = (order.total_amount.to_f64().unwrap_or(0.0) * 100.0) as i64;
        
        let _ = app_state.db_client
            .credit_wallet(
                order.buyer_id,
                total_kobo,
                TransactionType::JobRefund,
                format!("Refund: Order {}", order.order_number),
                format!("REFUND_{}", order.id),
                None,
                Some(serde_json::json!({"order_id": order.id})),
            )
            .await?;
    }
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Order cancelled and refunded",
        "data": cancelled_order
    })))
}

pub async fn get_my_purchases(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Query(params): Query<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let status = params.get("status").and_then(|v| v.as_str()).map(String::from);
    
    let orders = app_state.db_client
        .get_buyer_orders(auth.user.id, status)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": orders
    })))
}

pub async fn get_vendor_orders_handler(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Query(params): Query<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let vendor = app_state.db_client
        .get_vendor_profile_by_user(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Vendor profile not found"))?;
    
    let status = params.get("status").and_then(|v| v.as_str()).map(String::from);
    
    let orders = app_state.db_client
        .get_vendor_orders(vendor.id, status)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": orders
    })))
}

pub async fn create_order_review(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(order_id): Path<Uuid>,
    Json(body): Json<CreateReviewDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;
    
    let order = app_state.db_client
        .get_order_by_id(order_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Order not found"))?;
    
    if order.buyer_id != auth.user.id {
        return Err(HttpError::unauthorized("Only buyer can review"));
    }
    
    if order.status != Some(OrderStatus::Completed) {
        return Err(HttpError::bad_request("Can only review completed orders"));
    }
    
    let review = app_state.db_client
        .create_service_review(
            order.service_id,
            order.vendor_id,
            Some(order_id),
            auth.user.id,
            body.rating,
            body.comment,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Review submitted successfully",
        "data": review
    })))
}

pub async fn get_service_reviews_handler(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(service_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let reviews = app_state.db_client
        .get_service_reviews(service_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": reviews
    })))
}

pub async fn confirm_delivery(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(order_id): Path<Uuid>,
    Json(body): Json<ConfirmDeliveryDto>,
) -> Result<impl IntoResponse, HttpError> {
    let order_service = VendorOrderService::new(
        app_state.db_client.clone(),
        app_state.notification_service.clone(),
    );

    let mut dto = body;
    dto.order_id = order_id;

    let order = order_service
        .confirm_delivery(auth.user.id, dto)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Delivery confirmed successfully",
        "data": order
    })))
}

pub async fn mark_order_shipped(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(order_id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let tracking_info = body.get("tracking_info")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let order_service = VendorOrderService::new(
        app_state.db_client.clone(),
        app_state.notification_service.clone(),
    );

    let order = order_service
        .mark_as_shipped(auth.user.id, order_id, tracking_info)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Order marked as shipped",
        "data": order
    })))
}

pub async fn create_service_dispute(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(order_id): Path<Uuid>,
    Json(body): Json<CreateServiceDisputeDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let order_service = VendorOrderService::new(
        app_state.db_client.clone(),
        app_state.notification_service.clone(),
    );

    let dispute = order_service
        .create_service_dispute(
            auth.user.id,
            order_id,
            body.reason,
            body.description,
            body.evidence_urls.unwrap_or_default(),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Dispute created successfully",
        "data": dispute
    })))
}

// Additional DTOs needed
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateVendorProfileDto {
    #[validate(length(min = 3, max = 100, message = "Business name must be between 3 and 100 characters"))]
    pub business_name: String,
    
    pub description: Option<String>,
    
    #[validate(length(min = 1, message = "State is required"))]
    pub location_state: String,
    
    #[validate(length(min = 1, message = "City is required"))]
    pub location_city: String,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateServiceDto {
    #[validate(length(min = 3, max = 200, message = "Title must be between 3 and 200 characters"))]
    pub title: String,
    
    #[validate(length(min = 20, max = 2000, message = "Description must be between 20 and 2000 characters"))]
    pub description: String,
    
    pub category: ServiceCategory,
    
    #[validate(range(min = 1.0, message = "Price must be positive"))]
    pub price: f64,
    
    pub images: Option<Vec<String>>,
    
    #[validate(length(min = 1, message = "State is required"))]
    pub location_state: String,
    
    #[validate(length(min = 1, message = "City is required"))]
    pub location_city: String,
    
    pub tags: Option<Vec<String>>,
    
    #[validate(range(min = 0, message = "Stock quantity must be non-negative"))]
    pub stock_quantity: Option<i32>,
    
    pub is_negotiable: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchServicesDto {
    pub category: Option<ServiceCategory>,
    pub location_state: Option<String>,
    pub location_city: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub search_query: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateServiceDisputeDto {
    #[validate(length(min = 5, max = 100, message = "Reason must be between 5 and 100 characters"))]
    pub reason: String,
    
    #[validate(length(min = 20, max = 2000, message = "Description must be between 20 and 2000 characters"))]
    pub description: String,
    
    pub evidence_urls: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct VerifyPaymentDto {
    #[validate(length(min = 1, message = "Payment reference is required"))]
    pub payment_reference: String,
}

pub async fn calculate_delivery_fee(
    vendor_state: &str,
    vendor_city: &str,
    buyer_state: &str,
    buyer_city: &str,
    service_price: f64,
) -> f64 {
    if vendor_state == buyer_state {
        // Same state - minimal delivery fee
        service_price * 0.02 // 2% for same state
    } else {
        // Cross-state delivery - higher fee
        service_price * 0.08 // 8% for cross-state
    }
}


#[derive(Clone)]
pub struct RateLimiter {
    attempts: Arc<Mutex<HashMap<Uuid, (usize, DateTime<Utc>)>>>,
}

impl RateLimiter {
    pub async fn check_rate_limit(&self, user_id: Uuid, max_attempts: usize, window_seconds: i64) -> bool {
        let mut attempts = self.attempts.lock().await;
        let now = Utc::now();
        
        if let Some((count, last_attempt)) = attempts.get(&user_id) {
            if now.signed_duration_since(*last_attempt).num_seconds() < window_seconds {
                if *count >= max_attempts {
                    return false;
                }
            }
        }
        
        let count = attempts.get(&user_id).map_or(1, |(c, _)| c + 1);
        attempts.insert(user_id, (count, now));
        true
    }
}