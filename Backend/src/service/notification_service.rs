use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::{db::DBClient, userdb::UserExt}, mail::mails, 
    models::{
        chatnodels::Message, labourmodel::*, 
        vendormodels::{ServiceDispute, ServiceOrder, SubscriptionTier, VendorService}, 
        verificationmodels::VerificationDocument,
        usermodel::VerificationStatus,
    }
};
use crate::db::labourdb::LaborExt;

#[derive(Clone, Debug)]
pub struct NotificationService {
    db_client: Arc<DBClient>,
}

impl NotificationService {
    pub fn new(db_client: Arc<DBClient>) -> Self {
        Self { db_client }
    }
    
    // Helper to create notification and send email
    async fn create_notification_with_email(
        &self,
        user_id: Uuid,
        title: String,
        message: String,
        notification_type: String,
        related_id: Option<Uuid>,
        send_email: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create in-app notification
        sqlx::query(
            r#"
            INSERT INTO notifications (user_id, title, message, notification_type, related_id)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(user_id)
        .bind(&title)
        .bind(&message)
        .bind(&notification_type)
        .bind(related_id)
        .execute(&self.db_client.pool)
        .await?;
        
        // Send email if requested
        if send_email {
            if let Ok(Some(user)) = self.db_client.get_user(Some(user_id), None, None, None).await {
                // Send email based on notification type
                let _ = self.send_notification_email(&user.email, &user.name, &title, &message, &notification_type).await;
            }
        }
        
        Ok(())
    }
    
    async fn send_notification_email(
        &self,
        to_email: &str,
        username: &str,
        title: &str,
        message: &str,
        notification_type: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Use appropriate email template based on notification type
        match notification_type {
            "job_application" | "job_assigned" | "job_completion" => {
                // Use existing email functions from mails.rs
                Ok(())
            }
            _ => {
                // Send generic notification email
                Ok(())
            }
        }
    }
    
    // Job-related notifications with email
    pub async fn notify_new_job(&self, job: &Job) -> Result<(), Box<dyn std::error::Error>> {
        // Notify all workers in the same state and category
        let workers = self.db_client
            .get_workers_by_location_and_category(
                &job.location_state,
                job.category,
                100, // limit
                0,   // offset
            )
            .await?;
        
        for worker_profile in workers {
            self.create_notification_with_email(
                worker_profile.user_id,
                "New Job Available".to_string(),
                format!("A new {} job is available in {}: {}", 
                    job.category.to_str(), job.location_city, job.title),
                "new_job".to_string(),
                Some(job.id),
                false, // Don't send email for every new job
            ).await?;
        }
        
        Ok(())
    }
    
    pub async fn notify_job_application(
        &self,
        employer_id: Uuid,
        job: &Job,
        applicant_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get employer details
        if let Ok(Some(employer)) = self.db_client.get_user(Some(employer_id), None, None, None).await {
            // Create notification
            self.create_notification_with_email(
                employer_id,
                "New Job Application".to_string(),
                format!("{} has applied for your job: {}", applicant_name, job.title),
                "job_application".to_string(),
                Some(job.id),
                true, // Send email for applications
            ).await?;
            
            // Send dedicated email
            let _ = mails::send_job_application_email(
                &employer.email,
                &employer.name,
                &job.title,
                applicant_name,
            ).await;
        }
        
        Ok(())
    }

    pub async fn notify_application_rejected(
        &self,
        worker_id: Uuid,
        job: &Job,
        _application: &JobApplication,
        rejection_reason: &String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(Some(worker)) = self.db_client.get_user(Some(worker_id), None, None, None).await {
            self.create_notification_with_email(
                worker_id,
                "Application was Rejected".to_string(),
                format!("Unfortunately our esteemed worker your Application for {} was rejected due to {}, Make edits to your portfolio and keep hopes high and we would ensure to keep updating you with jobs close to you", job.title, rejection_reason),
                "job_assigned".to_string(),
                Some(job.id),
                true, // Send email for assignment
            ).await?;
            
            // Send dedicated email
            let _ = mails::send_job_assignment_email(
                &worker.email,
                &worker.name,
                &job.title,
            ).await;
        }
        
        Ok(())
    }

    pub async fn notify_verification_rejected(
        &self,
        verification: &VerificationDocument,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(Some(user)) = self.db_client.get_user(Some(verification.user_id), None, None, None).await {
            self.create_notification_with_email(
                user.id,
                "Verification request was Rejected".to_string(),
                format!("Unfortunately Dear {:?}, your Verification request was rejected due to {:?}",&user.name, verification.review_notes),
                "verification declined".to_string(),
                Some(verification.user_id),
                true, 
            ).await?;
            
            // Send dedicated email
            let _ = mails::send_verification_status_email(
                &user.email, 
                &user.username, 
                &crate::models::usermodel::VerificationStatus::Rejected,
                verification.review_notes.as_deref()
            ).await;
        }
        
        Ok(())
    }


    pub async fn notify_verification_accepted(
        &self,
        verification: &VerificationDocument,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(Some(user)) = self.db_client.get_user(Some(verification.user_id), None, None, None).await {
            self.create_notification_with_email(
                user.id,
                "Verification request was Rejected".to_string(),
                format!("Dear {:?} your Verification request has been accepted and you have been granted access to explore our platform, Please ensure to leave your honest feedback to help us serve you better",user.name),
                "verification declined".to_string(),
                Some(verification.user_id),
                true, 
            ).await?;
            
            // Send dedicated email
            let _ = mails::send_verification_status_email(
                &user.email, 
                &user.username, 
                &crate::models::usermodel::VerificationStatus::Approved,
                verification.review_notes.as_deref()
            ).await;
        }
        
        Ok(())
    }

    pub async fn notify_application_reviewed(
        &self,
        worker_id: Uuid,
        job: &Job,
        _application: &JobApplication,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(Some(worker)) = self.db_client.get_user(Some(worker_id), None, None, None).await {
            self.create_notification_with_email(
                worker_id,
                "Job Review".to_string(),
                format!("Your application is under Review"),
                "job_assigned".to_string(),
                Some(job.id),
                true, // Send email for assignment
            ).await?;
            
            // Send dedicated email
            let _ = mails::send_job_assignment_email(
                &worker.email,
                &worker.name,
                &job.title,
            ).await;
        }
        
        Ok(())
    }
    
    pub async fn notify_job_assigned_to_worker(
        &self,
        worker_id: Uuid,
        job: &Job,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(Some(worker)) = self.db_client.get_user(Some(worker_id), None, None, None).await {
            self.create_notification_with_email(
                worker_id,
                "Job Assignment".to_string(),
                format!("You have been assigned to the job: {}", job.title),
                "job_assigned".to_string(),
                Some(job.id),
                true, // Send email for assignment
            ).await?;
            
            // Send dedicated email
            let _ = mails::send_job_assignment_email(
                &worker.email,
                &worker.name,
                &job.title,
            ).await;
        }
        
        Ok(())
    }
    
    pub async fn notify_progress_update(
        &self,
        employer_id: Uuid,
        progress: &JobProgress,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            employer_id,
            "Job Progress Update".to_string(),
            format!("Worker updated progress to {}%: {}", 
                progress.progress_percentage, progress.description),
            "progress_update".to_string(),
            Some(progress.job_id),
            true, // Send email for progress updates
        ).await?;
        
        Ok(())
    }
    
