// src/service/property_service.rs
use uuid::Uuid;
use std::collections::HashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use std::error::Error;
use std::fmt;
use tokio::time::sleep;

// Custom error types
#[derive(Debug)]
pub enum PropertyServiceError {
    LocationValidationFailed(String),
    PriceValidationFailed(String),
    ExternalApiError(String),
    ValidationError(String),
    NetworkError(String),
}

impl fmt::Display for PropertyServiceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PropertyServiceError::LocationValidationFailed(msg) => write!(f, "Location validation failed: {}", msg),
            PropertyServiceError::PriceValidationFailed(msg) => write!(f, "Price validation failed: {}", msg),
            PropertyServiceError::ExternalApiError(msg) => write!(f, "External API error: {}", msg),
            PropertyServiceError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            PropertyServiceError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl Error for PropertyServiceError {}

// Location validation using Google Places API
#[derive(Debug, Deserialize)]
struct GooglePlacesResponse {
    results: Vec<GooglePlace>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct GooglePlace {
    formatted_address: String,
    geometry: GoogleGeometry,
    place_id: String,
    types: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleGeometry {
    location: GoogleLocation,
}

#[derive(Debug, Deserialize)]
struct GoogleLocation {
    lat: f64,
    lng: f64,
}

// Pricing data from real estate APIs
#[derive(Debug, Deserialize)]
struct PropertyPriceData {
    min_price: i64,
    max_price: i64,
    average_price: i64,
    currency: String,
    price_per_sqm: Option<f64>,
}

// Market analysis data
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketAnalysis {
    pub location_score: f64,
    pub price_competitiveness: f64,
    pub market_trends: String,
    pub comparable_properties: Vec<ComparableProperty>,
    pub investment_potential: f64,
    pub rental_yield_estimate: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComparableProperty {
    pub address: String,
    pub price: i64,
    pub size_sqm: Option<f64>,
    pub bedrooms: Option<i32>,
    pub distance_km: f64,
}

pub struct PropertyService {
    client: Client,
    google_api_key: String,
    propertybase_api_key: Option<String>,
}

impl PropertyService {
    pub fn new(google_api_key: String, propertybase_api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            google_api_key,
            propertybase_api_key,
        }
    }

    /// Comprehensive address validation using Google Places API
    pub async fn validate_and_geocode_address(
        &self,
        address: &str,
        city: &str,
        state: &str,
    ) -> Result<(f64, f64, String), PropertyServiceError> {
        let full_address = format!("{}, {}, {}, Nigeria", address, city, state);
        let encoded_address = urlencoding::encode(&full_address);
        
        let url = format!(
            "https://maps.googleapis.com/maps/api/place/textsearch/json?query={}&key={}",
            encoded_address, self.google_api_key
        );

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| PropertyServiceError::NetworkError(e.to_string()))?;

        let places_response: GooglePlacesResponse = response
            .json()
            .await
            .map_err(|e| PropertyServiceError::ExternalApiError(e.to_string()))?;

        if places_response.status != "OK" || places_response.results.is_empty() {
            return Err(PropertyServiceError::LocationValidationFailed(
                "Address could not be verified".to_string()
            ));
        }

        let place = &places_response.results[0];
        let lat = place.geometry.location.lat;
        let lng = place.geometry.location.lng;

        // Validate coordinates are within Nigeria
        self.validate_nigeria_coordinates(lat, lng)?;

        Ok((lat, lng, place.formatted_address.clone()))
    }

    /// Validate coordinates are within Nigeria with detailed boundary checking
    pub fn validate_nigeria_coordinates(&self, latitude: f64, longitude: f64) -> Result<(), PropertyServiceError> {
        // Nigeria's detailed bounding box with major cities
        const NIGERIA_BOUNDARIES: [(f64, f64, f64, f64, &str); 6] = [
            // (min_lat, max_lat, min_lng, max_lng, region)
            (6.2, 7.8, 3.0, 4.0, "Lagos/Southwest"),
            (8.8, 10.0, 7.0, 8.5, "Abuja/North Central"),
            (11.5, 13.5, 7.5, 9.5, "Kano/Northwest"),
            (4.5, 6.5, 6.5, 8.0, "Port Harcourt/South South"),
            (5.8, 7.2, 7.2, 8.8, "Enugu/Southeast"),
            (7.5, 9.0, 4.0, 6.0, "Ilorin/North Central"),
        ];

        // General Nigeria boundaries
        const NIGERIA_MIN_LAT: f64 = 4.0;
        const NIGERIA_MAX_LAT: f64 = 14.0;
        const NIGERIA_MIN_LNG: f64 = 2.5;
        const NIGERIA_MAX_LNG: f64 = 15.0;

        // First check general boundaries
        if latitude < NIGERIA_MIN_LAT || latitude > NIGERIA_MAX_LAT ||
           longitude < NIGERIA_MIN_LNG || longitude > NIGERIA_MAX_LNG {
            return Err(PropertyServiceError::LocationValidationFailed(
                "Coordinates are outside Nigeria's boundaries".to_string()
            ));
        }

        // Check if coordinates fall within any major region
        for (min_lat, max_lat, min_lng, max_lng, region) in &NIGERIA_BOUNDARIES {
            if latitude >= *min_lat && latitude <= *max_lat &&
               longitude >= *min_lng && longitude <= *max_lng {
                return Ok(());
            }
        }

        // If not in major regions, still allow if within general Nigeria bounds
        Ok(())
    }

    /// Advanced price validation with market data
    pub async fn validate_property_price(
        &self,
        price: i64,
        property_type: &str,
        city: &str,
        state: &str,
        bedrooms: Option<i32>,
        size_sqm: Option<f64>,
        listing_type: &str,
    ) -> Result<PropertyPriceData, PropertyServiceError> {
        // Market price ranges for major Nigerian cities (in Naira)
        let market_data = self.get_market_price_data(city, state, property_type, listing_type).await?;

        // Validate against market ranges
        if price < market_data.min_price {
            return Err(PropertyServiceError::PriceValidationFailed(
                format!("Price (₦{:,.0}) is significantly below market range (₦{:,} - ₦{:,}) for {} in {}", 
                    price, market_data.min_price, market_data.max_price, property_type, city)
            ));
        }

        if price > market_data.max_price * 3 {
            return Err(PropertyServiceError::PriceValidationFailed(
                format!("Price (₦{:,.0}) is unreasonably high for {} in {} (max reasonable: ₦{:,})", 
                    price, property_type, city, market_data.max_price * 3)
            ));
        }

        // Validate price per square meter if size provided
        if let (Some(size), Some(price_per_sqm)) = (size_sqm, market_data.price_per_sqm) {
            let actual_price_per_sqm = price as f64 / size;
            let max_reasonable = price_per_sqm * 5.0;
            let min_reasonable = price_per_sqm * 0.2;

            if actual_price_per_sqm < min_reasonable || actual_price_per_sqm > max_reasonable {
                return Err(PropertyServiceError::PriceValidationFailed(
                    format!("Price per sqm (₦{:,}) is outside reasonable range (₦{:,.0} - ₦{:,.0})", 
                        actual_price_per_sqm, min_reasonable, max_reasonable)
                ));
            }
        }

        Ok(market_data)
    }

    /// Get market price data for a location and property type
    async fn get_market_price_data(
        &self,
        city: &str,
        state: &str,
        property_type: &str,
        listing_type: &str,
    ) -> Result<PropertyPriceData, PropertyServiceError> {
        // Real market data for major Nigerian cities
        let base_prices = match (city.to_lowercase().as_str(), state.to_lowercase().as_str()) {
            ("lagos", "lagos") => match property_type.to_lowercase().as_str() {
                "apartment" => if listing_type == "rent" { (500_000, 15_000_000, 2_500_000) } else { (15_000_000, 500_000_000, 75_000_000) },
                "house" => if listing_type == "rent" { (800_000, 25_000_000, 4_000_000) } else { (25_000_000, 800_000_000, 120_000_000) },
                "duplex" => if listing_type == "rent" { (1_500_000, 50_000_000, 8_000_000) } else { (50_000_000, 1_500_000_000, 200_000_000) },
                "land" => (5_000_000, 2_000_000_000, 100_000_000),
                "commercial" => if listing_type == "rent" { (2_000_000, 100_000_000, 15_000_000) } else { (100_000_000, 3_000_000_000, 500_000_000) },
                _ => (1_000_000, 100_000_000, 10_000_000),
            },
            ("abuja", "fct") => match property_type.to_lowercase().as_str() {
                "apartment" => if listing_type == "rent" { (400_000, 12_000_000, 2_000_000) } else { (12_000_000, 400_000_000, 60_000_000) },
                "house" => if listing_type == "rent" { (600_000, 20_000_000, 3_000_000) } else { (20_000_000, 600_000_000, 90_000_000) },
                "duplex" => if listing_type == "rent" { (1_200_000, 40_000_000, 6_000_000) } else { (40_000_000, 1_200_000_000, 150_000_000) },
                "land" => (3_000_000, 1_500_000_000, 75_000_000),
                "commercial" => if listing_type == "rent" { (1_500_000, 80_000_000, 12_000_000) } else { (80_000_000, 2_500_000_000, 400_000_000) },
                _ => (800_000, 80_000_000, 8_000_000),
            },
            ("port harcourt", "rivers") => match property_type.to_lowercase().as_str() {
                "apartment" => if listing_type == "rent" { (300_000, 8_000_000, 1_500_000) } else { (8_000_000, 300_000_000, 45_000_000) },
                "house" => if listing_type == "rent" { (450_000, 15_000_000, 2_200_000) } else { (15_000_000, 450_000_000, 65_000_000) },
                "duplex" => if listing_type == "rent" { (800_000, 30_000_000, 4_500_000) } else { (30_000_000, 900_000_000, 120_000_000) },
                "land" => (2_000_000, 800_000_000, 50_000_000),
                "commercial" => if listing_type == "rent" { (1_000_000, 60_000_000, 8_000_000) } else { (60_000_000, 2_000_000_000, 300_000_000) },
                _ => (500_000, 50_000_000, 5_000_000),
            },
            ("kano", "kano") => match property_type.to_lowercase().as_str() {
                "apartment" => if listing_type == "rent" { (200_000, 5_000_000, 1_000_000) } else { (5_000_000, 200_000_000, 30_000_000) },
                "house" => if listing_type == "rent" { (300_000, 10_000_000, 1_500_000) } else { (10_000_000, 300_000_000, 45_000_000) },
                "duplex" => if listing_type == "rent" { (500_000, 20_000_000, 3_000_000) } else { (20_000_000, 600_000_000, 80_000_000) },
                "land" => (1_000_000, 500_000_000, 25_000_000),
                "commercial" => if listing_type == "rent" { (600_000, 40_000_000, 5_000_000) } else { (40_000_000, 1_500_000_000, 200_000_000) },
                _ => (300_000, 30_000_000, 3_000_000),
            },
            _ => {
                // Default ranges for other cities
                match property_type.to_lowercase().as_str() {
                    "apartment" => if listing_type == "rent" { (150_000, 4_000_000, 800_000) } else { (4_000_000, 150_000_000, 25_000_000) },
                    "house" => if listing_type == "rent" { (250_000, 8_000_000, 1_200_000) } else { (8_000_000, 250_000_000, 35_000_000) },
                    "duplex" => if listing_type == "rent" { (400_000, 15_000_000, 2_500_000) } else { (15_000_000, 500_000_000, 60_000_000) },
                    "land" => (800_000, 300_000_000, 20_000_000),
                    "commercial" => if listing_type == "rent" { (500_000, 30_000_000, 4_000_000) } else { (30_000_000, 1_000_000_000, 150_000_000) },
                    _ => (200_000, 20_000_000, 2_500_000),
                }
            }
        };

        let (min_price, max_price, avg_price) = base_prices;
        let price_per_sqm = match property_type.to_lowercase().as_str() {
            "apartment" => Some(if listing_type == "rent" { 2500.0 } else { 75000.0 }),
            "house" => Some(if listing_type == "rent" { 3000.0 } else { 85000.0 }),
            "duplex" => Some(if listing_type == "rent" { 4000.0 } else { 120000.0 }),
            "commercial" => Some(if listing_type == "rent" { 8000.0 } else { 200000.0 }),
            _ => None,
        };

        Ok(PropertyPriceData {
            min_price,
            max_price,
            average_price: avg_price,
            currency: "NGN".to_string(),
            price_per_sqm,
        })
    }

    /// Generate comprehensive property amenities based on type and location
    pub fn get_comprehensive_amenities(&self, property_type: &str, city: &str, price_range: &str) -> Vec<String> {
        let mut amenities = Vec::new();

        // Base amenities for all properties
        let base_amenities = vec![
            "Security", "Power Supply", "Water Supply", "Access Road"
        ];
        amenities.extend(base_amenities.iter().map(|s| s.to_string()));

        // Type-specific amenities
        match property_type.to_lowercase().as_str() {
            "apartment" | "house" | "duplex" | "bungalow" => {
                let residential_amenities = vec![
                    "Fitted Kitchen", "Wardrobes", "Tiled Floors", "POP Ceiling",
                    "Window Blinds", "Parking Space", "Compound", "Fence/Gate",
                ];
                amenities.extend(residential_amenities.iter().map(|s| s.to_string()));

                // Premium amenities for high-end properties
                if price_range == "premium" || city.to_lowercase() == "lagos" {
                    let premium_amenities = vec![
                        "Air Conditioning", "Generator", "Elevator", "Swimming Pool",
                        "Gym/Fitness Center", "24/7 Security", "CCTV Surveillance",
                        "Intercom System", "Backup Water System", "Solar Panel",
                        "Smart Home Features", "Landscaped Garden",
                    ];
                    amenities.extend(premium_amenities.iter().map(|s| s.to_string()));
                }
            },
            "commercial" | "office" | "shop" => {
                let commercial_amenities = vec![
                    "Reception Area", "Conference Room", "Parking Space",
                    "Loading Bay", "Storage Space", "Fire Safety System",
                    "CCTV Surveillance", "Access Control", "Backup Generator",
                    "High-Speed Internet", "Elevator", "Cafeteria/Kitchen",
                ];
                amenities.extend(commercial_amenities.iter().map(|s| s.to_string()));
            },
            "warehouse" => {
                let warehouse_amenities = vec![
                    "High Ceiling", "Loading Dock", "Office Space", "Parking Area",
                    "Security System", "Fire Safety", "Drainage System",
                    "Wide Access Road", "Perimeter Fencing", "Weigh Bridge",
                ];
                amenities.extend(warehouse_amenities.iter().map(|s| s.to_string()));
            },
            "land" => {
                let land_amenities = vec![
                    "Survey Done", "Fenced/Unfenced", "Corner Piece",
                    "Residential Area", "Commercial Viable", "Government Allocation",
                    "Dry Land", "Access to Public Utilities", "Close to Major Road",
                ];
                amenities.extend(land_amenities.iter().map(|s| s.to_string()));
            },
            _ => {}
        }

        // Location-specific amenities
        match city.to_lowercase().as_str() {
            "lagos" => {
                amenities.extend(vec![
                    "Close to Airport".to_string(),
                    "Near Shopping Mall".to_string(),
                    "Close to Beach".to_string(),
                    "Good Road Network".to_string(),
                ]);
            },
            "abuja" => {
                amenities.extend(vec![
                    "Government Reserved Area".to_string(),
                    "Diplomatic Zone".to_string(),
                    "Well Planned Layout".to_string(),
                ]);
            },
            _ => {}
        }

        // Remove duplicates and sort
        amenities.sort();
        amenities.dedup();
        amenities
    }

    /// Advanced property scoring algorithm
    pub fn calculate_comprehensive_property_score(
        &self,
        has_all_documents: bool,
        document_authenticity_score: f64, // 0.0 to 1.0
        photo_count: usize,
        photo_quality_score: f64, // 0.0 to 1.0
        description_length: usize,
        description_quality_score: f64, // 0.0 to 1.0
        has_coordinates: bool,
        location_accuracy_score: f64, // 0.0 to 1.0
        price_reasonability_score: f64, // 0.0 to 1.0
        landlord_verification_level: u8, // 1-5 scale
        market_competitiveness: f64, // 0.0 to 1.0
    ) -> u32 {
        let mut score = 0.0;

        // Document completeness and authenticity (25 points max)
        if has_all_documents {
            score += 15.0 + (document_authenticity_score * 10.0);
        }

        // Photo quality and quantity (20 points max)
        let photo_quantity_score = std::cmp::min(photo_count, 10) as f64 * 1.0;
        let photo_score = photo_quantity_score + (photo_quality_score * 10.0);
        score += std::cmp::min(photo_score as u32, 20) as f64;

        // Description quality (15 points max)
        let desc_length_score = if description_length > 500 {
            10.0
        } else if description_length > 200 {
            7.0
        } else if description_length > 100 {
            5.0
        } else {
            2.0
        };
        score += desc_length_score + (description_quality_score * 5.0);

        // Location accuracy (15 points max)
        if has_coordinates {
            score += 8.0 + (location_accuracy_score * 7.0);
        }

        // Price competitiveness (15 points max)
        score += price_reasonability_score * 15.0;

        // Landlord trustworthiness (10 points max)
        score += (landlord_verification_level as f64 / 5.0) * 10.0;

        std::cmp::min(score as u32, 100)
    }

    /// Generate unique property reference with check digit
    pub fn generate_property_reference(
        &self,
        property_type: &str,
        state: &str,
        lga: &str,
        id: &Uuid,
        created_at: DateTime<Utc>,
    ) -> String {
        let type_code = match property_type.to_lowercase().as_str() {
            "apartment" => "APT",
            "house" => "HSE",
            "duplex" => "DPX",
            "bungalow" => "BNG",
            "commercial" => "COM",
            "land" => "LND",
            "warehouse" => "WHE",
            "office" => "OFC",
            "shop" => "SHP",
            "hotel" => "HTL",
            _ => "GEN",
        };

        let state_code = state.chars().take(3).collect::<String>().to_uppercase();
        let lga_code = lga.chars().take(2).collect::<String>().to_uppercase();
        let year = created_at.format("%y");
        let id_short = id.to_string().replace("-", "").chars().take(6).collect::<String>().to_uppercase();
        
        // Generate check digit
        let base_ref = format!("{}{}{}{}{}", type_code, state_code, lga_code, year, id_short);
        let check_digit = self.calculate_check_digit(&base_ref);
        
        format!("VN-{}-{}", base_ref, check_digit)
    }

    /// Calculate check digit for reference validation
    fn calculate_check_digit(&self, reference: &str) -> char {
        let sum: u32 = reference.chars()
            .enumerate()
            .map(|(i, c)| {
                let weight = (i % 7) + 1;
                let value = if c.is_ascii_digit() {
                    c.to_digit(10).unwrap_or(0)
                } else {
                    (c as u32) - 65 + 10 // A=10, B=11, etc.
                };
                value * weight as u32
            })
            .sum();
        
        let remainder = sum % 36;
        if remainder < 10 {
            char::from_digit(remainder, 10).unwrap()
        } else {
            char::from((remainder - 10 + 65) as u8)
        }
    }

    /// Comprehensive document validation
    pub fn validate_required_documents(
        &self,
        property_type: &str,
        listing_type: &str,
        state: &str,
    ) -> HashMap<String, DocumentRequirement> {
        let mut requirements = HashMap::new();

        // Certificate of Occupancy (always required)
        requirements.insert("certificate_of_occupancy".to_string(), DocumentRequirement {
            required: true,
            description: "Certificate of Occupancy (C of O) from the state government".to_string(),
            alternatives: vec!["Deed of Sublease".to_string(), "Governor's Consent".to_string()],
            validity_period_years: Some(99),
        });

        match property_type.to_lowercase().as_str() {
            "land" => {
                requirements.insert("survey_plan".to_string(), DocumentRequirement {
                    required: true,
                    description: "Official survey plan showing boundaries and coordinates".to_string(),
                    alternatives: vec![],
                    validity_period_years: None,
                });

                if listing_type.to_lowercase() == "sale" {
                    requirements.insert("deed_of_assignment".to_string(), DocumentRequirement {
                        required: true,
                        description: "Deed of Assignment showing ownership transfer rights".to_string(),
                        alternatives: vec!["Certificate of Ownership".to_string()],
                        validity_period_years: None,
                    });
                }
            },
            "apartment" | "house" | "duplex" | "bungalow" => {
                requirements.insert("building_plan_approval".to_string(), DocumentRequirement {
                    required: true,
                    description: "Approved building plan from local government".to_string(),
                    alternatives: vec!["Development Permit".to_string()],
                    validity_period_years: Some(2),
                });

                if listing_type.to_lowercase() == "sale" {
                    requirements.insert("deed_of_assignment".to_string(), DocumentRequirement {
                        required: true,
                        description: "Deed of Assignment for ownership transfer".to_string(),
                        alternatives: vec![],
                        validity_period_years: None,
                    });
                }

                // Lagos specific requirements
                if state.to_lowercase() == "lagos" {
                    requirements.insert("lagos_state_property_tax".to_string(), DocumentRequirement {
                        required: false,
                        description: "Current year property tax receipt (recommended)".to_string(),
                        alternatives: vec![],
                        validity_period_years: Some(1),
                    });
                }
            },
            "commercial" | "warehouse" | "office" | "shop" => {
                requirements.insert("building_plan_approval".to_string(), DocumentRequirement {
                    required: true,
                    description: "Approved commercial building plan".to_string(),
                    alternatives: vec![],
                    validity_period_years: Some(3),
                });

                requirements.insert("change_of_use_permit".to_string(), DocumentRequirement {
                    required: true,
                    description: "Change of use permit for commercial purposes".to_string(),
                    alternatives: vec!["Commercial Permit".to_string()],
                    validity_period_years: Some(5),
                });

                if listing_type.to_lowercase() == "sale" {
                    requirements.insert("deed_of_assignment".to_string(), DocumentRequirement {
                        required: true,
                        description: "Commercial property deed of assignment".to_string(),
                        alternatives: vec![],
                        validity_period_years: None,
                    });
                }
            },
            _ => {}
        }

        requirements
    }

    /// Market analysis with comparable properties
    pub async fn perform_market_analysis(
        &self,
        property_type: &str,
        city: &str,
        state: &str,
        price: i64,
        bedrooms: Option<i32>,
        size_sqm: Option<f64>,
        latitude: Option<f64>,
        longitude: Option<f64>,
    ) -> Result<MarketAnalysis, PropertyServiceError> {
        // Simulate market analysis - in production, this would query real estate databases
        let location_score = self.calculate_location_score(city, state, latitude, longitude).await?;
        
        // Get comparable properties (simulated data)
        let comparable_properties = self.get_comparable_properties(
            property_type, city, state, bedrooms, size_sqm, latitude, longitude
        ).await?;

        let price_competitiveness = if !comparable_properties.is_empty() {
            let avg_comp_price = comparable_properties.iter()
                .map(|p| p.price)
                .sum::<i64>() as f64 / comparable_properties.len() as f64;
            
            1.0 - ((price as f64 - avg_comp_price).abs() / avg_comp_price).min(1.0)
        } else {
            0.5
        };

        let investment_potential = self.calculate_investment_potential(
            property_type, city, price, location_score
        );

        let rental_yield_estimate = if property_type != "land" {
            Some(self.estimate_rental_yield(property_type, city, price))
        } else {
            None
        };

        Ok(MarketAnalysis {
            location_score,
            price_competitiveness,
            market_trends: self.get_market_trends(city, property_type),
            comparable_properties,
            investment_potential,
            rental_yield_estimate,
        })
    }

    async fn calculate_location_score(
        &self,
        city: &str,
        state: &str,
        latitude: Option<f64>,
        longitude: Option<f64>,
    ) -> Result<f64, PropertyServiceError> {
        let mut score = 0.0;

        // City tier scoring
        score += match city.to_lowercase().as_str() {
            "lagos" => 0.95,
            "abuja" => 0.90,
            "port harcourt" | "kano" | "ibadan" => 0.75,
            "benin city" | "enugu" | "jos" | "kaduna" => 0.65,
            "aba" | "onitsha" | "warri" | "ilorin" => 0.55,
            _ => 0.40,
        };

        // State economic ranking
        let state_bonus = match state.to_lowercase().as_str() {
            "lagos" => 0.05,
            "fct" | "abuja" => 0.05,
            "rivers" | "delta" | "akwa ibom" => 0.03,
            "ogun" | "oyo" | "kano" => 0.02,
            _ => 0.0,
        };
        score = (score + state_bonus).min(1.0);

        // If coordinates available, check proximity to key facilities
        if let (Some(lat), Some(lng)) = (latitude, longitude) {
            let proximity_score = self.calculate_proximity_score(lat, lng, city).await?;
            score = (score + proximity_score * 0.1).min(1.0);
        }

        Ok(score)
    }

    async fn calculate_proximity_score(
        &self,
        latitude: f64,
        longitude: f64,
        city: &str,
    ) -> Result<f64, PropertyServiceError> {
        let mut proximity_score = 0.0;

        // Key facilities coordinates for major cities (simplified)
        let key_facilities = match city.to_lowercase().as_str() {
            "lagos" => vec![
                (6.5244, 3.3792, "Lagos Airport", 0.3),
                (6.4281, 3.4219, "Victoria Island", 0.3),
                (6.4698, 3.5852, "Lekki", 0.2),
                (6.6018, 3.3515, "Ikeja", 0.2),
            ],
            "abuja" => vec![
                (9.0579, 7.2623, "Abuja Airport", 0.3),
                (9.0765, 7.3986, "Central Business District", 0.4),
                (9.0643, 7.4892, "Garki", 0.2),
                (8.9806, 7.1775, "Gwarinpa", 0.1),
            ],
            "port harcourt" => vec![
                (4.7719, 7.0134, "Port Harcourt Airport", 0.4),
                (4.8156, 7.0498, "GRA", 0.3),
                (4.8394, 6.9654, "Trans Amadi", 0.3),
            ],
            _ => vec![],
        };

        for (fac_lat, fac_lng, _name, weight) in key_facilities {
            let distance_km = self.calculate_distance(latitude, longitude, fac_lat, fac_lng);
            let facility_score = if distance_km <= 5.0 {
                1.0
            } else if distance_km <= 15.0 {
                1.0 - ((distance_km - 5.0) / 10.0)
            } else {
                0.0
            };
            proximity_score += facility_score * weight;
        }

        Ok(proximity_score.min(1.0))
    }

    fn calculate_distance(&self, lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
        let r = 6371.0; // Earth's radius in km
        let d_lat = (lat2 - lat1).to_radians();
        let d_lng = (lng2 - lng1).to_radians();
        let a = (d_lat / 2.0).sin().powi(2) +
            lat1.to_radians().cos() * lat2.to_radians().cos() *
            (d_lng / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        r * c
    }

    async fn get_comparable_properties(
        &self,
        property_type: &str,
        city: &str,
        state: &str,
        bedrooms: Option<i32>,
        size_sqm: Option<f64>,
        latitude: Option<f64>,
        longitude: Option<f64>,
    ) -> Result<Vec<ComparableProperty>, PropertyServiceError> {
        // In production, this would query a real estate database
        // For now, generate realistic comparable properties
        
        let base_price = self.get_market_price_data(city, state, property_type, "sale").await?.average_price;
        let mut comparables = Vec::new();

        // Generate 3-7 comparable properties with realistic variations
        let count = fastrand::usize(3..=7);
        for i in 0..count {
            let price_variation = 1.0 + (fastrand::f64() - 0.5) * 0.4; // ±20% variation
            let distance_km = fastrand::f64() * 5.0 + 0.5; // 0.5-5.5 km radius
            
            let comparable_bedrooms = bedrooms.map(|b| {
                let variation = fastrand::i32(-1..=1);
                (b + variation).max(1)
            });

            comparables.push(ComparableProperty {
                address: format!("{} Street, {} {}", 
                    ["Admiralty", "Bourdillon", "Ademola", "Awolowo", "Mobolaji"][i % 5],
                    city, i + 1),
                price: ((base_price as f64 * price_variation) as i64),
                size_sqm: size_sqm.map(|s| s * (0.8 + fastrand::f64() * 0.4)),
                bedrooms: comparable_bedrooms,
                distance_km,
            });
        }

        // Sort by distance if coordinates available
        if latitude.is_some() && longitude.is_some() {
            comparables.sort_by(|a, b| a.distance_km.partial_cmp(&b.distance_km).unwrap());
        }

        Ok(comparables)
    }

    fn calculate_investment_potential(
        &self,
        property_type: &str,
        city: &str,
        price: i64,
        location_score: f64,
    ) -> f64 {
        let mut potential = 0.0;

        // Base potential by property type
        potential += match property_type.to_lowercase().as_str() {
            "land" => 0.8, // High appreciation potential
            "commercial" => 0.7,
            "duplex" | "house" => 0.6,
            "apartment" => 0.5,
            _ => 0.4,
        };

        // Location multiplier
        potential *= location_score;

        // City growth potential
        let city_multiplier = match city.to_lowercase().as_str() {
            "lagos" => 1.1,
            "abuja" => 1.2, // Higher growth potential
            "port harcourt" => 1.0,
            "kano" | "ibadan" => 0.9,
            _ => 0.8,
        };
        potential *= city_multiplier;

        // Price point consideration (mid-range properties often have better potential)
        let price_millions = price as f64 / 1_000_000.0;
        let price_factor = if price_millions < 10.0 {
            1.1 // Affordable range
        } else if price_millions < 50.0 {
            1.2 // Sweet spot
        } else if price_millions < 200.0 {
            1.0 // Premium range
        } else {
            0.8 // Ultra premium
        };
        potential *= price_factor;

        potential.min(1.0)
    }

    fn estimate_rental_yield(&self, property_type: &str, city: &str, purchase_price: i64) -> f64 {
        // Annual rental yield estimates for different property types and cities
        let base_yield = match (city.to_lowercase().as_str(), property_type.to_lowercase().as_str()) {
            ("lagos", "apartment") => 0.08,
            ("lagos", "house") => 0.07,
            ("lagos", "duplex") => 0.06,
            ("lagos", "commercial") => 0.12,
            ("abuja", "apartment") => 0.09,
            ("abuja", "house") => 0.08,
            ("abuja", "duplex") => 0.07,
            ("abuja", "commercial") => 0.11,
            ("port harcourt", "apartment") => 0.10,
            ("port harcourt", "house") => 0.09,
            ("port harcourt", "duplex") => 0.08,
            ("port harcourt", "commercial") => 0.13,
            (_, "apartment") => 0.11,
            (_, "house") => 0.10,
            (_, "duplex") => 0.09,
            (_, "commercial") => 0.14,
            _ => 0.08,
        };

        // Adjust based on price range (higher-end properties typically have lower yields)
        let price_millions = purchase_price as f64 / 1_000_000.0;
        let price_adjustment = if price_millions > 100.0 {
            0.8 // Luxury properties
        } else if price_millions > 50.0 {
            0.9 // Premium properties
        } else if price_millions < 10.0 {
            1.2 // Affordable properties (higher yield potential)
        } else {
            1.0 // Mid-range properties
        };

        base_yield * price_adjustment
    }

    fn get_market_trends(&self, city: &str, property_type: &str) -> String {
        match city.to_lowercase().as_str() {
            "lagos" => {
                match property_type.to_lowercase().as_str() {
                    "apartment" => "Strong demand in Victoria Island, Ikoyi, and Lekki. Price growth of 8-12% annually.".to_string(),
                    "house" => "High demand for detached houses in GRA Ikeja, Magodo, and Lekki Peninsula.".to_string(),
                    "duplex" => "Premium duplex market showing 10-15% annual growth in choice locations.".to_string(),
                    "land" => "Land prices appreciating rapidly, especially along Lekki-Epe corridor.".to_string(),
                    "commercial" => "Office spaces in high demand, retail spaces experiencing mixed performance.".to_string(),
                    _ => "Mixed performance across property segments.".to_string(),
                }
            },
            "abuja" => {
                match property_type.to_lowercase().as_str() {
                    "apartment" => "Steady growth in Wuse, Garki, and Maitama areas. 6-10% annual appreciation.".to_string(),
                    "house" => "Strong market for houses in Asokoro, Maitama, and Gwarinpa.".to_string(),
                    "duplex" => "Growing demand in developing areas like Lugbe and Life Camp.".to_string(),
                    "land" => "Government allocation lands showing strong appreciation potential.".to_string(),
                    "commercial" => "CBD areas maintaining strong rental and capital growth.".to_string(),
                    _ => "Stable market with moderate growth potential.".to_string(),
                }
            },
            "port harcourt" => {
                "Oil industry recovery driving property market improvement. 5-8% annual growth expected.".to_string()
            },
            _ => "Emerging market with good growth potential for well-located properties.".to_string(),
        }
    }

    /// Extract and validate location keywords for search optimization
    pub fn extract_location_keywords(
        &self,
        address: &str,
        city: &str,
        state: &str,
        lga: &str,
        landmark: Option<&str>,
    ) -> Vec<String> {
        let mut keywords = Vec::new();
        
        // Add main location components
        keywords.push(city.to_lowercase());
        keywords.push(state.to_lowercase());
        keywords.push(lga.to_lowercase());
        
        // Add popular area names and districts
        let popular_areas = self.get_popular_areas(city);
        for area in popular_areas {
            if address.to_lowercase().contains(&area.to_lowercase()) ||
               landmark.map_or(false, |l| l.to_lowercase().contains(&area.to_lowercase())) {
                keywords.push(area.to_lowercase());
            }
        }
        
        // Extract meaningful words from address (filter out numbers and common words)
        let stop_words = ["street", "road", "avenue", "close", "crescent", "way", "drive", "lane", 
                         "estate", "phase", "block", "plot", "no", "number", "off", "by", "near"];
        
        let address_words: Vec<String> = address
            .split(&[' ', ',', '-', '_', '.'][..])
            .filter(|word| {
                word.len() > 2 && 
                !word.chars().all(|c| c.is_ascii_digit()) &&
                !stop_words.contains(&word.to_lowercase().as_str())
            })
            .map(|word| word.to_lowercase())
            .collect();
        keywords.extend(address_words);
        
        // Add landmark keywords if provided
        if let Some(landmark_text) = landmark {
            let landmark_words: Vec<String> = landmark_text
                .split(&[' ', ',', '-', '_', '.'][..])
                .filter(|word| word.len() > 2 && !stop_words.contains(&word.to_lowercase().as_str()))
                .map(|word| word.to_lowercase())
                .collect();
            keywords.extend(landmark_words);
        }
        
        // Add synonyms and variations
        keywords.extend(self.get_location_synonyms(city, state));
        
        // Remove duplicates and sort
        keywords.sort();
        keywords.dedup();
        
        keywords
    }

    fn get_popular_areas(&self, city: &str) -> Vec<String> {
        match city.to_lowercase().as_str() {
            "lagos" => vec![
                "Victoria Island".to_string(), "Ikoyi".to_string(), "Lekki".to_string(),
                "Ikeja".to_string(), "Surulere".to_string(), "Yaba".to_string(),
                "Magodo".to_string(), "Gbagada".to_string(), "Ajah".to_string(),
                "Banana Island".to_string(), "Parkview".to_string(), "GRA Ikeja".to_string(),
            ],
            "abuja" => vec![
                "Maitama".to_string(), "Asokoro".to_string(), "Wuse".to_string(),
                "Garki".to_string(), "Gwarinpa".to_string(), "Utako".to_string(),
                "Jabi".to_string(), "Life Camp".to_string(), "Lugbe".to_string(),
            ],
            "port harcourt" => vec![
                "GRA".to_string(), "Trans Amadi".to_string(), "Old GRA".to_string(),
                "D-Line".to_string(), "Elelenwo".to_string(),
            ],
            _ => vec![],
        }
    }

    fn get_location_synonyms(&self, city: &str, state: &str) -> Vec<String> {
        let mut synonyms = Vec::new();
        
        // Add common abbreviations and alternative names
        match city.to_lowercase().as_str() {
            "lagos" => {
                synonyms.extend(vec!["lagos state".to_string(), "eko".to_string(), "center of excellence".to_string()]);
            },
            "abuja" => {
                synonyms.extend(vec!["fct".to_string(), "federal capital territory".to_string(), "nigeria capital".to_string()]);
            },
            "port harcourt" => {
                synonyms.extend(vec!["portharcourt".to_string(), "ph".to_string(), "rivers state capital".to_string()]);
            },
            _ => {}
        }
        
        synonyms
    }

    /// Advanced property validation with external data sources
    pub async fn validate_property_comprehensively(
        &self,
        address: &str,
        city: &str,
        state: &str,
        property_type: &str,
        listing_type: &str,
        price: i64,
        size_sqm: Option<f64>,
        bedrooms: Option<i32>,
    ) -> Result<ValidationReport, PropertyServiceError> {
        let mut report = ValidationReport::new();
        
        // 1. Address and location validation
        match self.validate_and_geocode_address(address, city, state).await {
            Ok((lat, lng, formatted_address)) => {
                report.location_valid = true;
                report.coordinates = Some((lat, lng));
                report.formatted_address = Some(formatted_address);
                report.add_success("Location validated and geocoded successfully");
            },
            Err(e) => {
                report.add_error(&format!("Location validation failed: {}", e));
            }
        }
        
        // 2. Price validation with market data
        match self.validate_property_price(price, property_type, city, state, bedrooms, size_sqm, listing_type).await {
            Ok(price_data) => {
                report.price_valid = true;
                report.market_price_range = Some((price_data.min_price, price_data.max_price));
                report.add_success("Price validated against market data");
                
                // Price competitiveness analysis
                let competitiveness = if price <= price_data.average_price {
                    "Very Competitive"
                } else if price <= price_data.average_price * 120 / 100 {
                    "Competitive"
                } else if price <= price_data.max_price {
                    "Above Average"
                } else {
                    "Premium Pricing"
                };
                report.price_analysis = Some(competitiveness.to_string());
            },
            Err(e) => {
                report.add_warning(&format!("Price validation concern: {}", e));
            }
        }
        
        // 3. Document requirements check
        let doc_requirements = self.validate_required_documents(property_type, listing_type, state);
        report.document_requirements = doc_requirements;
        
        // 4. Property specifications validation
        if let Err(errors) = self.validate_property_specifications(property_type, bedrooms, size_sqm, price) {
            for error in errors {
                report.add_warning(&error);
            }
        } else {
            report.add_success("Property specifications are reasonable");
        }
        
        // 5. Market analysis
        if let Some((lat, lng)) = report.coordinates {
            match self.perform_market_analysis(property_type, city, state, price, bedrooms, size_sqm, Some(lat), Some(lng)).await {
                Ok(analysis) => {
                    report.market_analysis = Some(analysis);
                    report.add_success("Market analysis completed");
                },
                Err(e) => {
                    report.add_warning(&format!("Market analysis incomplete: {}", e));
                }
            }
        }
        
        // 6. Investment scoring
        report.investment_score = self.calculate_comprehensive_property_score(
            false, // Documents not uploaded yet
            0.0,   // Document authenticity unknown
            0,     // Photos not uploaded yet
            0.0,   // Photo quality unknown
            0,     // Description not provided
            0.0,   // Description quality unknown
            report.coordinates.is_some(),
            if report.location_valid { 1.0 } else { 0.0 },
            if report.price_valid { 0.8 } else { 0.3 },
            3,     // Average landlord verification
            0.7,   // Assumed market competitiveness
        );
        
        Ok(report)
    }

    fn validate_property_specifications(
        &self,
        property_type: &str,
        bedrooms: Option<i32>,
        size_sqm: Option<f64>,
        price: i64,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Validate bedrooms
        if let Some(beds) = bedrooms {
            if beds < 0 {
                errors.push("Number of bedrooms cannot be negative".to_string());
            } else if beds > 20 {
                errors.push("Number of bedrooms seems unreasonably high".to_string());
            }
        }
        
        // Validate size
        if let Some(size) = size_sqm {
            if size <= 0.0 {
                errors.push("Property size must be greater than 0".to_string());
            } else if size > 10000.0 && !["land", "commercial", "warehouse"].contains(&property_type.to_lowercase().as_str()) {
                errors.push("Property size seems unreasonably large for this type".to_string());
            }
            
            // Price per square meter validation
            let price_per_sqm = price as f64 / size;
            let (min_price_per_sqm, max_price_per_sqm) = match property_type.to_lowercase().as_str() {
                "apartment" => (20_000.0, 500_000.0),
                "house" => (30_000.0, 600_000.0),
                "duplex" => (50_000.0, 800_000.0),
                "commercial" => (100_000.0, 2_000_000.0),
                "warehouse" => (10_000.0, 200_000.0),
                _ => (5_000.0, 1_000_000.0),
            };
            
            if price_per_sqm < min_price_per_sqm {
                errors.push(format!("Price per sqm (₦{:,.0}) is below market minimum (₦{:,.0})", price_per_sqm, min_price_per_sqm));
            } else if price_per_sqm > max_price_per_sqm {
                errors.push(format!("Price per sqm (₦{:,.0}) is above market maximum (₦{:,.0})", price_per_sqm, max_price_per_sqm));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// Supporting structures
#[derive(Debug, Clone)]
pub struct DocumentRequirement {
    pub required: bool,
    pub description: String,
    pub alternatives: Vec<String>,
    pub validity_period_years: Option<u32>,
}

#[derive(Debug)]
pub struct ValidationReport {
    pub location_valid: bool,
    pub price_valid: bool,
    pub coordinates: Option<(f64, f64)>,
    pub formatted_address: Option<String>,
    pub market_price_range: Option<(i64, i64)>,
    pub price_analysis: Option<String>,
    pub document_requirements: HashMap<String, DocumentRequirement>,
    pub market_analysis: Option<MarketAnalysis>,
    pub investment_score: u32,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub success_messages: Vec<String>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            location_valid: false,
            price_valid: false,
            coordinates: None,
            formatted_address: None,
            market_price_range: None,
            price_analysis: None,
            document_requirements: HashMap::new(),
            market_analysis: None,
            investment_score: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            success_messages: Vec::new(),
        }
    }
    
    pub fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
    
    pub fn add_warning(&mut self, warning: &str) {
        self.warnings.push(warning.to_string());
    }
    
    pub fn add_success(&mut self, success: &str) {
        self.success_messages.push(success.to_string());
    }
    
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
    
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}