use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;
use crate::models::vendormodels::*;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateServiceOrderDto {
    pub service_id: Uuid,
    pub quantity: i32,
    
    #[validate(length(min = 1, message = "Buyer name is required"))]
    pub buyer_name: String,
    
    #[validate(email(message = "Invalid email address"))]
    pub buyer_email: String,
    
    pub buyer_phone: Option<String>,
    
    pub delivery_type: DeliveryType,
    
    // Required for cross-state delivery
    pub delivery_address: Option<String>,
    pub delivery_state: Option<String>,
    pub delivery_city: Option<String>,
    
    pub notes: Option<String>,
    
    // Security
    pub transaction_pin: Option<String>,
    pub email_otp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceOrderResponse {
    pub order: ServiceOrder,
    pub service: VendorService,
    pub vendor: VendorProfileSummary,
    pub payment_info: PaymentInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VendorProfileSummary {
    pub id: Uuid,
    pub business_name: String,
    pub rating: Option<f32>,
    pub total_sales: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentInfo {
    pub total_amount: f64,
    pub platform_fee: f64,
    pub delivery_fee: f64,
    pub vendor_amount: f64,
    pub held_amount: f64, // Amount held for delivery confirmation
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmDeliveryDto {
    pub order_id: Uuid,
    pub rating: Option<i32>,
    pub review_comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DisputeResolution {
    #[serde(rename = "full_refund")]
    FullRefund,
    
    #[serde(rename = "partial_refund")]
    PartialRefund {
        /// Percentage to refund to buyer (1-100)
        percentage: u32,
    },
    
    #[serde(rename = "dismissed")]
    Dismissed,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateServiceDisputeDto {
    pub order_id: Uuid,
    pub reason: String,
    
    #[validate(length(min = 10, message = "Description must be at least 10 characters"))]
    pub description: String,
    
    pub evidence_urls: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettleDisputeDto {
    pub dispute_id: Uuid,
    pub resolution: DisputeResolution,
}