    pub async fn notify_job_completion(
        &self,
        user_id: Uuid,
        job: &Job,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(Some(user)) = self.db_client.get_user(Some(user_id), None, None, None).await {
            self.create_notification_with_email(
                user_id,
                "Job Completed".to_string(),
                format!("The job '{}' has been marked as completed", job.title),
                "job_completion".to_string(),
                Some(job.id),
                true, // Send email for completion
            ).await?;
            
            // Send dedicated email
            let _ = mails::send_job_completion_email(
                &user.email,
                &user.name,
                &job.title,
            ).await;
        }
        
        Ok(())
    }
    
    pub async fn notify_payment_release(
        &self,
        worker_id: Uuid,
        job_id: Uuid,
        amount: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(Some(worker)) = self.db_client.get_user(Some(worker_id), None, None, None).await {
            self.create_notification_with_email(
                worker_id,
                "Payment Released".to_string(),
                format!("Payment of ₦{:.2} has been released for your work", amount),
                "payment_released".to_string(),
                Some(job_id),
                true, // Send email for payment release
            ).await?;
            
            // Send dedicated email
            let _ = mails::send_payment_released_email(
                &worker.email,
                &worker.name,
                amount,
            ).await;
        }
        
        Ok(())
    }
    
    // Dispute notifications with email
    pub async fn notify_dispute_creation(
        &self,
        raised_by: Uuid,
        against: Uuid,
        dispute: &Dispute,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Notify the party being disputed against
        self.create_notification_with_email(
            against,
            "Dispute Raised".to_string(),
            format!("A dispute has been raised against you: {}", dispute.reason),
            "dispute_created".to_string(),
            Some(dispute.id),
            true, // Send email for disputes
        ).await?;
        
        // Notify the party who raised the dispute (confirmation)
        self.create_notification_with_email(
            raised_by,
            "Dispute Created".to_string(),
            format!("Your dispute has been created and is under review"),
            "dispute_confirmation".to_string(),
            Some(dispute.id),
            true,
        ).await?;
        
        Ok(())
    }
    
