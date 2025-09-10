use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json};
use uuid::Uuid;
use validator::Validate;

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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "currency_type", rename_all = "lowercase")]
pub enum CurrencyType {
    Naira,
    Usd,
    Vern,
}

#[derive(Debug, Serialize, Deserialize, FromRow, sqlx::Type, Clone)]
pub struct Property {
    pub id: Uuid,
    pub landlord_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub lawyer_id: Option<Uuid>,

    //Basic property info
    pub title: String,
    pub description: String,
    pub property_type: PropertyType,
    pub listing_type: ListingType,

    //Location Details(uniqueness checking)
    pub address: String,
    pub city: String,
    pub state: String,
    pub lga: String,
    pub country: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub landmark: Option<String>,

    //property Specifications
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub toilets: Option<i32>,
    pub size_sqm: Option<f64>,
    pub plot_size: Option<String>,

    //Pricing
    pub price: i64,
    pub currency: CurrencyType,
    pub price_negotiable: bool,
    pub biding_price: Option<i64>,

    //Property Features
    pub amenities: Option<Json<Vec<String>>>,
    pub features: Option<Json<Vec<String>>>,

    //Legal Information
    pub certification_of_occupancy: Option<String>,  //Document URL
    pub deed_of_agreement: Option<String>,          //Document URL
    pub survey_plan: Option<String>,                 //Document URL
    pub building_plan_approval: Option<String>,      //Document URL

    //Verification Details
    pub property_photos: Json<Vec<String>>,    //Photo URLs
    pub agent_verification_photos: Option<Json<Vec<String>>>,
    pub agent_verification_notes: Option<String>,
    pub lawyer_verification_notes: Option<String>,

    //Unique Identifiers to prevent fraud/Duplicate listings
    pub property_hash: String, //Hash of key identifying fields,
    pub coordinates_hash: String,   //Hash of lat/lng for location uniqueness


    pub status: PropertyStatus,
    pub agent_verified_at: Option<DateTime<Utc>>,
    pub lawyer_verified_at: Option<DateTime<Utc>>,
    pub listed_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

//Property Verification Records
#[derive(Debug,Serialize, Deserialize, FromRow)]
pub struct PropertyVerification {
    pub id: Uuid,
    pub property_id: Uuid,
    pub verifier_id: Uuid,  
    pub verifier_type: String,    //Agent or Lawyer
    pub verification_status: String, //"approved", "rejected", "pending"
    pub notes: String,
    pub verification_photos: Option<Json<Vec<String>>>,
    pub created_at: DateTime<Utc>
}
