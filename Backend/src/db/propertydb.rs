use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use sqlx::types::Json;
use async_trait::async_trait;
use num_traits::ToPrimitive;
use anyhow;

// Import the correct BigDecimal type that SQLx uses
use sqlx::types::BigDecimal as SqlxBigDecimal;

use crate::{
    db::db::DBClient, 
    dtos::propertydtos::{
        AgentVerificationDto, CreatePropertyDto, LawyerVerificationDto
    }, 
    models::propertymodel::{ListingType, Property, PropertyStatus, CurrencyType, PropertyType, PropertyVerification}
};

#[derive(Debug)]
pub struct PropertySearchFilters {
    pub property_type: Option<PropertyType>,
    pub listing_type: Option<ListingType>,
    pub min_price: Option<i64>,
    pub max_price: Option<i64>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
}

#[async_trait]
pub trait PropertyExt {
    async fn create_property(
        &self,
        landlord_id: Uuid,
        property_data: CreatePropertyDto
    ) -> Result<Property, anyhow::Error>;

    async fn get_property_by_id(
        &self,
        property_id: Uuid,
    ) -> Result<Option<Property>, sqlx::Error>;

    async fn get_properties_by_landlord(
        &self,
        landlord_id: Uuid,
        page: u32,
        limit: usize,
    ) -> Result<Vec<Property>, sqlx::Error>;

    async fn check_property_duplicate(
        &self,
        property_hash: &str,
        coordinates_hash: &str,
    ) -> Result<Option<Property>, sqlx::Error>;

    async fn assign_agent_to_property(
        &self,
        property_id: Uuid,
        agent_id: Uuid,
    ) -> Result<Property, sqlx::Error>;

    async fn get_all_properties_within_agent_landmark(
        &self,
        landmark:  String,
        page: u32,
        limit: usize,
    ) -> Result<Vec<Property>, sqlx::Error>;

    async fn get_all_properties_within_lawyer_state(
        &self,
        state:  String,
        page: u32,
        limit: usize,
    ) -> Result<Vec<Property>, sqlx::Error>;

    async fn get_properties_for_agent_verification(
        &self,
        agent_id: Uuid,
    ) -> Result<Vec<Property>, sqlx::Error>;

    async fn agent_verify_property(
        &self,
        agent_id: Uuid,
        verification_data: AgentVerificationDto
    ) -> Result<Property, sqlx::Error>;

    async fn get_properties_for_lawyer_verification(
        &self,
        lawyer_id: Uuid,
    ) -> Result<Vec<Property>, sqlx::Error>;

    async fn lawyer_verify_property(
        &self,
        lawyer_id: Uuid,
        verification_data: LawyerVerificationDto,
    ) -> Result<Property, sqlx::Error>;

    async fn get_active_properties(
        &self,
        filter: PropertySearchFilters,
        page: u32,
        limit: usize,
    ) -> Result<Vec<Property>, sqlx::Error>;

    async fn update_property_status(
        &self,
        property_id: Uuid,
        status: PropertyStatus,
    ) -> Result<Property, sqlx::Error>;

    async fn update_property_bidding_price(
        &self,
        property_id: Uuid,
        bidding_price: i64,
    ) -> Result<Property, sqlx::Error>;

    async fn get_property_verification_history(
        &self,
        property_id: Uuid,
    ) -> Result<Vec<PropertyVerification>, sqlx::Error>;
}

