// db/vendordb.rs - COMPLETE IMPLEMENTATION
use async_trait::async_trait;
use chrono::{Utc};
use uuid::Uuid;
use sqlx::Error;
use num_traits::ToPrimitive;

use super::db::DBClient;
use crate::models::{vendormodels::*, walletmodels::TransactionType};
use crate::db::naira_walletdb::NairaWalletExt;

#[async_trait]
pub trait VendorExt {
    // Vendor Profile Management
    async fn create_vendor_profile(
        &self,
        user_id: Uuid,
        business_name: String,
        description: Option<String>,
        location_state: String,
        location_city: String,
    ) -> Result<VendorProfile, Error>;
    
    async fn get_vendor_profile_by_user(&self, user_id: Uuid) -> Result<Option<VendorProfile>, Error>;
    async fn get_vendor_profile_by_id(&self, vendor_id: Uuid) -> Result<Option<VendorProfile>, Error>;
    
    async fn update_vendor_profile(
        &self,
        vendor_id: Uuid,
        business_name: Option<String>,
        description: Option<String>,
        location_state: Option<String>,
        location_city: Option<String>,
    ) -> Result<VendorProfile, Error>;
    
    // Subscription Management
    async fn upgrade_subscription(
        &self,
        vendor_id: Uuid,
        tier: SubscriptionTier,
        payment_reference: String,
        duration_months: i32,
    ) -> Result<(VendorProfile, VendorSubscription), Error>;
    
    async fn check_subscription_active(&self, vendor_id: Uuid) -> Result<bool, Error>;
    
    // Service/Product Management
    async fn create_service(
        &self,
        vendor_id: Uuid,
        title: String,
        description: String,
        category: ServiceCategory,
        price: f64,
        images: Vec<String>,
        location_state: String,
        location_city: String,
        tags: Option<Vec<String>>,
        stock_quantity: i32,
        is_negotiable: bool,
    ) -> Result<VendorService, Error>;
    
    async fn update_service(
        &self,
        service_id: Uuid,
        title: Option<String>,
        description: Option<String>,
        price: Option<f64>,
        images: Option<Vec<String>>,
        tags: Option<Vec<String>>,
        stock_quantity: Option<i32>,
        is_negotiable: Option<bool>,
    ) -> Result<VendorService, Error>;
    
    async fn get_service(&self, service_id: Uuid) -> Result<Option<VendorService>, Error>;
    
    async fn get_vendor_services(
        &self,
        vendor_id: Uuid,
        status: Option<ServiceStatus>,
    ) -> Result<Vec<VendorService>, Error>;
    
    async fn update_service_status(
        &self,
        service_id: Uuid,
        status: ServiceStatus,
    ) -> Result<VendorService, Error>;
    
    async fn delete_service(&self, service_id: Uuid) -> Result<(), Error>;
    