    pub async fn notify_dispute_resolution(
        &self,
        raised_by: Uuid,
        against: Uuid,
        dispute: &Dispute,
        decision: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = format!("Dispute resolved: {}", decision);
        
        // Notify both parties
        for user_id in [raised_by, against] {
            self.create_notification_with_email(
                user_id,
                "Dispute Resolved".to_string(),
                message.clone(),
                "dispute_resolved".to_string(),
                Some(dispute.id),
                true, // Send email for resolution
            ).await?;
        }
        
        Ok(())
    }
    
    // Chat notifications
    pub async fn notify_new_message(
        &self,
        recipient_id: Uuid,
        sender_name: &str,
        message: &Message,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content_preview = if message.content.len() > 50 {
            format!("{}...", &message.content[..50])
        } else {
            message.content.clone()
        };
        
        self.create_notification_with_email(
            recipient_id,
            format!("New message from {}", sender_name),
            content_preview,
            "new_message".to_string(),
            Some(message.chat_id),
            false, // Don't send email for every message
        ).await?;
        
        Ok(())
    }
    
    pub async fn notify_contract_proposal(
        &self,
        recipient_id: Uuid,
        proposer_name: &str,
        job: &Job,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            recipient_id,
            "Contract Proposal".to_string(),
            format!("{} has sent you a contract proposal for: {}", proposer_name, job.title),
            "contract_proposal".to_string(),
            Some(job.id),
            true, // Send email for contract proposals
        ).await?;
        