impl DBClient {
    fn generate_property_hash(
        &self,
        property_data: &CreatePropertyDto
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(property_data.address.to_lowercase().as_bytes());
        hasher.update(property_data.city.to_lowercase().as_bytes());
        hasher.update(property_data.state.to_lowercase().as_bytes());
        hasher.update(property_data.lga.to_lowercase().as_bytes());
        hasher.update(property_data.country.to_lowercase().as_bytes());
        hasher.update(format!("{:?}", property_data.property_type).as_bytes());
        hasher.update(format!("{:?}", property_data.listing_type).as_bytes());

        // Includes Size and bedrooms in hash for similar properties
        if let Some(bedrooms) = property_data.bedrooms {
            hasher.update(bedrooms.to_string().as_bytes());
        }
        if let Some(size) = &property_data.size_sqm {
            hasher.update(size.to_string().as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    fn generate_coordinates_hash(
        &self,
        lat: Option<SqlxBigDecimal>,
        lng: Option<SqlxBigDecimal>,
    ) -> String {
        if let (Some(latitude), Some(longitude)) = (lat, lng) {
            // Convert SQLx BigDecimal to f64
            let lat_f64 = latitude.to_f64().unwrap_or(0.0);
            let lng_f64 = longitude.to_f64().unwrap_or(0.0);
            
            // Round to ~100m precision to catch very close properties
            let rounded_lat = (lat_f64 * 1000.0).round() / 1000.0;
            let rounded_lng = (lng_f64 * 1000.0).round() / 1000.0;

            let mut hasher = Sha256::new();
            hasher.update(rounded_lat.to_string().as_bytes());
            hasher.update(rounded_lng.to_string().as_bytes());
            format!("{:x}", hasher.finalize())
        } else {
            "no_coordinates".to_string()
        }
    }
}

#[async_trait]
impl PropertyExt for DBClient {
    async fn create_property(
        &self,
        landlord_id: Uuid,
        property_data: CreatePropertyDto
    ) -> Result<Property, anyhow::Error> {
        let property_hash = self.generate_property_hash(&property_data);
        let coordinates_hash = self.generate_coordinates_hash(
            property_data.latitude,
            property_data.longitude
        );

        // Check for duplicates
        if let Some(_existing) = self.check_property_duplicate(&property_hash, &coordinates_hash).await? {
            return Err(anyhow::anyhow!("Duplicate property listing"));
        }

        let property = sqlx::query_as!(
            Property,
            r#"
                INSERT INTO properties (
                    landlord_id, title, description, property_type, listing_type, address, city, state, lga, country, latitude, longitude, landmark,
                    bedrooms, bathrooms, toilets, size_sqm, plot_size, price, currency, price_negotiable, amenities, features, certificate_of_occupancy, deed_of_agreement, survey_plan, building_plan_approval, property_photos, property_hash, coordinates_hash, status 
                ) VALUES (
                   $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, 
                   $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32
                ) RETURNING 
                    id, landlord_id, agent_id, lawyer_id, title, description, 
                    property_type as "property_type: PropertyType", 
                    listing_type as "listing_type: ListingType",
                    address, city, state, lga, country, latitude, longitude, landmark, 
                    bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                    currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                    amenities, features, certificate_of_occupancy, deed_of_agreement, 
                    survey_plan, building_plan_approval, property_photos,
                    agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                    property_hash, coordinates_hash, 
                    status as "status: PropertyStatus", 
                    agent_verified_at, lawyer_verified_at, listed_at,
                    created_at, updated_at
            "#,
            landlord_id,
            property_data.title,
            property_data.description,
            property_data.property_type as PropertyType,
            property_data.listing_type as ListingType,
            property_data.address,
            property_data.city,
            property_data.state,
            property_data.lga,
            property_data.country,
            property_data.latitude,
            property_data.longitude,
            property_data.landmark,
            property_data.bedrooms,
            property_data.bathrooms,
            property_data.toilets,
            property_data.size_sqm,
            property_data.plot_size,
            property_data.price,
            property_data.currency as CurrencyType,
            property_data.price_negotiable.unwrap_or(false),
            Json(property_data.amenities.unwrap_or_default()) as Json<Vec<String>>,
            Json(property_data.features.unwrap_or_default()) as Json<Vec<String>>,
            property_data.certificate_of_occupancy,
            property_data.deed_of_agreement,
            property_data.survey_plan,
            property_data.building_plan_approval,
            Json(property_data.property_photos) as Json<Vec<String>>,
            property_hash,
            coordinates_hash,
            PropertyStatus::AwaitingAgent as PropertyStatus,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(property)
    }

    async fn check_property_duplicate(
        &self,
        property_hash: &str,
        coordinate_hash: &str,
    ) -> Result<Option<Property>, sqlx::Error> {
        let property = sqlx::query_as!(
            Property,
            r#"
                SELECT 
                    id, landlord_id, agent_id, lawyer_id, title, description, 
                    property_type as "property_type: PropertyType", 
                    listing_type as "listing_type: ListingType",
                    address, city, state, lga, country, latitude, longitude, landmark, 
                    bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                    currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                    amenities, features, certificate_of_occupancy, deed_of_agreement, 
                    survey_plan, building_plan_approval, property_photos, 
                    agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                    property_hash, coordinates_hash,
                    status as "status: PropertyStatus", 
                    agent_verified_at, lawyer_verified_at, listed_at, 
                    created_at, updated_at
                FROM properties 
                WHERE property_hash = $1 OR coordinates_hash = $2
            "#,
            property_hash,
            coordinate_hash
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(property)
    }

    async fn get_property_by_id(
        &self,
        property_id: Uuid,
    ) -> Result<Option<Property>, sqlx::Error> {
        let property = sqlx::query_as!(
            Property,
            r#"
            SELECT 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            FROM properties
            WHERE id = $1
            "#,
            property_id
        ) 
        .fetch_optional(&self.pool)
        .await?;

        Ok(property)
    }

    async fn assign_agent_to_property(
        &self,
        property_id: Uuid,
        agent_id: Uuid,
    ) -> Result<Property, sqlx::Error> {
        let property = sqlx::query_as!(
            Property,
            r#"
            UPDATE properties
            SET agent_id = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            "#,
            agent_id,
            property_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(property)
    }

    async fn get_properties_for_agent_verification(
        &self,
        agent_id: Uuid,
    ) -> Result<Vec<Property>, sqlx::Error> {
        let properties = sqlx::query_as!(
            Property,
            r#"
            SELECT 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            FROM properties 
            WHERE agent_id = $1 AND status = $2
            ORDER BY created_at DESC
            "#,
            agent_id,
            PropertyStatus::AwaitingAgent as PropertyStatus,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(properties)
    }

    async fn agent_verify_property(
        &self,
        agent_id: Uuid,
        verification_data: AgentVerificationDto,
    ) -> Result<Property, sqlx::Error> {
        let new_status = if verification_data.verification_status == "approved" {
            PropertyStatus::AwaitingLawyer
        } else {
            PropertyStatus::Rejected
        };

        // Updating the property
        let property = sqlx::query_as!(
            Property,
            r#"
            UPDATE properties
            SET 
                agent_verification_photos = $1,
                agent_verification_notes = $2,
                status = $3,
                agent_verified_at = NOW(),
                updated_at = NOW()
            WHERE id = $4 AND agent_id = $5
            RETURNING 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            "#,
            Json(verification_data.verification_photos) as Json<Vec<String>>,
            verification_data.notes,
            new_status as PropertyStatus,
            verification_data.property_id,
            agent_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Create verification records
        sqlx::query!(
            r#"
            INSERT INTO property_verifications (property_id, verifier_id, verifier_type, verification_status, notes, verification_photos) 
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            verification_data.property_id,
            agent_id,
            "agent",
            verification_data.verification_status,
            verification_data.notes,
            Json(verification_data.verification_photos) as Json<Vec<String>>
        )
        .execute(&self.pool)
        .await?;

        Ok(property)
    }

    async fn get_all_properties_within_agent_landmark(
        &self,
        landmark: String,
        page: u32,
        limit: usize,
    ) -> Result<Vec<Property>, sqlx::Error> {
        let offset = (page.saturating_sub(1)) * limit as u32;

        let properties = sqlx::query_as!(
            Property,
            r#"
            SELECT 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            FROM properties 
            WHERE landmark = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            landmark,
            limit as i64,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(properties)
    }

    async fn get_all_properties_within_lawyer_state(
        &self,
        state: String,
        page: u32,
        limit: usize,
    ) -> Result<Vec<Property>, sqlx::Error> {
        let offset = (page.saturating_sub(1)) * limit as u32;

        let properties = sqlx::query_as!(
            Property,
            r#"
            SELECT 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            FROM properties 
            WHERE state = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            state,
            limit as i64,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(properties)
    }

    async fn get_properties_for_lawyer_verification(
        &self,
        lawyer_id: Uuid
    ) -> Result<Vec<Property>, sqlx::Error> {
        let properties = sqlx::query_as!(
            Property,
            r#"
            SELECT 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            FROM properties 
            WHERE status = $1 AND (lawyer_id = $2 OR lawyer_id is NULL)
            ORDER BY created_at DESC
            "#,
            PropertyStatus::AwaitingLawyer as PropertyStatus,
            lawyer_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(properties)
    }

    async fn lawyer_verify_property(
        &self,
        lawyer_id: Uuid,
        verification_data: LawyerVerificationDto,
    ) -> Result<Property, sqlx::Error> {
        let new_status = if verification_data.verification_status == "approved" {
            PropertyStatus::Active
        } else {
            PropertyStatus::Rejected
        };

        let listed_at = if new_status == PropertyStatus::Active {
            Some(Utc::now())
        } else {
            None
        };

        // Updating property
        let property = sqlx::query_as!(
            Property,
            r#"
            UPDATE properties
            SET 
                lawyer_id = $1,
                lawyer_verification_notes = $2,
                status = $3,
                lawyer_verified_at = NOW(),
                listed_at = $4,
                updated_at = NOW()
            WHERE id = $5
            RETURNING 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            "#,
            lawyer_id,
            verification_data.notes,
            new_status as PropertyStatus,
            listed_at,
            verification_data.property_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Create verification record
        sqlx::query!(
            r#"
            INSERT INTO property_verifications (property_id, verifier_id, verifier_type, verification_status, notes)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            verification_data.property_id,
            lawyer_id,
            "lawyer",
            verification_data.verification_status,
            verification_data.notes
        )
        .execute(&self.pool)
        .await?;

        Ok(property)
    }

    async fn get_properties_by_landlord(
        &self,
        landlord_id: Uuid,
        page: u32,
        limit: usize,
    ) -> Result<Vec<Property>, sqlx::Error> {
        let offset = (page.saturating_sub(1)) * limit as u32;

        let properties = sqlx::query_as!(
            Property,
            r#"
            SELECT 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            FROM properties
            WHERE landlord_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            landlord_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(properties)
    }

    async fn get_active_properties(
        &self,
        filters: PropertySearchFilters,
        page: u32,
        limit: usize,
    ) -> Result<Vec<Property>, sqlx::Error> {
        let offset = (page.saturating_sub(1)) * limit as u32;

        let properties = sqlx::query_as!(
            Property,
            r#"
            SELECT 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            FROM properties
            WHERE status = $1
            AND ($2::text IS NULL OR property_type = $2::property_type)
            AND ($3::text IS NULL OR listing_type = $3::listing_type)
            AND ($4::bigint IS NULL OR price >= $4)
            AND ($5::bigint IS NULL OR price <= $5)
            AND ($6::text IS NULL OR city ILIKE $6)
            AND ($7::text IS NULL OR state ILIKE $7)
            AND ($8::int IS NULL OR bedrooms >= $8)
            AND ($9::int IS NULL OR bathrooms >= $9)
            ORDER BY created_at DESC
            LIMIT $10 OFFSET $11
            "#,
            PropertyStatus::Active as PropertyStatus,
            filters.property_type.map(|t| format!("{:?}", t).to_lowercase()),
            filters.listing_type.map(|t| format!("{:?}", t).to_lowercase()),
            filters.min_price,
            filters.max_price,
            filters.city.as_ref().map(|c| format!("%{}%", c)),
            filters.state.as_ref().map(|c| format!("%{}%", c)),
            filters.bedrooms,
            filters.bathrooms,
            limit as i64,
            offset as i64,
        ) 
        .fetch_all(&self.pool)
        .await?;

        Ok(properties)
    }

    async fn update_property_status(
        &self,
        property_id: Uuid,
        status: PropertyStatus,
    ) -> Result<Property, sqlx::Error> {
        let property = sqlx::query_as!(
            Property,
            r#"
            UPDATE properties
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            "#,
            status as PropertyStatus,
            property_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(property)
    }

    async fn update_property_bidding_price(
        &self,
        property_id: Uuid,
        bidding_price: i64,
    ) -> Result<Property, sqlx::Error> {
        let property = sqlx::query_as!(
            Property,
            r#"
            UPDATE properties
            SET bidding_price = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
                id, landlord_id, agent_id, lawyer_id, title, description, 
                property_type as "property_type: PropertyType", 
                listing_type as "listing_type: ListingType",
                address, city, state, lga, country, latitude, longitude, landmark, 
                bedrooms, bathrooms, toilets, size_sqm, plot_size, price, 
                currency as "currency: CurrencyType", price_negotiable, bidding_price, 
                amenities, features, certificate_of_occupancy, deed_of_agreement, 
                survey_plan, building_plan_approval, property_photos, 
                agent_verification_photos, agent_verification_notes, lawyer_verification_notes, 
                property_hash, coordinates_hash,
                status as "status: PropertyStatus", 
                agent_verified_at, lawyer_verified_at, listed_at, 
                created_at, updated_at
            "#,
            bidding_price,
            property_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(property)
    }

    async fn get_property_verification_history(
        &self,
        property_id: Uuid
    ) -> Result<Vec<PropertyVerification>, sqlx::Error> {
        let verifications = sqlx::query_as!(
            PropertyVerification,
            r#"
            SELECT 
                id, property_id, verifier_id, verifier_type, verification_status,
                notes, verification_photos, created_at
            FROM property_verifications
            WHERE property_id = $1
            ORDER BY created_at DESC
            "#,
            property_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(verifications)
    }
}