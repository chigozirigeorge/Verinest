// models/vendormodels.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::types::BigDecimal;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "subscription_tier", rename_all = "snake_case")]
pub enum SubscriptionTier {
    Normal,   // Free - 2 services
    Pro,      // ₦5,000/month - 5 services + higher visibility + pro badge
    Premium,  // ₦12,000/month - unlimited + highest visibility + premium badge
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, PartialEq, Clone)]
#[sqlx(type_name = "delivery_type", rename_all = "snake_case")]
pub enum DeliveryType {
    LocalPickup,
    CrossStateDelivery,
    DigitalDelivery,
}

impl SubscriptionTier {
    pub fn to_str(&self) -> &str {
        match self {
            SubscriptionTier::Normal => "normal",
            SubscriptionTier::Pro => "pro",
            SubscriptionTier::Premium => "premium",
        }
    }

    pub fn max_services(&self) -> Option<i32> {
        match self {
            SubscriptionTier::Normal => Some(2),
            SubscriptionTier::Pro => Some(5),
            SubscriptionTier::Premium => None, // Unlimited
        }
    }

    pub fn monthly_price(&self) -> f64 {
        match self {
            SubscriptionTier::Normal => 0.0,
            SubscriptionTier::Pro => 5000.0,
            SubscriptionTier::Premium => 12000.0,
        }
    }

    pub fn visibility_boost(&self) -> f64 {
        match self {
            SubscriptionTier::Normal => 1.0,
            SubscriptionTier::Pro => 2.5,
            SubscriptionTier::Premium => 5.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "service_status", rename_all = "snake_case")]
pub enum ServiceStatus {
    Active,
    Paused,
    Sold,
    Expired,
    Removed,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "service_category", rename_all = "snake_case")]
pub enum ServiceCategory {
    Electronics,
    HomeAppliances,
    Fashion,
    Beauty,
    FoodDrinks,
    Health,
    Sports,
    Books,
    Toys,
    Automotive,
    RealEstate,
    Services,
    Other,
}

impl ServiceCategory {
    pub fn to_str(&self) -> &str {
        match self {
            ServiceCategory::Electronics => "electronics",
            ServiceCategory::HomeAppliances => "home_appliances",
            ServiceCategory::Fashion => "fashion",
            ServiceCategory::Beauty => "beauty",
            ServiceCategory::FoodDrinks => "food_drinks",
            ServiceCategory::Health => "health",
            ServiceCategory::Sports => "sports",
            ServiceCategory::Books => "books",
            ServiceCategory::Toys => "toys",
            ServiceCategory::Automotive => "automotive",
            ServiceCategory::RealEstate => "real_estate",
            ServiceCategory::Services => "services",
            ServiceCategory::Other => "other",
        }
    }
}

// Vendor Profile
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct VendorProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub business_name: String,
    pub description: Option<String>,
    pub location_state: String,
    pub location_city: String,
    pub subscription_tier: SubscriptionTier,
    pub subscription_expires_at: Option<DateTime<Utc>>,
    pub is_verified: Option<bool>,
    pub total_sales: Option<i32>,
    pub rating: Option<f32>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// Vendor Service/Product Listing
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct VendorService {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub title: String,
    pub description: String,
    pub category: ServiceCategory,
    pub price: BigDecimal,
    pub images: Option<Vec<String>>,
    pub status: Option<ServiceStatus>,
    pub stock_quantity: i32,
    pub view_count: Option<i32>,
    pub inquiry_count: Option<i32>,
    pub location_state: String,
    pub location_city: String,
    pub tags: Option<Vec<String>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

// Vendor Subscription Payments
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct VendorSubscription {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub tier: SubscriptionTier,
    pub amount: BigDecimal,
    pub payment_reference: String,
    pub starts_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_active: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
}

// Service Views/Impressions (for analytics & algorithm)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServiceView {
    pub id: Uuid,
    pub service_id: Uuid,
    pub viewer_id: Option<Uuid>, // Null for anonymous
    pub viewed_at: DateTime<Utc>,
    pub session_id: String,
}

// Service Inquiries (messages/interest)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServiceInquiry {
    pub id: Uuid,
    pub service_id: Uuid,
    pub vendor_id: Uuid,
    pub inquirer_id: Uuid,
    pub message: String,
    pub status: Option<String>, // pending, responded, closed
    pub created_at: Option<DateTime<Utc>>,
}

// User Service Preferences (for recommendation algorithm)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserServicePreference {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category: ServiceCategory,
    pub preference_score: f32, // Higher = more interested
    pub last_viewed_at: DateTime<Utc>,
    pub view_count: i32,
}

#[derive(Debug, Serialize, Clone, Deserialize, sqlx::FromRow)]
pub struct ServiceOrder {
    pub id: Uuid,
    pub order_number: String,
    pub service_id: Uuid,
    pub vendor_id: Uuid,
    pub buyer_id: Uuid,
    pub quantity: i32,
    pub unit_price: BigDecimal,
    pub total_amount: BigDecimal,
    pub platform_fee: BigDecimal,
    pub vendor_amount: BigDecimal,
    pub payment_reference: String,
    pub status: Option<OrderStatus>,
    pub buyer_name: String,
    pub buyer_email: String,
    pub buyer_phone: Option<String>,
    pub delivery_type: DeliveryType,
    pub delivery_fee: Option<BigDecimal>,
    pub delivery_amount_held: Option<BigDecimal>, // Amount held for delivery confirmation
    pub delivery_confirmed: Option<bool>,
    pub delivery_confirmed_at: Option<DateTime<Utc>>,
    pub delivery_address: Option<String>,
    pub delivery_state: Option<String>,
    pub delivery_city: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub escrow_id: Option<Uuid>, // Link to escrow transaction
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServiceReview {
    pub id: Uuid,
    pub service_id: Uuid,
    pub vendor_id: Uuid,
    pub order_id: Option<Uuid>,
    pub reviewer_id: Uuid,
    pub rating: i32,
    pub comment: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "order_status", rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,           // Payment pending
    Paid,              // Payment received, processing
    Confirmed,         // Order confirmed by vendor
    Processing,        // Vendor is preparing order
    Shipped,           // Order shipped (for cross-state)
    InTransit,         // In delivery
    Delivered,         // Delivered, awaiting confirmation
    Completed,         // Buyer confirmed receipt
    Disputed,          // Dispute raised
    Cancelled,         // Order cancelled
    Refunded,          // Money refunded
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeliveryTracking {
    pub id: Uuid,
    pub order_id: Uuid,
    pub status: String,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub updated_by: Uuid,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServiceDispute {
    pub id: Uuid,
    pub order_id: Uuid,
    pub service_id: Uuid,
    pub raised_by: Uuid,
    pub against: Uuid,
    pub reason: String,
    pub description: String,
    pub evidence_urls: Option<Vec<String>>,
    pub status: String, // open, under_review, resolved
    pub assigned_verifier: Option<Uuid>,
    pub resolution: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}