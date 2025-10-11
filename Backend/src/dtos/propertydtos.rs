use serde::{Serialize, Deserialize};
use uuid::Uuid;
use validator::Validate;
use chrono::{DateTime, Utc};
use crate::models::propertymodel::{
    CurrencyType, ListingType, Property, PropertyType
};
use sqlx::types::BigDecimal;
use serde_json::Value as JsonValue;


#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreatePropertyDto {
    #[validate(length(min = 10, max = 200, message = "Title must be between 10 and 200 characters"))]
    pub title: String,

    #[validate(length(min = 50, max = 2000, message = "Description must be between 50 and 2000 characters"))]
    pub description: String,

    pub property_type: PropertyType,
    pub listing_type: ListingType,

    //Location
    #[validate(length(min = 10, max = 500, message = "Address must be between 10 and 500 characters"))]
    pub address: String,

    #[validate(length(min = 2, max = 100, message = "City is required"))]
    pub city: String,

    #[validate(length(min = 2, max = 100, message = "State is required"))]
    pub state: String,

    #[validate(length(min = 2, max = 100, message = "LGA is required"))]
    pub lga: String,

    #[validate(length(min = 2, max = 100, message = "Country is required"))]
    pub country: String,

    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
    pub landmark: Option<String>,

    //Specifications
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub toilets: Option<i32>,
    pub size_sqm: Option<BigDecimal>,
    pub plot_size: Option<String>,

    //Pricing
    #[validate(range(min = 1000, message = "Price must be at least 1000"))]
    pub price: i64,
    pub currency: CurrencyType,
    pub price_negotiable: Option<bool>,

    //features
    pub amenities: Option<Vec<String>>,
    pub features: Option<Vec<String>>,

    //Document (URLs after uploads )
    pub certificate_of_occupancy: Option<String>,
    pub deed_of_agreement: Option<String>,
    pub survey_plan: Option<String>,
    pub building_plan_approval: Option<String>,

    //Photos (URLs after uploads)
    #[validate(length(min = 1, message = "At least one property photo is required"))]
    pub property_photos: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct AgentVerificationDto {
    pub property_id: Uuid,
    pub verification_status: String,  //"approved" || "rejected"
    pub notes: String,
    pub verification_photos: Vec<String>, //URLs of verification photos
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct LawyerVerificationDto {
    pub property_id: Uuid,
    pub verification_status: String,  //"approved" || "rejected"
    pub notes: String,
    pub document_issues: Option<Vec<String>>,   //List of documents with issues if any
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PropertyFilterDto {
    pub id: Uuid,
    pub title: String,
    pub property_type: String,
    pub listing_type: String,
    pub address: String,
    pub landmark: Option<String>,
    pub city: String,
    pub state: String,
    pub country: String,
    pub price: i64,
    pub bidding_price: Option<i64>,
    pub currency: String,
    pub status: String,
    pub bedroom: Option<i32>,
    pub bathrooms: Option<i32>,
    pub landlord_username: String,
    pub property_photo: JsonValue,
    pub created_at: DateTime<Utc>,
}

impl PropertyFilterDto {
    pub fn from_property(property: &Property, landlord_username: String) -> Self {
        Self { 
            id: property.id, 
            title: property.title.clone(), 
            property_type: format!("{:?}", property.property_type), 
            listing_type: format!("{:?}", property.listing_type), 
            address: property.address.clone(),
            landmark: property.landmark.clone(), 
            city: property.city.clone(), 
            state: property.state.clone(), 
            country: property.country.clone(), 
            price: property.price,
            bidding_price: property.bidding_price, 
            currency: format!("{:?}", property.currency), 
            status: format!("{:?}", property.status), 
            bedroom: property.bedrooms, 
            bathrooms: property.bathrooms, 
            landlord_username, 
            property_photo: property.property_photos.clone(), 
            created_at: property.created_at.expect("Created must be set")
        }
    }
}