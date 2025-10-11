// services/notification_service.rs
use std::sync::Arc;
use uuid::Uuid;
use serde::Serialize;
use chrono::{DateTime, Utc};

use crate::{
    db::db::DBClient,
    models::labourmodel::*,
    service::error::ServiceError,
};

#[derive(Debug, Clone)]
pub struct NotificationService {
    db_client: Arc<DBClient>,
    // In production, you'd have email service, push notification service, etc.
}

impl NotificationService {
    pub fn new(db_client: Arc<DBClient>) -> Self {
        Self { db_client }
    }

    pub async fn notify_new_job(&self, job: &Job) -> Result<(), ServiceError> {
        // In a real implementation, this would:
        // 1. Find workers in the same location and category
        // 2. Send push notifications/emails
        // 3. Store notifications in database
        
        // For now, we'll just log the notification
        tracing::info!(
            "New job notification: {} in {} for {} category",
            job.title,
            job.location_state,
            job.category.to_str()
        );

        self.store_notification(
            None, // Broadcast to multiple users
            "new_job".to_string(),
            Some(job.id),
            Some(serde_json::json!({
                "job_title": job.title,
                "location": job.location_state,
                "category": job.category.to_str(),
                "budget": job.budget
            })),
            format!("New job available: {}", job.title),
        ).await
    }

    pub async fn notify_job_assignment(
        &self,
        worker_id: Uuid,
        job: &Job,
    ) -> Result<(), ServiceError> {
        tracing::info!(
            "Job assignment notification: worker {} assigned to job {}",
            worker_id,
            job.id
        );

        self.store_notification(
            Some(worker_id),
            "job_assigned".to_string(),
            Some(job.id),
            Some(serde_json::json!({
                "job_title": job.title,
                "employer_id": job.employer_id
            })),
            format!("You've been assigned to job: {}", job.title),
        ).await
    }

    pub async fn notify_progress_update(
        &self,
        employer_id: Uuid,
        progress: &JobProgress,
    ) -> Result<(), ServiceError> {
        tracing::info!(
            "Progress update notification: job {} is {}% complete",
            progress.job_id,
            progress.progress_percentage
        );

        self.store_notification(
            Some(employer_id),
            "progress_update".to_string(),
            Some(progress.job_id),
            Some(serde_json::json!({
                "progress_percentage": progress.progress_percentage,
                "description": progress.description
            })),
            format!("Job progress: {}% complete", progress.progress_percentage),
        ).await
    }

    pub async fn notify_job_completion(
        &self,
        worker_id: Uuid,
        job: &Job,
    ) -> Result<(), ServiceError> {
        tracing::info!(
            "Job completion notification: worker {} completed job {}",
            worker_id,
            job.id
        );

        self.store_notification(
            Some(worker_id),
            "job_completed".to_string(),
            Some(job.id),
            Some(serde_json::json!({
                "job_title": job.title,
                "completion_status": "successful"
            })),
            format!("Job completed: {}", job.title),
        ).await
    }

    pub async fn notify_dispute_creation(
        &self,
        raised_by: Uuid,
        against: Uuid,
        dispute: &Dispute,
    ) -> Result<(), ServiceError> {
        // Notify both parties
        self.store_notification(
            Some(raised_by),
            "dispute_raised".to_string(),
            Some(dispute.job_id),
            Some(serde_json::json!({
                "dispute_id": dispute.id,
                "against": against
            })),
            "Dispute raised successfully".to_string(),
        ).await?;

        self.store_notification(
            Some(against),
            "dispute_against".to_string(),
            Some(dispute.job_id),
            Some(serde_json::json!({
                "dispute_id": dispute.id,
                "raised_by": raised_by,
                "reason": dispute.reason
            })),
            "Dispute raised against you".to_string(),
        ).await
    }

    pub async fn notify_dispute_resolution(
        &self,
        raised_by: Uuid,
        against: Uuid,
        dispute: &Dispute,
        decision: &str,
    ) -> Result<(), ServiceError> {
        // Notify both parties about resolution
        let message = format!("Dispute resolved: {}", decision);

        self.store_notification(
            Some(raised_by),
            "dispute_resolved".to_string(),
            Some(dispute.job_id),
            Some(serde_json::json!({
                "decision": decision,
                "resolution": dispute.resolution
            })),
            message.clone(),
        ).await?;

        self.store_notification(
            Some(against),
            "dispute_resolved".to_string(),
            Some(dispute.job_id),
            Some(serde_json::json!({
                "decision": decision,
                "resolution": dispute.resolution
            })),
            message,
        ).await
    }

    pub async fn notify_payment_release(
        &self,
        worker_id: Uuid,
        job_id: Uuid,
        amount: f64,
    ) -> Result<(), ServiceError> {
        self.store_notification(
            Some(worker_id),
            "payment_released".to_string(),
            Some(job_id),
            Some(serde_json::json!({
                "amount": amount,
                "currency": "NGN"
            })),
            format!("Payment of ₦{} released", amount),
        ).await
    }

    async fn store_notification(
        &self,
        user_id: Option<Uuid>,
        notification_type: String,
        job_id: Option<Uuid>,
        metadata: Option<serde_json::Value>,
        message: String,
    ) -> Result<(), ServiceError> {
        if let Some(uid) = user_id {
            sqlx::query!(
                r#"
                INSERT INTO notifications 
                (user_id, type, job_id, metadata, message, created_at)
                VALUES ($1, $2, $3, $4, $5, NOW())
                "#,
                uid,
                notification_type,
                job_id,
                metadata,
                message
            )
            .execute(&self.db_client.pool)
            .await?;
        }
        // For broadcast notifications, you'd insert for multiple users

        Ok(())
    }

    pub async fn get_user_notifications(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserNotification>, ServiceError> {
        let notifications = sqlx::query_as!(
            UserNotification,
            r#"
            SELECT id, user_id, type, job_id, metadata, message, is_read, created_at
            FROM notifications 
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit,
            offset
        )
        .fetch_all(&self.db_client.pool)
        .await?;

        Ok(notifications)
    }

    pub async fn mark_notification_read(
        &self,
        notification_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ServiceError> {
        sqlx::query!(
            r#"
            UPDATE notifications 
            SET is_read = true
            WHERE id = $1 AND user_id = $2
            "#,
            notification_id,
            user_id
        )
        .execute(&self.db_client.pool)
        .await?;

        Ok(())
    }

    pub async fn mark_all_notifications_read(
        &self,
        user_id: Uuid,
    ) -> Result<(), ServiceError> {
        sqlx::query!(
            r#"
            UPDATE notifications 
            SET is_read = true
            WHERE user_id = $1 AND is_read = false
            "#,
            user_id
        )
        .execute(&self.db_client.pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserNotification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub r#type: String,
    pub job_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub message: String,
    pub is_read: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
}