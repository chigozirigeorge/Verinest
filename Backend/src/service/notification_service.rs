// // services/notification_service.rs
// use std::sync::Arc;
// use uuid::Uuid;
// use serde::Serialize;
// use chrono::{DateTime, Utc};

// use crate::{
//     db::db::DBClient,
//     models::labourmodel::*,
//     service::error::ServiceError,
// };

// #[derive(Debug, Clone)]
// pub struct NotificationService {
//     db_client: Arc<DBClient>,
//     // In production, you'd have email service, push notification service, etc.
// }

// impl NotificationService {
//     pub fn new(db_client: Arc<DBClient>) -> Self {
//         Self { db_client }
//     }

//     // Generic helper used by all notify_* methods to persist a notification row
//     async fn store_notification(
//         &self,
//         user_id: Option<Uuid>,
//         notification_type: &str,
//         job_id: Option<Uuid>,
//         metadata: Option<serde_json::Value>,
//         message: &str,
//     ) -> Result<(), ServiceError> {
//         // If user_id is Some, insert a row for that user. If None, treat as broadcast and
//         // insert a row with NULL user_id (you can later query broadcasts separately or expand to
//         // insert per-recipient as needed).
//         sqlx::query(
//             r#"
//             INSERT INTO notifications 
//             (user_id, type, job_id, metadata, message, created_at)
//             VALUES ($1, $2, $3, $4, $5, NOW())
//             "#,
//         )
//         .bind(user_id)
//         .bind(notification_type)
//         .bind(job_id)
//         .bind(metadata)
//         .bind(message)
//         .execute(&self.db_client.pool)
//         .await?;

//         Ok(())
//     }

//     pub async fn notify_new_job(&self, job: &Job) -> Result<(), ServiceError> {
//         // Persist a broadcast notification (user_id = NULL)
//         self.store_notification(
//             None,
//             "new_job",
//             Some(job.id),
//             Some(serde_json::json!({
//                 "job_title": job.title,
//                 "location": job.location_state,
//                 "category": job.category.to_str(),
//                 "budget": job.budget
//             })),
//             &format!("New job available: {}", job.title),
//         )
//         .await
//     }

//     pub async fn notify_job_assignment(
//         &self,
//         worker_id: Uuid,
//         job: &Job,
//     ) -> Result<(), ServiceError> {
//         self.store_notification(
//             Some(worker_id),
//             "job_assigned",
//             Some(job.id),
//             Some(serde_json::json!({
//                 "job_title": job.title,
//                 "employer_id": job.employer_id
//             })),
//             &format!("You've been assigned to job: {}", job.title),
//         )
//         .await
//     }

//     pub async fn notify_progress_update(
//         &self,
//         employer_id: Uuid,
//         progress: &JobProgress,
//     ) -> Result<(), ServiceError> {
//         self.store_notification(
//             Some(employer_id),
//             "progress_update",
//             Some(progress.job_id),
//             Some(serde_json::json!({
//                 "progress_percentage": progress.progress_percentage,
//                 "description": progress.description
//             })),
//             &format!("Job progress: {}% complete", progress.progress_percentage),
//         )
//         .await
//     }

//     pub async fn notify_job_completion(
//         &self,
//         worker_id: Uuid,
//         job: &Job,
//     ) -> Result<(), ServiceError> {
//         self.store_notification(
//             Some(worker_id),
//             "job_completed",
//             Some(job.id),
//             Some(serde_json::json!({
//                 "job_title": job.title,
//                 "completion_status": "successful"
//             })),
//             &format!("Job completed: {}", job.title),
//         )
//         .await
//     }

//     pub async fn notify_dispute_creation(
//         &self,
//         raised_by: Uuid,
//         against: Uuid,
//         dispute: &Dispute,
//     ) -> Result<(), ServiceError> {
//         // Notify both parties
//         self.store_notification(
//             Some(raised_by),
//             "dispute_raised",
//             Some(dispute.job_id),
//             Some(serde_json::json!({
//                 "dispute_id": dispute.id,
//                 "against": against
//             })),
//             "Dispute raised successfully",
//         )
//         .await?;

//         self.store_notification(
//             Some(against),
//             "dispute_against",
//             Some(dispute.job_id),
//             Some(serde_json::json!({
//                 "dispute_id": dispute.id,
//                 "raised_by": raised_by,
//                 "reason": dispute.reason
//             })),
//             "Dispute raised against you",
//         )
//         .await
//     }

//     pub async fn notify_dispute_resolution(
//         &self,
//         raised_by: Uuid,
//         against: Uuid,
//         dispute: &Dispute,
//         decision: &str,
//     ) -> Result<(), ServiceError> {
//         // Notify both parties about resolution
//         let message = format!("Dispute resolved: {}", decision);

//         self.store_notification(
//             Some(raised_by),
//             "dispute_resolved",
//             Some(dispute.job_id),
//             Some(serde_json::json!({
//                 "decision": decision,
//                 "resolution": dispute.resolution
//             })),
//             &message,
//         )
//         .await?;

//         self.store_notification(
//             Some(against),
//             "dispute_resolved",
//             Some(dispute.job_id),
//             Some(serde_json::json!({
//                 "decision": decision,
//                 "resolution": dispute.resolution
//             })),
//             &message,
//         )
//         .await
//     }

//     pub async fn notify_payment_release(
//         &self,
//         worker_id: Uuid,
//         job_id: Uuid,
//         amount: f64,
//     ) -> Result<(), ServiceError> {
//         self.store_notification(
//             Some(worker_id),
//             "payment_released",
//             Some(job_id),
//             Some(serde_json::json!({
//                 "amount": amount,
//                 "currency": "NGN"
//             })),
//             &format!("Payment of ₦{} released", amount),
//         )
//         .await
//     }

