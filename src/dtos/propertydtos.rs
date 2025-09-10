use serde::{Serialize, Deserialize};
use uuid::Uuid;
use validator::Validate;

use crate::db::propertydb::{
    PropertyType, ListingType,
    CurrencyType,
};


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

    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub landmark: Option<String>,

    //Specifications
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub toilets: Option<i32>,
    pub size_sqm: Option<f64>,
    pub plot_size: Option<String>,

    //Pricing
    #[validate(range(min = 1000, message = "Price must be at least 1000"))]
    pub price: f64,
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



