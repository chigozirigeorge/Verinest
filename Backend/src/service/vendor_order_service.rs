use std::sync::Arc;
use uuid::Uuid;
use chrono;
use sqlx::types::BigDecimal;
use num_traits::ToPrimitive;

use crate::{
    db::{
        db::DBClient, naira_walletdb::NairaWalletExt, userdb::UserExt, vendordb::VendorExt
    }, dtos::vendordtos::*, models::{
        usermodel::VerificationStatus, vendormodels::*, walletmodels::*
    }, service::{
        error::ServiceError,
        notification_service::NotificationService,

    }
};

pub struct VendorOrderService {
    db_client: Arc<DBClient>,
    notification_service: Arc<NotificationService>,
}

impl VendorOrderService {
    pub fn new(
        db_client: Arc<DBClient>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            db_client,
            notification_service,
        }
    }
    
    /// Create order and process payment with escrow
    pub async fn create_order_with_escrow(
        &self,
        buyer_id: Uuid,
        dto: CreateServiceOrderDto,
    ) -> Result<ServiceOrderResponse, ServiceError> {
        let mut tx = self.db_client.pool.begin().await?;
        
        // 1. Get service and validate
        let service = self.db_client
            .get_service(dto.service_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Service not found".to_string()))?;
        
        if service.status != Some(ServiceStatus::Active) {
            return Err(ServiceError::Validation("Service is not available".to_string()));
        }
        
        if service.stock_quantity < dto.quantity {
            return Err(ServiceError::Validation("Insufficient stock".to_string()));
        }
        
        // 2. Get vendor profile and validate
        let vendor = self.db_client
            .get_vendor_profile_by_id(service.vendor_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Vendor not found".to_string()))?;
        
        // Validate vendor subscription is active (expires_at must be in future)
        if let Some(expires_at) = vendor.subscription_expires_at {
            let now = chrono::Utc::now();
            if expires_at < now {
                return Err(ServiceError::Validation(
                    "Vendor subscription has expired. Please renew your subscription to sell services.".to_string()
                ));
            }
        } else {
            return Err(ServiceError::Validation(
                "Vendor subscription not found or not active.".to_string()
            ));
        }
        
        // 4. Validate buyer wallet exists and check balance first
        let buyer_wallet = self.db_client
            .get_naira_wallet(buyer_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Buyer wallet not found".to_string()))?;
        
        // 5. Calculate costs
        let unit_price = service.price.to_f64().unwrap_or(0.0);
        let subtotal = unit_price * dto.quantity as f64;
        let platform_fee = subtotal * 0.03; // 3% platform fee
        
        // Calculate delivery fee for cross-state orders
        let (delivery_fee, delivery_type) = match dto.delivery_type {
            DeliveryType::LocalPickup => (0.0, DeliveryType::LocalPickup),
            DeliveryType::CrossStateDelivery => {
                // Validate delivery address for cross-state
                if dto.delivery_address.is_none() || dto.delivery_state.is_none() {
                    return Err(ServiceError::Validation(
                        "Delivery address and state required for cross-state delivery".to_string()
                    ));
                }
                
                // Validate buyer identity for cross-state delivery
                let buyer = self.db_client
                    .get_user(Some(buyer_id), None, None, None)
                    .await?
                    .ok_or_else(|| ServiceError::Validation("Buyer not found".to_string()))?;
                
                if buyer.verification_status != Some(VerificationStatus::Approved){
                    return Err(ServiceError::Validation("Buyer account must be verified for cross-state delivery".to_string()));
                }
                
                // Calculate delivery fee based on distance/state
                let delivery_fee = self.calculate_delivery_fee(
                    &service.location_state,
                    dto.delivery_state.as_ref().unwrap(),
                ).await?;
                
                (delivery_fee, DeliveryType::CrossStateDelivery)
            },
            DeliveryType::DigitalDelivery => (0.0, DeliveryType::DigitalDelivery),
        };
        
        let total_amount = subtotal + platform_fee + delivery_fee;
        
        // For cross-state delivery, hold delivery fee until confirmation
        let (vendor_immediate_amount, held_amount) = if delivery_type == DeliveryType::CrossStateDelivery {
            (delivery_fee, subtotal) // Vendor gets delivery fee immediately, subtotal held
        } else {
            (subtotal, 0.0) // Local pickup - vendor gets everything immediately
        };
        
        // 6. Check buyer's wallet balance
        let total_amount_kobo = naira_to_kobo(total_amount);
        if buyer_wallet.available_balance < total_amount_kobo {
            return Err(ServiceError::Validation("Insufficient wallet balance".to_string()));
        }
        
        // 7. Generate order number
        let order_number = format!("ORD-{}", uuid::Uuid::new_v4().to_string()[..8].to_uppercase());
        
        // 8. Create order
        let order = sqlx::query_as::<_, ServiceOrder>(
            r#"
            INSERT INTO service_orders (
                order_number, service_id, vendor_id, buyer_id, quantity,
                unit_price, total_amount, platform_fee, vendor_amount,
                payment_reference, buyer_name, buyer_email, buyer_phone,
                delivery_type, delivery_fee, delivery_amount_held,
                delivery_address, delivery_state, delivery_city, notes, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, 'pending')
            RETURNING *
            "#
        )
        .bind(&order_number)
        .bind(service.id)
        .bind(service.vendor_id)
        .bind(buyer_id)
        .bind(dto.quantity)
        .bind(BigDecimal::try_from(unit_price).unwrap())
        .bind(BigDecimal::try_from(total_amount).unwrap())
        .bind(BigDecimal::try_from(platform_fee).unwrap())
        .bind(BigDecimal::try_from(vendor_immediate_amount).unwrap())
        .bind(&order_number) // Use order number as payment reference
        .bind(dto.buyer_name)
        .bind(dto.buyer_email)
        .bind(dto.buyer_phone)
        .bind(delivery_type.clone())
        .bind(BigDecimal::try_from(delivery_fee).unwrap())
        .bind(BigDecimal::try_from(held_amount).unwrap())
        .bind(dto.delivery_address)
        .bind(dto.delivery_state)
        .bind(dto.delivery_city)
        .bind(dto.notes)
        .fetch_one(&mut *tx)
        .await?;
        
        // 9. Debit buyer's wallet - THIS HOLDS THE FUNDS IN ESCROW
        let _wallet_tx = self.db_client
            .debit_wallet(
                buyer_id,
                total_amount_kobo,
                TransactionType::ServicePayment, // ✅ Use ServicePayment
                format!("Purchase: {}", service.title),
                order_number.clone(),
                None,
                Some(serde_json::json!({
                    "order_id": order.id,
                    "service_id": service.id,
                    "vendor_id": vendor.id,
                    "type": "escrow_hold"
                })),
            )
            .await?;

        tracing::info!("✓ Escrow hold created for order {}: {:.2} ₦", order.order_number, total_amount);
        
        // 10. Create escrow transaction record (HELD state - funds are locked)
        sqlx::query(
            r#"
            INSERT INTO escrow_transactions (
                order_id, service_id, vendor_id, buyer_id,
                total_amount, platform_fee, vendor_amount, held_amount,
                status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'held', NOW())
            "#
        )
        .bind(order.id)
        .bind(service.id)
        .bind(vendor.id)
        .bind(buyer_id)
        .bind(BigDecimal::try_from(total_amount).unwrap())
        .bind(BigDecimal::try_from(platform_fee).unwrap())
        .bind(BigDecimal::try_from(vendor_immediate_amount).unwrap())
        .bind(BigDecimal::try_from(held_amount).unwrap())
        .execute(&mut *tx)
        .await?;
        
        // 11. Update service stock
        sqlx::query(
            "UPDATE vendor_services SET stock_quantity = stock_quantity - $1 WHERE id = $2"
        )
        .bind(dto.quantity)
        .bind(service.id)
        .execute(&mut *tx)
        .await?;
        
        // 12. Update order status to paid
        let updated_order = sqlx::query_as::<_, ServiceOrder>(
            "UPDATE service_orders SET status = 'paid', paid_at = NOW() WHERE id = $1 RETURNING *"
        )
        .bind(order.id)
        .fetch_one(&mut *tx)
        .await?;
        
        tx.commit().await?;
        
        // 13. Send notifications
        if let Err(e) = self.notification_service
            .notify_service_purchase(vendor.user_id, buyer_id, &service, &updated_order)
            .await {
            tracing::error!("Failed to send purchase notification: {:?}", e);
        }
        
        Ok(ServiceOrderResponse {
            order: updated_order,
            service,
            vendor: VendorProfileSummary {
                id: vendor.id,
                business_name: vendor.business_name,
                rating: vendor.rating,
                total_sales: vendor.total_sales,
            },
            payment_info: PaymentInfo {
                total_amount,
                platform_fee,
                delivery_fee,
                vendor_amount: vendor_immediate_amount,
                held_amount,
            },
        })
    }
    
    
    async fn calculate_delivery_fee(
        &self,
        origin_state: &str,
        destination_state: &str,
    ) -> Result<f64, ServiceError> {
        // Simple distance-based calculation (you can make this more sophisticated)
        if origin_state == destination_state {
            Ok(2500.0) // Same state, no cross-state delivery
        } else {
            // Base delivery fee + distance multiplier
            // You could use a state distance matrix or API here
            Ok(7500.0) // Fixed â‚¦2,500 for cross-state delivery
        }
    }
    
    /// Confirm delivery and release held funds
    pub async fn confirm_delivery(
        &self,
        buyer_id: Uuid,
        dto: ConfirmDeliveryDto,
    ) -> Result<ServiceOrder, ServiceError> {
        let mut tx = self.db_client.pool.begin().await?;
        
        // Get order
        let order = self.db_client
            .get_order_by_id(dto.order_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Order not found".to_string()))?;
        
        // Verify buyer owns this order
        if order.buyer_id != buyer_id {
            return Err(ServiceError::UnauthorizedServiceAccess(buyer_id, order.id));
        }
        
        // Check if already confirmed
        if order.delivery_confirmed.unwrap_or(false) {
            return Err(ServiceError::Validation("Delivery already confirmed".to_string()));
        }
        
        // Only for cross-state deliveries with held amount
        if order.delivery_type == DeliveryType::CrossStateDelivery 
            && order.delivery_amount_held.is_some() 
        {
            let held_amount = order.delivery_amount_held.unwrap();
            let held_amount_f64 = held_amount.to_f64().unwrap_or(0.0);
            
            // Credit vendor with held amount
            let vendor = self.db_client
                .get_vendor_profile_by_id(order.vendor_id)
                .await?
                .ok_or_else(|| ServiceError::Validation("Vendor not found".to_string()))?;
            
            let held_amount_kobo = naira_to_kobo(held_amount_f64);
            
            self.db_client
                .credit_wallet(
                    vendor.user_id,
                    held_amount_kobo,
                    TransactionType::ServicePayment, // ✅ Use ServicePayment
                    format!("Delivery confirmed: Order {}", order.order_number),
                    format!("DELIVERY_CONF_{}", order.id),
                    None,
                    Some(serde_json::json!({
                        "order_id": order.id,
                        "type": "escrow_release"
                    })),
                )
                .await?;

            tracing::info!("✓ Escrow released for order {}: {:.2} ₦", order.order_number, held_amount_f64);

            // Update escrow status
            sqlx::query(
                "UPDATE escrow_transactions SET status = 'released', released_at = NOW() WHERE order_id = $1"
            )
            .bind(order.id)
            .execute(&mut *tx)
            .await?;
        }
        
        // Update order status
        let updated_order = sqlx::query_as::<_, ServiceOrder>(
            r#"
            UPDATE service_orders 
            SET status = 'completed',
                delivery_confirmed = true,
                delivery_confirmed_at = NOW(),
                completed_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(dto.order_id)
        .fetch_one(&mut *tx)
        .await?;
        
        // Create review if provided
        if let Some(rating) = dto.rating {
            let service = self.db_client
                .get_service(order.service_id)
                .await?
                .ok_or_else(|| ServiceError::Validation("Service not found".to_string()))?;
            
            let _ = self.db_client
                .create_service_review(
                    service.id,
                    order.vendor_id,
                    Some(order.id),
                    buyer_id,
                    rating,
                    dto.review_comment,
                )
                .await?;
            
            // Update service rating
            let _ = self.db_client.update_service_rating(order.service_id).await;
        }
        
        tx.commit().await?;
        
        // Send notification to vendor
        let vendor = self.db_client
            .get_vendor_profile_by_id(order.vendor_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Vendor not found".to_string()))?;
        
        if let Err(e) = self.notification_service
            .notify_delivery_confirmed(vendor.user_id, &updated_order)
            .await {
            tracing::error!("Failed to send delivery confirmation notification: {:?}", e);
        }
        
        Ok(updated_order)
    }
    
    /// Vendor marks order as shipped
    pub async fn mark_as_shipped(
        &self,
        vendor_user_id: Uuid,
        order_id: Uuid,
        tracking_info: Option<String>,
    ) -> Result<ServiceOrder, ServiceError> {
        // Verify vendor owns this order
        let order = self.db_client
            .get_order_by_id(order_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Order not found".to_string()))?;
        
        let vendor = self.db_client
            .get_vendor_profile_by_id(order.vendor_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Vendor not found".to_string()))?;
        
        if vendor.user_id != vendor_user_id {
            return Err(ServiceError::UnauthorizedServiceAccess(vendor_user_id, order_id));
        }
        
        // Update order status
        let updated_order = sqlx::query_as::<_, ServiceOrder>(
            r#"
            UPDATE service_orders 
            SET status = 'shipped'
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(order_id)
        .fetch_one(&self.db_client.pool)
        .await?;
        
        // Create delivery tracking entry
        if let Some(tracking) = tracking_info {
            sqlx::query(
                r#"
                INSERT INTO delivery_tracking (order_id, status, notes, updated_by)
                VALUES ($1, 'shipped', $2, $3)
                "#
            )
            .bind(order_id)
            .bind(tracking)
            .bind(vendor_user_id)
            .execute(&self.db_client.pool)
            .await?;
        }
        
        // Notify buyer
        let _ = self.notification_service
            .notify_order_shipped(order.buyer_id, &updated_order)
            .await;
        
        Ok(updated_order)
    }
    
    /// Create dispute for service order
    pub async fn create_service_dispute(
        &self,
        raised_by: Uuid,
        order_id: Uuid,
        reason: String,
        description: String,
        evidence_urls: Vec<String>,
    ) -> Result<ServiceDispute, ServiceError> {
        let mut tx = self.db_client.pool.begin().await?;
        
        // Get order
        let order = self.db_client
            .get_order_by_id(order_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Order not found".to_string()))?;
        
        // Verify user is involved in this order
        let is_buyer = order.buyer_id == raised_by;
        let vendor = self.db_client
            .get_vendor_profile_by_id(order.vendor_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Vendor not found".to_string()))?;
        
        let is_vendor = vendor.user_id == raised_by;
        
        if !is_buyer && !is_vendor {
            return Err(ServiceError::Validation("Not involved in this order".to_string()));
        }
        
        let against = if is_buyer { vendor.user_id } else { order.buyer_id };
        
        // Create dispute
        let dispute = sqlx::query_as::<_, ServiceDispute>(
            r#"
            INSERT INTO service_disputes (
                order_id, service_id, raised_by, against, reason, 
                description, evidence_urls, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'open')
            RETURNING *
            "#
        )
        .bind(order.id)
        .bind(order.service_id)
        .bind(raised_by)
        .bind(against)
        .bind(reason)
        .bind(description)
        .bind(&evidence_urls)
        .fetch_one(&mut *tx)
        .await?;
        
        // Update order status
        sqlx::query(
            "UPDATE service_orders SET status = 'disputed' WHERE id = $1"
        )
        .bind(order_id)
        .execute(&mut *tx)
        .await?;
        
        tx.commit().await?;
        
        // Notify both parties
        match self.notification_service
            .notify_service_dispute_created(raised_by, against, &dispute)
            .await {
            Ok(_) => tracing::info!("Dispute notification sent to {} and {}", raised_by, against),
            Err(e) => tracing::error!("Failed to notify dispute: {:?}", e),
        }
        
        Ok(dispute)
    }
    
    /// Auto-confirm delivery after timeout (7 days)
    pub async fn process_auto_confirmations(&self) -> Result<(), ServiceError> {
        let orders = sqlx::query_as::<_, ServiceOrder>(
            r#"
            SELECT * FROM service_orders 
            WHERE status = 'delivered'
            AND delivery_confirmed = false
            AND delivery_type = 'cross_state_delivery'
            AND paid_at < NOW() - INTERVAL '7 days'
            "#
        )
        .fetch_all(&self.db_client.pool)
        .await?;
        
        for order in orders {
            // Auto-confirm delivery
            let _ = self.confirm_delivery(
                order.buyer_id,
                ConfirmDeliveryDto {
                    order_id: order.id,
                    rating: None,
                    review_comment: None,
                },
            ).await;
            
            tracing::info!("Auto-confirmed delivery for order: {}", order.order_number);
        }
        
        Ok(())
    }
    
    /// Handle dispute resolution with partial or full refund
    pub async fn settle_dispute(
        &self,
        dispute_id: Uuid,
        admin_id: Uuid,
        resolution: DisputeResolution,
    ) -> Result<ServiceDispute, ServiceError> {
        let mut tx = self.db_client.pool.begin().await?;
        
        // Get dispute
        let dispute = sqlx::query_as::<_, ServiceDispute>(
            "SELECT * FROM service_disputes WHERE id = $1"
        )
        .bind(dispute_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ServiceError::Validation("Dispute not found".to_string()))?;
        
        // Get order for fund calculations
        let order = self.db_client
            .get_order_by_id(dispute.order_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Order not found".to_string()))?;
        
        let _buyer = self.db_client
            .get_user(Some(order.buyer_id), None, None, None)
            .await?
            .ok_or_else(|| ServiceError::Validation("Buyer not found".to_string()))?;
        
        let vendor = self.db_client
            .get_vendor_profile_by_id(order.vendor_id)
            .await?
            .ok_or_else(|| ServiceError::Validation("Vendor not found".to_string()))?;
        
        match resolution {
            DisputeResolution::FullRefund => {
                // Refund 100% to buyer, release no funds to vendor
                let order_amount_kobo = (order.total_amount.to_f64().unwrap_or(0.0) * 100.0) as i64;
                
                self.db_client
                    .credit_wallet(
                        order.buyer_id,
                        order_amount_kobo,
                        TransactionType::Refund,
                        format!("Dispute refund: Order {}", order.order_number),
                        format!("DISPUTE_REFUND_{}", dispute_id),
                        None,
                        Some(serde_json::json!({
                            "order_id": order.id,
                            "dispute_id": dispute_id,
                            "type": "dispute_refund"
                        })),
                    )
                    .await?;
                
                tracing::info!("✓ Full refund issued to buyer {} for order {}", order.buyer_id, order.order_number);
            }
            
            DisputeResolution::PartialRefund { percentage } => {
                // Validate percentage
                if percentage <= 0 || percentage > 100 {
                    return Err(ServiceError::Validation("Refund percentage must be between 1-100".to_string()));
                }
                
                // Calculate amounts
                let order_amount_f64 = order.total_amount.to_f64().unwrap_or(0.0);
                let refund_amount = (order_amount_f64 * (percentage as f64 / 100.0)) as i64;
                let vendor_amount = (order_amount_f64 * ((100 - percentage) as f64 / 100.0)) as i64;
                
                // Refund percentage to buyer
                self.db_client
                    .credit_wallet(
                        order.buyer_id,
                        refund_amount as i64,
                        TransactionType::Refund,
                        format!("Partial refund ({}%): Order {}", percentage, order.order_number),
                        format!("PARTIAL_REFUND_{}_{}", dispute_id, percentage),
                        None,
                        Some(serde_json::json!({
                            "order_id": order.id,
                            "dispute_id": dispute_id,
                            "refund_percentage": percentage,
                            "type": "partial_dispute_refund"
                        })),
                    )
                    .await?;
                
                // Release remaining amount to vendor
                self.db_client
                    .credit_wallet(
                        vendor.user_id,
                        vendor_amount as i64,
                        TransactionType::ServicePayment,
                        format!("Partial payment after dispute ({}%): Order {}", 100 - percentage, order.order_number),
                        format!("DISPUTE_VENDOR_PAY_{}", dispute_id),
                        None,
                        Some(serde_json::json!({
                            "order_id": order.id,
                            "dispute_id": dispute_id,
                            "vendor_percentage": 100 - percentage,
                            "type": "dispute_partial_release"
                        })),
                    )
                    .await?;
                
                tracing::info!("✓ Partial refund settled: {}% to buyer, {}% to vendor for order {}", 
                    percentage, 100 - percentage, order.order_number);
            }
            
            DisputeResolution::Dismissed => {
                // Release full held amount to vendor
                if let Some(held_amount) = order.delivery_amount_held {
                    let held_amount_kobo = (held_amount.to_f64().unwrap_or(0.0) * 100.0) as i64;
                    
                    self.db_client
                        .credit_wallet(
                            vendor.user_id,
                            held_amount_kobo,
                            TransactionType::ServicePayment,
                            format!("Dispute dismissed: Order {}", order.order_number),
                            format!("DISPUTE_DISMISSED_{}", dispute_id),
                            None,
                            Some(serde_json::json!({
                                "order_id": order.id,
                                "dispute_id": dispute_id,
                                "type": "dispute_dismissed_release"
                            })),
                        )
                        .await?;
                    
                    tracing::info!("✓ Dispute dismissed, full escrow released to vendor for order {}", order.order_number);
                }
            }
        }
        
        // Update dispute status
        let updated_dispute = sqlx::query_as::<_, ServiceDispute>(
            r#"
            UPDATE service_disputes 
            SET status = 'resolved',
                resolved_at = NOW(),
                resolution_notes = $2,
                resolved_by = $3
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(dispute_id)
        .bind(serde_json::to_string(&resolution).unwrap_or_default())
        .bind(admin_id)
        .fetch_one(&mut *tx)
        .await?;
        
        // Update order status
        sqlx::query("UPDATE service_orders SET status = 'disputed' WHERE id = $1")
            .bind(order.id)
            .execute(&mut *tx)
            .await?;
        
        // Update escrow transaction
        sqlx::query(
            "UPDATE escrow_transactions SET status = 'settled', settled_at = NOW() WHERE order_id = $1"
        )
        .bind(order.id)
        .execute(&mut *tx)
        .await?;
        
        tx.commit().await?;
        
        // Notify both parties of dispute resolution
        let _ = self.notification_service
            .notify_service_dispute_created(
                admin_id,
                order.buyer_id,
                &updated_dispute,
            )
            .await;
        
        tracing::info!("✓ Dispute {} resolved with {:?}", dispute_id, resolution);
        
        Ok(updated_dispute)
    }
}