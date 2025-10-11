use sqlx::types::chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::{Json, BigDecimal}};
use uuid::Uuid;
use serde_json::Value as JsonValue;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "property_status", rename_all = "lowercase")]
pub enum PropertyStatus {
    Draft, // Landlord is creating
    AwaitingAgent,  //Awaiting agent to verify
    AgentVerified,
    AwaitingLawyer,
    LawyerVerified,
    Active,
    Suspended,  //Temporarily disabled
    Rejected, //Failed Verification
    Sold,
    Rented,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "property_type", rename_all = "lowercase")]
pub enum PropertyType {
    Apartment,
    House,
    Duplex,
    Bungalow,
    Commercial,
    Land,
    Warehouse,
    Office,
    Shop,
    Hotel,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "listing_type", rename_all = "lowercase")]
pub enum ListingType {
    Sale,
    Rent,
    Lease,
    Asset,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]     //
#[sqlx(type_name = "currency_type", rename_all = "lowercase")]
pub enum CurrencyType {
    Naira,
    Usd,
    Vern,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Property {
    pub id: Uuid,
    pub landlord_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub lawyer_id: Option<Uuid>,

    // Basic property info
    pub title: String,
    pub description: String,
    pub property_type: PropertyType,
    pub listing_type: ListingType,

    // Location Details
    pub address: String,
    pub city: String,
    pub state: String,
    pub lga: String,
    pub country: String,
    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
    pub landmark: Option<String>,

    // Property Specifications
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub toilets: Option<i32>,
    pub size_sqm: Option<BigDecimal>,
    pub plot_size: Option<String>,

    // Pricing
    pub price: i64,
    pub currency: CurrencyType,
    pub price_negotiable: Option<bool>,
    pub bidding_price: Option<i64>,

    // Property Features - Use JsonValue instead of serde_json::Value
    pub amenities: Option<JsonValue>,
    pub features: Option<JsonValue>,

    // Legal Information
    pub certificate_of_occupancy: Option<String>,
    pub deed_of_agreement: Option<String>,
    pub survey_plan: Option<String>,
    pub building_plan_approval: Option<String>,

    // Verification Details
    pub property_photos: JsonValue, // Changed to JsonValue
    pub agent_verification_photos: Option<JsonValue>,
    pub agent_verification_notes: Option<String>,
    pub lawyer_verification_notes: Option<String>,

    // Unique Identifiers
    pub property_hash: String,
    pub coordinates_hash: String,

    pub status: PropertyStatus,
    pub agent_verified_at: Option<DateTime<Utc>>,
    pub lawyer_verified_at: Option<DateTime<Utc>>,
    pub listed_at: Option<DateTime<Utc>>,

    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PropertyVerification {
    pub id: Uuid,
    pub property_id: Uuid,
    pub verifier_id: Uuid,
    pub verifier_type: String,
    pub verification_status: String,
    pub notes: String,
    pub verification_photos: Option<JsonValue>, // Use serde_json::Value
    pub created_at: Option<DateTime<Utc>>
}