        Ok(())
    }

    pub async fn notify_contract_fully_signed(
        &self,
        employer_id: Uuid,
        worker_id: Uuid,
        job: &Job,
    ) -> Result<(), Box<dyn std::error::Error>> {
       let _worker_email =  self.create_notification_with_email(
            worker_id, 
            "Contract Accepted".to_string(), 
            format!("Job Contract proposal for '{}' has been fully signed, please remember to update the job", job.title), 
            "contract_signed".to_string(), 
            Some(job.id), 
            true
        ).await?;

        let _employer_email =  self.create_notification_with_email(
            employer_id, 
            "Contract Accepted".to_string(), 
            format!("Job Contract proposal for '{}' has been fully signed, please remember to update the job", job.title), 
            "contract_signed".to_string(), 
            Some(job.id), 
            true
        ).await?;

        Ok(())
    }
    
    pub async fn notify_contract_accepted(
        &self,
        proposer_id: Uuid,
        job: &Job,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            proposer_id,
            "Contract Accepted".to_string(),
            format!("Your contract proposal for '{}' has been accepted", job.title),
            "contract_accepted".to_string(),
            Some(job.id),
            true, // Send email for acceptance
        ).await?;
        
        Ok(())
    }
    
    pub async fn notify_contract_rejected(
        &self,
        proposer_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            proposer_id,
            "Contract Declined".to_string(),
            "Your contract proposal has been declined".to_string(),
            "contract_rejected".to_string(),
            None,
            true, // Send email for rejection
        ).await?;
        
        Ok(())
    }
    
    // Wallet notifications with email
    pub async fn notify_wallet_transaction(
        &self,
        user_id: Uuid,
        transaction_type: &str,
        amount: f64,
        reference: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(Some(user)) = self.db_client.get_user(Some(user_id), None, None, None).await {
            let title = match transaction_type {
                "deposit" => "Deposit Successful",
                "withdrawal" => "Withdrawal Processed",
                "transfer_sent" => "Transfer Sent",
                "transfer_received" => "Transfer Received",
                _ => "Wallet Transaction",
            };
            
            self.create_notification_with_email(
                user_id,
                title.to_string(),
                format!("₦{:.2} - Ref: {}", amount, reference),
                transaction_type.to_string(),
                None,
                true, // Send email for wallet transactions
            ).await?;
            
            // Send dedicated emails based on type
            match transaction_type {
                "deposit" => {
                    let _ = mails::send_deposit_email(&user.email, &user.name, amount, reference).await;
                }
                "withdrawal" => {
                    let _ = mails::send_withdrawal_email(&user.email, &user.name, amount, reference).await;
                }
                "transfer_sent" => {
                    let _ = mails::send_transfer_email(&user.email, &user.name, amount, reference, "sent").await;
                }
                "transfer_received" => {
                    let _ = mails::send_transfer_email(&user.email, &user.name, amount, reference, "received").await;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    // Existing method for getting user notifications
    pub async fn get_user_notifications(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Notification>, sqlx::Error> {
        sqlx::query_as::<_, Notification>(
            r#"
            SELECT id, user_id, title, message, notification_type, related_id,
                   is_read, created_at
            FROM notifications
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db_client.pool)
        .await
    }
    
    pub async fn mark_notification_read(
        &self,
        notification_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE notifications
            SET is_read = true
            WHERE id = $1 AND user_id = $2
            "#
        )
        .bind(notification_id)
        .bind(user_id)
        .execute(&self.db_client.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn mark_all_notifications_read(
        &self,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE notifications
            SET is_read = true
            WHERE user_id = $1 AND is_read = false
            "#
        )
        .bind(user_id)
        .execute(&self.db_client.pool)
        .await?;
        
        Ok(())
    }

    pub async fn notify_employer_worker_assigned(
        &self,
        employer_id: Uuid,
        job: &Job,
        _worker_profile: &WorkerProfile,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            employer_id,
            format!("Worker assigned to job {}", job.title),
            format!("A worker has been assigned to your job: {}", job.title),
            "worker assigned to job".to_string(),
            None,
            true
        ).await?;
        Ok(())
    }

    pub async fn notify_contract_awaiting_signature(
        &self,
        user_id: Uuid,
        contract: &JobContract,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            user_id,
            "Contract needs signature".to_string(),
            format!("A new contract is awaiting your signature for job ID: {}", contract.job_id),
            "Contract awaiting signature".to_string(),
            None,
            true
        ).await?;
        Ok(())
    }

    //  pub async fn notify_subscription_upgraded(
    //     &self,
    //     user_id: Uuid,
    //     tier: SubscriptionTier,
    // ) -> Result<(), Box<dyn std::error::Error>> {
    //     let tier_name = match tier {
    //         SubscriptionTier::Normal => "Normal",
    //         SubscriptionTier::Pro => "Pro",
    //         SubscriptionTier::Premium => "Premium",
    //     };
        
    //     self.create_notification_with_email(
    //         user_id,
    //         format!("Subscription Upgraded to {}", tier_name),
    //         format!("Your vendor subscription has been upgraded to {} tier", tier_name),
    //         "subscription_upgraded".to_string(),
    //         None,
    //         true,
    //     ).await?;
        
    //     Ok(())
    // }

    pub async fn notify_new_order(
        &self,
        vendor_user_id: Uuid,
        service_title: &str,
        total_amount: f64,
        order_number: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            vendor_user_id,
            "New Order Received".to_string(),
            format!("New order #{} for '{}' - ₦{:.2}", order_number, service_title, total_amount),
            "new_order".to_string(),
            None,
            true,
        ).await?;
        
        Ok(())
    }

    pub async fn notify_order_placed(
        &self,
        buyer_id: Uuid,
        service_title: &str,
        total_amount: f64,
        order_number: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            buyer_id,
            "Order Placed Successfully".to_string(),
            format!("Your order #{} for '{}' has been placed - ₦{:.2}", order_number, service_title, total_amount),
            "order_placed".to_string(),
            None,
            true,
        ).await?;
        
        Ok(())
    }

    pub async fn notify_order_confirmed(
        &self,
        buyer_id: Uuid,
        service_title: &str,
        order_number: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            buyer_id,
            "Order Confirmed".to_string(),
            format!("Vendor confirmed your order #{} for '{}'", order_number, service_title),
            "order_confirmed".to_string(),
            None,
            true,
        ).await?;
        
        Ok(())
    }

    pub async fn notify_order_completed(
        &self,
        vendor_user_id: Uuid,
        service_title: &str,
        vendor_amount: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            vendor_user_id,
            "Order Completed - Payment Released".to_string(),
            format!("Order for '{}' completed. ₦{:.2} credited to your wallet", service_title, vendor_amount),
            "order_completed".to_string(),
            None,
            true,
        ).await?;
        
        Ok(())
    }
    
    pub async fn notify_service_inquiry(
        &self,
        vendor_user_id: Uuid,
        inquirer_name: &str,
        service_title: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(Some(user)) = self.db_client.get_user(Some(vendor_user_id), None, None, None).await {
            self.create_notification_with_email(
                vendor_user_id,
                "New Service Inquiry".to_string(),
                format!("{} is interested in your service: {}", inquirer_name, service_title),
                "service_inquiry".to_string(),
                None,
                true,
            ).await?;
            
            // Send dedicated email
            let _ = mails::send_service_inquiry_email(
                &user.email,
                &user.name,
                inquirer_name,
                service_title,
            ).await;
        }
        
        Ok(())
    }
    
    pub async fn notify_service_expiring(
        &self,
        vendor_user_id: Uuid,
        service_title: &str,
        days_until_expiry: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            vendor_user_id,
            "Service Expiring Soon".to_string(),
            format!("Your service '{}' will expire in {} days", service_title, days_until_expiry),
            "service_expiring".to_string(),
            None,
            true,
        ).await?;
        
        Ok(())
    }
    
    pub async fn notify_subscription_expiring(
        &self,
        vendor_user_id: Uuid,
        days_until_expiry: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            vendor_user_id,
            "Subscription Expiring Soon".to_string(),
            format!("Your vendor subscription will expire in {} days. Renew now to avoid service interruptions.", days_until_expiry),
            "subscription_expiring".to_string(),
            None,
            true,
        ).await?;
        
        Ok(())
    }

    pub async fn notify_service_purchase(
        &self,
        vendor_user_id: Uuid,
        buyer_id: Uuid,
        service: &VendorService,
        order: &ServiceOrder,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Notify vendor
        self.create_notification_with_email(
            vendor_user_id,
            "New Order Received".to_string(),
            format!("New order for: {} (Qty: {})", service.title, order.quantity),
            "service_order".to_string(),
            Some(order.id),
            true,
        ).await?;
        
        // Notify buyer (confirmation)
        self.create_notification_with_email(
            buyer_id,
            "Order Confirmed".to_string(),
            format!("Your order for '{}' has been confirmed", service.title),
            "order_confirmation".to_string(),
            Some(order.id),
            true,
        ).await?;
        
        Ok(())
    }
    
    pub async fn notify_order_shipped(
        &self,
        buyer_id: Uuid,
        order: &ServiceOrder,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            buyer_id,
            "Order Shipped".to_string(),
            format!("Your order #{} has been shipped", order.order_number),
            "order_shipped".to_string(),
            Some(order.id),
            true,
        ).await?;
        
        Ok(())
    }
    
    pub async fn notify_delivery_confirmed(
        &self,
        vendor_user_id: Uuid,
        order: &ServiceOrder,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            vendor_user_id,
            "Delivery Confirmed".to_string(),
            format!("Buyer confirmed delivery for order #{}", order.order_number),
            "delivery_confirmed".to_string(),
            Some(order.id),
            true,
        ).await?;
        
        Ok(())
    }
    
    pub async fn notify_service_dispute_created(
        &self,
        raised_by: Uuid,
        against: Uuid,
        dispute: &ServiceDispute,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Notify party being disputed against
        self.create_notification_with_email(
            against,
            "Service Dispute Raised".to_string(),
            format!("A dispute has been raised for order: {}", dispute.reason),
            "service_dispute".to_string(),
            Some(dispute.id),
            true,
        ).await?;
        
        // Confirm to party who raised it
        self.create_notification_with_email(
            raised_by,
            "Dispute Created".to_string(),
            "Your dispute has been submitted and is under review".to_string(),
            "dispute_confirmation".to_string(),
            Some(dispute.id),
            true,
        ).await?;
        
        Ok(())
    }

    pub async fn notify_subscription_upgraded(
        &self,
        vendor_id: Uuid,
        subscription_tier: SubscriptionTier
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_notification_with_email(
            vendor_id, 
            "Subscription upgraded".to_string(), 
            format!("Subscription upgrade to {:?} was successful", subscription_tier), 
            "service upgrade".to_string(), 
            None, 
            true
        ).await?;

        Ok(())
    }
    
    // pub async fn notify_contract_fully_signed(
    //     &self,
    //     employer_id: Uuid,
    //     worker_id: Uuid,
    //     job: &Job,
    // ) -> Result<(), Box<dyn std::error::Error>> {
    //     // Notify employer
    //     self.create_notification_with_email(
    //         employer_id,
    //         "Contract Activated".to_string(),
    //         format!("Both parties have signed the contract for: {}", job.title),
    //         "contract_active".to_string(),
    //         Some(job.id),
    //         true,
    //     ).await?;
        
    //     // Notify worker
    //     self.create_notification_with_email(
    //         worker_id,
    //         "Contract Activated".to_string(),
    //         format!("Contract is now active for: {}", job.title),
    //         "contract_active".to_string(),
    //         Some(job.id),
    //         true,
    //     ).await?;
        
    //     Ok(())
    // }

    // pub async fn notify_job_assigned_to_worker(
    //     &self,
    //     worker_id: Uuid,
    //     job: &Job,
    // ) -> Result<(), Box<dyn std::error::Error>> {
    //     self.create_notification_with_email(
    //         worker_id,
    //         "New job assignment".to_string(),
    //         format!("You have been assigned to job: {}", job.title),
    //         "Job assigned notification".to_string(),
    //         None,
    //         true,
    //     ).await?;
    //     Ok(())
    // }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub related_id: Option<Uuid>,
    pub is_read: Option<bool>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}