    // Service Discovery & Search
    async fn search_services(
        &self,
        category: Option<ServiceCategory>,
        location_state: Option<String>,
        location_city: Option<String>,
        min_price: Option<f64>,
        max_price: Option<f64>,
        search_query: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<VendorService>, Error>;
    
    async fn get_recommended_services(
        &self,
        user_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<VendorService>, Error>;
    
    // Analytics
    async fn record_service_view(
        &self,
        service_id: Uuid,
        viewer_id: Option<Uuid>,
        session_id: String,
    ) -> Result<(), Error>;
    
    async fn update_user_preferences(
        &self,
        user_id: Uuid,
        category: ServiceCategory,
    ) -> Result<(), Error>;
    
    // Inquiries
    async fn create_inquiry(
        &self,
        service_id: Uuid,
        vendor_id: Uuid,
        inquirer_id: Uuid,
        message: String,
    ) -> Result<ServiceInquiry, Error>;
    
    async fn get_vendor_inquiries(
        &self,
        vendor_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<ServiceInquiry>, Error>;
    
    async fn update_inquiry_status(
        &self,
        inquiry_id: Uuid,
        status: String,
    ) -> Result<ServiceInquiry, Error>;
    
    // Purchase Flow
    async fn create_service_order(
        &self,
        service_id: Uuid,
        vendor_id: Uuid,
        buyer_id: Uuid,
        quantity: i32,
        unit_price: f64,
        total_amount: f64,
        platform_fee: f64,
        payment_reference: String,
        buyer_name: String,
        buyer_email: String,
        buyer_phone: Option<String>,
        delivery_address: Option<String>,
        delivery_state: Option<String>,
        delivery_city: Option<String>,
        notes: Option<String>,
    ) -> Result<ServiceOrder, Error>;
    
    async fn get_order_by_id(&self, order_id: Uuid) -> Result<Option<ServiceOrder>, Error>;
    async fn get_order_by_reference(&self, reference: &str) -> Result<Option<ServiceOrder>, Error>;
    
    async fn update_order_status(
        &self,
        order_id: Uuid,
        status: String,
    ) -> Result<ServiceOrder, Error>;
    
    async fn get_vendor_orders(
        &self,
        vendor_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<ServiceOrder>, Error>;
    
    async fn get_buyer_orders(
        &self,
        buyer_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<ServiceOrder>, Error>;
    
    // Reviews
    async fn create_service_review(
        &self,
        service_id: Uuid,
        vendor_id: Uuid,
        order_id: Option<Uuid>,
        reviewer_id: Uuid,
        rating: i32,
        comment: Option<String>,
    ) -> Result<ServiceReview, Error>;
    
    async fn get_service_reviews(&self, service_id: Uuid) -> Result<Vec<ServiceReview>, Error>;

     async fn confirm_delivery_receipt(
        &self,
        order_id: Uuid,
    ) -> Result<ServiceOrder, Error>;

    async fn update_service_rating(
        &self,
        service_id: Uuid,
    ) -> Result<(), Error>;
}

#[async_trait]
impl VendorExt for DBClient {
    async fn create_vendor_profile(
        &self,
        user_id: Uuid,
        business_name: String,
        description: Option<String>,
        location_state: String,
        location_city: String,
    ) -> Result<VendorProfile, Error> {
        sqlx::query_as::<_, VendorProfile>(
            r#"
            INSERT INTO vendor_profiles 
            (user_id, business_name, description, location_state, location_city)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(business_name)
        .bind(description)
        .bind(location_state)
        .bind(location_city)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn get_vendor_profile_by_user(&self, user_id: Uuid) -> Result<Option<VendorProfile>, Error> {
        sqlx::query_as::<_, VendorProfile>(
            "SELECT * FROM vendor_profiles WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
    
    async fn get_vendor_profile_by_id(&self, vendor_id: Uuid) -> Result<Option<VendorProfile>, Error> {
        sqlx::query_as::<_, VendorProfile>(
            "SELECT * FROM vendor_profiles WHERE id = $1"
        )
        .bind(vendor_id)
        .fetch_optional(&self.pool)
        .await
    }
    
    async fn update_vendor_profile(
        &self,
        vendor_id: Uuid,
        business_name: Option<String>,
        description: Option<String>,
        location_state: Option<String>,
        location_city: Option<String>,
    ) -> Result<VendorProfile, Error> {
        sqlx::query_as::<_, VendorProfile>(
            r#"
            UPDATE vendor_profiles
            SET business_name = COALESCE($2, business_name),
                description = COALESCE($3, description),
                location_state = COALESCE($4, location_state),
                location_city = COALESCE($5, location_city),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(vendor_id)
        .bind(business_name)
        .bind(description)
        .bind(location_state)
        .bind(location_city)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn upgrade_subscription(
        &self,
        vendor_id: Uuid,
        tier: SubscriptionTier,
        payment_reference: String,
        duration_months: i32,
    ) -> Result<(VendorProfile, VendorSubscription), Error> {
        let mut tx = self.pool.begin().await?;
        
        let starts_at = Utc::now();
        let expires_at = starts_at + chrono::Duration::days((duration_months * 30) as i64);
        let amount = tier.monthly_price() * duration_months as f64;
        
        let subscription = sqlx::query_as::<_, VendorSubscription>(
            r#"
            INSERT INTO vendor_subscriptions
            (vendor_id, tier, amount, payment_reference, starts_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(vendor_id)
        .bind(tier)
        .bind(sqlx::types::BigDecimal::try_from(amount).unwrap())
        .bind(payment_reference)
        .bind(starts_at)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;
        
        let profile = sqlx::query_as::<_, VendorProfile>(
            r#"
            UPDATE vendor_profiles
            SET subscription_tier = $2,
                subscription_expires_at = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(vendor_id)
        .bind(tier)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;
        
        tx.commit().await?;
        Ok((profile, subscription))
    }
    
    async fn check_subscription_active(&self, vendor_id: Uuid) -> Result<bool, Error> {
        let profile = sqlx::query_as::<_, VendorProfile>(
            "SELECT * FROM vendor_profiles WHERE id = $1"
        )
        .bind(vendor_id)
        .fetch_one(&self.pool)
        .await?;
        
        if profile.subscription_tier == SubscriptionTier::Normal {
            return Ok(true);
        }
        
        if let Some(expires_at) = profile.subscription_expires_at {
            Ok(expires_at > Utc::now())
        } else {
            Ok(false)
        }
    }
    
    async fn create_service(
        &self,
        vendor_id: Uuid,
        title: String,
        description: String,
        category: ServiceCategory,
        price: f64,
        images: Vec<String>,
        location_state: String,
        location_city: String,
        tags: Option<Vec<String>>,
        stock_quantity: i32,
        is_negotiable: bool,
    ) -> Result<VendorService, Error> {
        let price_bd = sqlx::types::BigDecimal::try_from(price)
            .map_err(|_| sqlx::Error::Decode("Invalid price".into()))?;
        
        let expires_at = Utc::now() + chrono::Duration::days(30);
        
        sqlx::query_as::<_, VendorService>(
            r#"
            INSERT INTO vendor_services
            (vendor_id, title, description, category, price, images, location_state, 
             location_city, tags, expires_at, stock_quantity, is_negotiable)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#
        )
        .bind(vendor_id)
        .bind(title)
        .bind(description)
        .bind(category)
        .bind(price_bd)
        .bind(&images)
        .bind(location_state)
        .bind(location_city)
        .bind(&tags)
        .bind(expires_at)
        .bind(stock_quantity)
        .bind(is_negotiable)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn update_service(
        &self,
        service_id: Uuid,
        title: Option<String>,
        description: Option<String>,
        price: Option<f64>,
        images: Option<Vec<String>>,
        tags: Option<Vec<String>>,
        stock_quantity: Option<i32>,
        is_negotiable: Option<bool>,
    ) -> Result<VendorService, Error> {
        let price_bd = price.map(|p| sqlx::types::BigDecimal::try_from(p).ok()).flatten();
        
        sqlx::query_as::<_, VendorService>(
            r#"
            UPDATE vendor_services
            SET title = COALESCE($2, title),
                description = COALESCE($3, description),
                price = COALESCE($4, price),
                images = COALESCE($5, images),
                tags = COALESCE($6, tags),
                stock_quantity = COALESCE($7, stock_quantity),
                is_negotiable = COALESCE($8, is_negotiable),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(service_id)
        .bind(title)
        .bind(description)
        .bind(price_bd)
        .bind(&images)
        .bind(&tags)
        .bind(stock_quantity)
        .bind(is_negotiable)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn get_service(&self, service_id: Uuid) -> Result<Option<VendorService>, Error> {
        sqlx::query_as::<_, VendorService>(
            "SELECT * FROM vendor_services WHERE id = $1"
        )
        .bind(service_id)
        .fetch_optional(&self.pool)
        .await
    }
    
    async fn get_vendor_services(
        &self,
        vendor_id: Uuid,
        status: Option<ServiceStatus>,
    ) -> Result<Vec<VendorService>, Error> {
        if let Some(status) = status {
            sqlx::query_as::<_, VendorService>(
                "SELECT * FROM vendor_services WHERE vendor_id = $1 AND status = $2 ORDER BY created_at DESC"
            )
            .bind(vendor_id)
            .bind(status)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, VendorService>(
                "SELECT * FROM vendor_services WHERE vendor_id = $1 ORDER BY created_at DESC"
            )
            .bind(vendor_id)
            .fetch_all(&self.pool)
            .await
        }
    }
    
    async fn update_service_status(
        &self,
        service_id: Uuid,
        status: ServiceStatus,
    ) -> Result<VendorService, Error> {
        sqlx::query_as::<_, VendorService>(
            "UPDATE vendor_services SET status = $2, updated_at = NOW() WHERE id = $1 RETURNING *"
        )
        .bind(service_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn delete_service(&self, service_id: Uuid) -> Result<(), Error> {
        sqlx::query("DELETE FROM vendor_services WHERE id = $1")
            .bind(service_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    
    async fn search_services(
        &self,
        category: Option<ServiceCategory>,
        location_state: Option<String>,
        location_city: Option<String>,
        min_price: Option<f64>,
        max_price: Option<f64>,
        search_query: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<VendorService>, Error> {
        let mut query_str = String::from(
            r#"
            SELECT s.* FROM vendor_services s
            JOIN vendor_profiles v ON s.vendor_id = v.id
            WHERE s.status = 'active'
            AND (s.expires_at IS NULL OR s.expires_at > NOW())
            AND s.stock_quantity > 0
            "#
        );
        
        let mut bind_idx = 1;
        let mut bindings: Vec<String> = Vec::new();
        
        if category.is_some() {
            query_str.push_str(&format!(" AND s.category = ${}", bind_idx));
            bind_idx += 1;
        }
        if location_state.is_some() {
            query_str.push_str(&format!(" AND s.location_state = ${}", bind_idx));
            bind_idx += 1;
        }
        if location_city.is_some() {
            query_str.push_str(&format!(" AND s.location_city = ${}", bind_idx));
            bind_idx += 1;
        }
        if min_price.is_some() {
            query_str.push_str(&format!(" AND s.price >= ${}", bind_idx));
            bind_idx += 1;
        }
        if max_price.is_some() {
            query_str.push_str(&format!(" AND s.price <= ${}", bind_idx));
            bind_idx += 1;
        }
        if search_query.is_some() {
            query_str.push_str(&format!(" AND (s.title ILIKE ${} OR s.description ILIKE ${})", bind_idx, bind_idx));
            bind_idx += 1;
        }
        
        query_str.push_str(&format!(
            r#"
            ORDER BY 
                CASE v.subscription_tier
                    WHEN 'premium' THEN 3
                    WHEN 'pro' THEN 2
                    ELSE 1
                END DESC,
                RANDOM() * (
                    CASE v.subscription_tier
                        WHEN 'premium' THEN 5.0
                        WHEN 'pro' THEN 2.5
                        ELSE 1.0
                    END
                ) DESC,
                s.created_at DESC
            LIMIT ${} OFFSET ${}
            "#,
            bind_idx, bind_idx + 1
        ));
        
        let mut query = sqlx::query_as::<_, VendorService>(&query_str);
        
        if let Some(cat) = category {
            query = query.bind(cat);
        }
        if let Some(state) = location_state {
            query = query.bind(state);
        }
        if let Some(city) = location_city {
            query = query.bind(city);
        }
        if let Some(min) = min_price {
            query = query.bind(sqlx::types::BigDecimal::try_from(min).unwrap());
        }
        if let Some(max) = max_price {
            query = query.bind(sqlx::types::BigDecimal::try_from(max).unwrap());
        }
        if let Some(search) = search_query {
            let search_pattern = format!("%{}%", search);
            query = query.bind(search_pattern);
        }
        
        query = query.bind(limit).bind(offset);
        
        query.fetch_all(&self.pool).await
    }
    
    async fn get_recommended_services(
        &self,
        user_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<VendorService>, Error> {
        if let Some(uid) = user_id {
            sqlx::query_as::<_, VendorService>(
                r#"
                SELECT DISTINCT s.* FROM vendor_services s
                JOIN vendor_profiles v ON s.vendor_id = v.id
                LEFT JOIN user_service_preferences p ON p.category = s.category AND p.user_id = $1
                WHERE s.status = 'active'
                AND (s.expires_at IS NULL OR s.expires_at > NOW())
                AND s.stock_quantity > 0
                ORDER BY
                    COALESCE(p.preference_score, 0) DESC,
                    CASE v.subscription_tier
                        WHEN 'premium' THEN 3
                        WHEN 'pro' THEN 2
                        ELSE 1
                    END DESC,
                    RANDOM()
                LIMIT $2
                "#
            )
            .bind(uid)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, VendorService>(
                r#"
                SELECT s.* FROM vendor_services s
                JOIN vendor_profiles v ON s.vendor_id = v.id
                WHERE s.status = 'active'
                AND (s.expires_at IS NULL OR s.expires_at > NOW())
                AND s.stock_quantity > 0
                ORDER BY
                    CASE v.subscription_tier
                        WHEN 'premium' THEN 3
                        WHEN 'pro' THEN 2
                        ELSE 1
                    END DESC,
                    s.view_count DESC,
                    RANDOM()
                LIMIT $1
                "#
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        }
    }
    
    async fn record_service_view(
        &self,
        service_id: Uuid,
        viewer_id: Option<Uuid>,
        session_id: String,
    ) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO service_views (service_id, viewer_id, session_id) VALUES ($1, $2, $3)"
        )
        .bind(service_id)
        .bind(viewer_id)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    
    async fn update_user_preferences(
        &self,
        user_id: Uuid,
        category: ServiceCategory,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
            INSERT INTO user_service_preferences (user_id, category, preference_score, view_count)
            VALUES ($1, $2, 1.0, 1)
            ON CONFLICT (user_id, category)
            DO UPDATE SET
                preference_score = user_service_preferences.preference_score + 0.1,
                view_count = user_service_preferences.view_count + 1,
                last_viewed_at = NOW()
            "#
        )
        .bind(user_id)
        .bind(category)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    
    async fn create_inquiry(
        &self,
        service_id: Uuid,
        vendor_id: Uuid,
        inquirer_id: Uuid,
        message: String,
    ) -> Result<ServiceInquiry, Error> {
        sqlx::query("UPDATE vendor_services SET inquiry_count = inquiry_count + 1 WHERE id = $1")
            .bind(service_id)
            .execute(&self.pool)
            .await?;
        
        sqlx::query_as::<_, ServiceInquiry>(
            "INSERT INTO service_inquiries (service_id, vendor_id, inquirer_id, message) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(service_id)
        .bind(vendor_id)
        .bind(inquirer_id)
        .bind(message)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn get_vendor_inquiries(
        &self,
        vendor_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<ServiceInquiry>, Error> {
        if let Some(status) = status {
            sqlx::query_as::<_, ServiceInquiry>(
                "SELECT * FROM service_inquiries WHERE vendor_id = $1 AND status = $2 ORDER BY created_at DESC"
            )
            .bind(vendor_id)
            .bind(status)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ServiceInquiry>(
                "SELECT * FROM service_inquiries WHERE vendor_id = $1 ORDER BY created_at DESC"
            )
            .bind(vendor_id)
            .fetch_all(&self.pool)
            .await
        }
    }
    
    async fn update_inquiry_status(
        &self,
        inquiry_id: Uuid,
        status: String,
    ) -> Result<ServiceInquiry, Error> {
        sqlx::query_as::<_, ServiceInquiry>(
            "UPDATE service_inquiries SET status = $2, responded_at = NOW() WHERE id = $1 RETURNING *"
        )
        .bind(inquiry_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn create_service_order(
        &self,
        service_id: Uuid,
        vendor_id: Uuid,
        buyer_id: Uuid,
        quantity: i32,
        unit_price: f64,
        total_amount: f64,
        platform_fee: f64,
        payment_reference: String,
        buyer_name: String,
        buyer_email: String,
        buyer_phone: Option<String>,
        delivery_address: Option<String>,
        delivery_state: Option<String>,
        delivery_city: Option<String>,
        notes: Option<String>,
    ) -> Result<ServiceOrder, Error> {
        let order_number = format!("ORD-{}", uuid::Uuid::new_v4().to_string()[..8].to_uppercase());
        let vendor_amount = total_amount - platform_fee;
        
        sqlx::query_as::<_, ServiceOrder>(
            r#"
            INSERT INTO service_orders 
            (order_number, service_id, vendor_id, buyer_id, quantity, unit_price, total_amount, 
             platform_fee, vendor_amount, payment_reference, buyer_name, buyer_email, buyer_phone,
             delivery_address, delivery_state, delivery_city, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING *
            "#
        )
        .bind(order_number)
        .bind(service_id)
        .bind(vendor_id)
        .bind(buyer_id)
        .bind(quantity)
        .bind(sqlx::types::BigDecimal::try_from(unit_price).unwrap())
        .bind(sqlx::types::BigDecimal::try_from(total_amount).unwrap())
        .bind(sqlx::types::BigDecimal::try_from(platform_fee).unwrap())
        .bind(sqlx::types::BigDecimal::try_from(vendor_amount).unwrap())
        .bind(payment_reference)
        .bind(buyer_name)
        .bind(buyer_email)
        .bind(buyer_phone)
        .bind(delivery_address)
        .bind(delivery_state)
        .bind(delivery_city)
        .bind(notes)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn get_order_by_id(&self, order_id: Uuid) -> Result<Option<ServiceOrder>, Error> {
        sqlx::query_as::<_, ServiceOrder>(
            "SELECT * FROM service_orders WHERE id = $1"
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
    }
    
    async fn get_order_by_reference(&self, reference: &str) -> Result<Option<ServiceOrder>, Error> {
        sqlx::query_as::<_, ServiceOrder>(
            "SELECT * FROM service_orders WHERE payment_reference = $1"
        )
        .bind(reference)
        .fetch_optional(&self.pool)
        .await
    }
    
    async fn update_order_status(
        &self,
        order_id: Uuid,
        status: String,
    ) -> Result<ServiceOrder, Error> {
        let mut update_fields = vec!["status = $2"];
        
        if status == "paid" {
            update_fields.push("paid_at = NOW()");
        } else if status == "completed" {
            update_fields.push("completed_at = NOW()");
        } else if status == "cancelled" {
            update_fields.push("cancelled_at = NOW()");
        }
        
        let query_str = format!(
            "UPDATE service_orders SET {} WHERE id = $1 RETURNING *",
            update_fields.join(", ")
        );
        
        sqlx::query_as::<_, ServiceOrder>(&query_str)
            .bind(order_id)
            .bind(status)
            .fetch_one(&self.pool)
            .await
    }
    
    async fn get_vendor_orders(
        &self,
        vendor_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<ServiceOrder>, Error> {
        if let Some(status) = status {
            sqlx::query_as::<_, ServiceOrder>(
                "SELECT * FROM service_orders WHERE vendor_id = $1 AND status = $2 ORDER BY created_at DESC"
            )
            .bind(vendor_id)
            .bind(status)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ServiceOrder>(
                "SELECT * FROM service_orders WHERE vendor_id = $1 ORDER BY created_at DESC"
            )
            .bind(vendor_id)
            .fetch_all(&self.pool)
            .await
        }
    }
    
    async fn get_buyer_orders(
        &self,
        buyer_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<ServiceOrder>, Error> {
        if let Some(status) = status {
            sqlx::query_as::<_, ServiceOrder>(
                "SELECT * FROM service_orders WHERE buyer_id = $1 AND status = $2 ORDER BY created_at DESC"
            )
            .bind(buyer_id)
            .bind(status)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ServiceOrder>(
                "SELECT * FROM service_orders WHERE buyer_id = $1 ORDER BY created_at DESC"
            )
            .bind(buyer_id)
            .fetch_all(&self.pool)
            .await
        }
    }

     // Reviews
   async fn create_service_review(
        &self,
        service_id: Uuid,
        vendor_id: Uuid,
        order_id: Option<Uuid>,
        reviewer_id: Uuid,
        rating: i32,
        comment: Option<String>,
    ) -> Result<ServiceReview, Error> {
        sqlx::query_as::<_, ServiceReview>(
            r#"
            INSERT INTO service_reviews 
            (service_id, vendor_id, order_id, reviewer_id, rating, comment)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(service_id)
        .bind(vendor_id)
        .bind(order_id)
        .bind(reviewer_id)
        .bind(rating)
        .bind(comment)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn get_service_reviews(&self, service_id: Uuid) -> Result<Vec<ServiceReview>, Error> {
        sqlx::query_as::<_, ServiceReview>(
            "SELECT * FROM service_reviews WHERE service_id = $1 ORDER BY created_at DESC"
        )
        .bind(service_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn confirm_delivery_receipt(
        &self,
        order_id: Uuid,
    ) -> Result<ServiceOrder, Error> {
        let mut tx = self.pool.begin().await?;
        
        let order = self.get_order_by_id(order_id).await?
            .ok_or_else(|| Error::RowNotFound)?;
        
        // Only for cross-state deliveries with held amount
        if order.delivery_type == DeliveryType::CrossStateDelivery 
            && order.delivery_amount_held.is_some() 
            && !order.delivery_confirmed.unwrap_or(false) {
            
            // Release held amount to vendor
            let held_amount = order.delivery_amount_held.unwrap();
            let total_vendor_amount = order.vendor_amount + held_amount.clone();
            
            // Update vendor amount
            sqlx::query(
                "UPDATE service_orders SET vendor_amount = $1, delivery_confirmed = true WHERE id = $2"
            )
            .bind(total_vendor_amount)
            .bind(order_id)
            .execute(&mut *tx)
            .await?;
            
            // Credit vendor with the held amount
            let vendor = self.get_vendor_profile_by_id(order.vendor_id).await?.unwrap();
            let held_amount_kobo = (held_amount.to_f64().unwrap_or(0.0) * 100.0) as i64;
            
            self.credit_wallet(
                vendor.user_id,
                held_amount_kobo,
                TransactionType::ServiceDelivery,
                format!("Delivery confirmation: Order {}", order.order_number),
                format!("DELIVERY_{}", order.id),
                None,
                Some(serde_json::json!({"order_id": order.id})),
            ).await?;
        }
        
        let updated_order = self.update_order_status(order_id, "delivered".to_string()).await?;
        tx.commit().await?;
        Ok(updated_order)
    }

    async fn update_service_rating(
        &self,
        service_id: Uuid,
    ) -> Result<(), Error> {
        let avg_rating: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(rating::float) FROM service_reviews WHERE service_id = $1"
        )
        .bind(service_id)
        .fetch_one(&self.pool)
        .await?;
        
        sqlx::query(
            "UPDATE vendor_services SET rating = $1 WHERE id = $2"
        )
        .bind(avg_rating)
        .bind(service_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}