//     pub async fn get_user_notifications(
//         &self,
//         user_id: Uuid,
//         limit: i64,
//         offset: i64,
//     ) -> Result<Vec<UserNotification>, ServiceError> {
//         let notifications = sqlx::query_as::<_, UserNotification>(
//             r#"
//             SELECT id, user_id, type, job_id, metadata, message, is_read, created_at
//             FROM notifications 
//             WHERE user_id = $1 OR user_id IS NULL
//             ORDER BY created_at DESC
//             LIMIT $2 OFFSET $3
//             "#,
//         )
//         .bind(user_id)
//         .bind(limit)
//         .bind(offset)
//         .fetch_all(&self.db_client.pool)
//         .await?;

//         Ok(notifications)
//     }

//     pub async fn mark_notification_read(
//         &self,
//         notification_id: Uuid,
//         user_id: Uuid,
//     ) -> Result<(), ServiceError> {
//         sqlx::query(
//             r#"
//             UPDATE notifications 
//             SET is_read = true
//             WHERE id = $1 AND (user_id = $2 OR user_id IS NULL)
//             "#,
//         )
//         .bind(notification_id)
//         .bind(user_id)
//         .execute(&self.db_client.pool)
//         .await?;

//         Ok(())
//     }

//     pub async fn mark_all_notifications_read(
//         &self,
//         user_id: Uuid,
//     ) -> Result<(), ServiceError> {
//         sqlx::query(
//             r#"
//             UPDATE notifications 
//             SET is_read = true
//             WHERE (user_id = $1 OR user_id IS NULL) AND is_read = false
//             "#,
//         )
//         .bind(user_id)
//         .execute(&self.db_client.pool)
//         .await?;

//         Ok(())
//     }

//     // In notification_service.rs - Add new notification methods
//     pub async fn notify_job_application(
//         &self,
//         employer_id: Uuid,
//         job: &Job,
//         applicant_name: &str,
//     ) -> Result<(), ServiceError> {
//         self.store_notification(
//             Some(employer_id),
//             "job_application",
//             Some(job.id),
//             Some(serde_json::json!({
//                 "job_title": job.title,
//                 "applicant_name": applicant_name,
//                 "job_id": job.id
//             })),
//             &format!("New application received for job: {}", job.title),
//         )
//         .await
//     }

//     pub async fn notify_job_assigned_to_worker(
//         &self,
//         worker_id: Uuid,
//         job: &Job,
//     ) -> Result<(), ServiceError> {
//         self.store_notification(
//             Some(worker_id),
//             "job_assigned",
//             Some(job.id),
//             Some(serde_json::json!({
//                 "job_title": job.title,
//                 "employer_id": job.employer_id,
//                 "escrow_created": true
//             })),
//             &format!("You've been assigned to job: {}", job.title),
//         )
//         .await
//     }

//     pub async fn notify_employer_worker_assigned(
//         &self,
//         employer_id: Uuid,
//         job: &Job,
//         worker_profile: &WorkerProfile,
//     ) -> Result<(), ServiceError> {
//         self.store_notification(
//             Some(employer_id),
//             "worker_assigned",
//             Some(job.id),
//             Some(serde_json::json!({
//                 "job_title": job.title,
//                 "worker_name": "Worker", // You might want to fetch worker name
//                 "worker_category": worker_profile.category.to_str(),
//                 "escrow_created": true
//             })),
//             &format!("Worker assigned to your job: {}", job.title),
//         )
//         .await
//     }

//     pub async fn notify_contract_awaiting_signature(
//         &self,
//         user_id: Uuid,
//         contract: &JobContract,
//     ) -> Result<(), ServiceError> {
//         let user_type = if user_id == contract.employer_id {
//             "employer"
//         } else {
//             "worker"
//         };

//         self.store_notification(
//             Some(user_id),
//             "contract_pending_signature",
//             Some(contract.job_id),
//             Some(serde_json::json!({
//                 "contract_id": contract.id,
//                 "user_type": user_type,
//                 "agreed_rate": contract.agreed_rate
//             })),
//             "Contract awaiting your signature - please review and sign",
//         )
//         .await
//     }

//     pub async fn notify_dispute_against_user(
//         &self,
//         user_id: Uuid,
//         dispute: &Dispute,
//         raised_by_name: &str,
//     ) -> Result<(), ServiceError> {
//         self.store_notification(
//             Some(user_id),
//             "dispute_raised_against",
//             Some(dispute.job_id),
//             Some(serde_json::json!({
//                 "dispute_id": dispute.id,
//                 "raised_by": raised_by_name,
//                 "reason": dispute.reason
//             })),
//             &format!("Dispute raised against you: {}", dispute.reason),
//         )
//         .await
//     }
// }

// #[derive(Debug, Serialize, sqlx::FromRow)]
// pub struct UserNotification {
//     pub id: Uuid,
//     pub user_id: Option<Uuid>,
//     pub r#type: String,
//     pub job_id: Option<Uuid>,
//     pub metadata: Option<serde_json::Value>,
//     pub message: String,
//     pub is_read: Option<bool>,
//     pub created_at: Option<DateTime<Utc>>,
// }









// service/notification_service.rs - Enhanced version with email integration

use std::sync::Arc;
use uuid::Uuid;

use crate::{
    models::{
        labourmodel::*,
        chatnodels::Message,
    },
    db::userdb::UserExt,
    db::db::DBClient,
    mail::mails,
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