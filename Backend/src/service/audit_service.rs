// services/audit_service.rs
use std::sync::Arc;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde_json;
use serde::Serialize;

use crate::{
    db::db::DBClient,
    models::labourmodel::*,
    service::error::ServiceError,
};

#[derive(Debug, Clone)]
pub struct AuditService {
    db_client: Arc<DBClient>,
}

impl AuditService {
    pub fn new(db_client: Arc<DBClient>) -> Self {
        Self { db_client }
    }

    pub async fn log_job_creation(
        &self,
        employer_id: Uuid,
        job: &Job,
    ) -> Result<(), ServiceError> {
        self.log_audit_event(
            employer_id,
            "job_creation".to_string(),
            Some(job.id),
            None,
            Some(serde_json::json!({
                "job_title": job.title,
                "budget": job.budget,
            })),
            "Job created without escrow".to_string(),
        ).await
    }

    pub async fn log_job_assignment(
        &self,
        employer_id: Uuid,
        worker_id: Uuid,
        job: &Job,
        contract: &JobContract,
    ) -> Result<(), ServiceError> {
        self.log_audit_event(
            employer_id,
            "job_assignment".to_string(),
            Some(job.id),
            Some(worker_id),
            Some(serde_json::json!({
                "contract_id": contract.id,
                "agreed_rate": contract.agreed_rate,
                "agreed_timeline": contract.agreed_timeline
            })),
            "Worker assigned to job".to_string(),
        ).await
    }

    pub async fn log_progress_submission(
        &self,
        worker_id: Uuid,
        progress: &JobProgress,
        payment_release: Option<&EscrowTransaction>,
    ) -> Result<(), ServiceError> {
        let mut metadata = serde_json::json!({
            "progress_percentage": progress.progress_percentage,
            "description": progress.description
        });

        if let Some(payment) = payment_release {
            metadata["payment_released"] = serde_json::json!(true);
            metadata["payment_amount"] = serde_json::json!(payment.amount);
        }

        self.log_audit_event(
            worker_id,
            "progress_submission".to_string(),
            Some(progress.job_id),
            None,
            Some(metadata),
            "Job progress submitted".to_string(),
        ).await
    }

    pub async fn log_job_completion(
        &self,
        employer_id: Uuid,
        job: &Job,
        payment: &EscrowTransaction,
    ) -> Result<(), ServiceError> {
        self.log_audit_event(
            employer_id,
            "job_completion".to_string(),
            Some(job.id),
            job.assigned_worker_id,
            Some(serde_json::json!({
                "final_payment": payment.amount,
                "completion_status": "successful"
            })),
            "Job completed and payment released".to_string(),
        ).await
    }

    pub async fn log_dispute_creation(
        &self,
        raised_by: Uuid,
        against: Uuid,
        dispute: &Dispute,
    ) -> Result<(), ServiceError> {
        self.log_audit_event(
            raised_by,
            "dispute_creation".to_string(),
            Some(dispute.job_id),
            Some(against),
            Some(serde_json::json!({
                "dispute_id": dispute.id,
                "reason": dispute.reason,
                "evidence_count": dispute.evidence_urls.as_ref().map_or(0, |e| e.len())
            })),
            "Dispute raised".to_string(),
        ).await
    }

    pub async fn log_dispute_resolution(
        &self,
        verifier_id: Uuid,
        dispute: &Dispute,
        decision: &str,
    ) -> Result<(), ServiceError> {
        self.log_audit_event(
            verifier_id,
            "dispute_resolution".to_string(),
            Some(dispute.job_id),
            None,
            Some(serde_json::json!({
                "dispute_id": dispute.id,
                "decision": decision,
                "resolution": dispute.resolution
            })),
            format!("Dispute resolved: {}", decision),
        ).await
    }

    pub async fn log_escrow_activity(
        &self,
        user_id: Uuid,
        job_id: Uuid,
        activity_type: &str,
        amount: Option<f64>,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), ServiceError> {
        let mut meta = metadata.unwrap_or(serde_json::json!({}));
        if let Some(amt) = amount {
            meta["amount"] = serde_json::json!(amt);
        }

        self.log_audit_event(
            user_id,
            format!("escrow_{}", activity_type),
            Some(job_id),
            None,
            Some(meta),
            format!("Escrow {} activity", activity_type),
        ).await
    }

    pub async fn log_payment_activity(
        &self,
        user_id: Uuid,
        job_id: Uuid,
        transaction_type: &str,
        amount: f64,
        status: &str,
    ) -> Result<(), ServiceError> {
        self.log_audit_event(
            user_id,
            format!("payment_{}", transaction_type),
            Some(job_id),
            None,
            Some(serde_json::json!({
                "amount": amount,
                "status": status,
                "transaction_type": transaction_type
            })),
            format!("Payment {}: {}", transaction_type, status),
        ).await
    }

    async fn log_audit_event(
        &self,
        user_id: Uuid,
        event_type: String,
        job_id: Option<Uuid>,
        related_user_id: Option<Uuid>,
        metadata: Option<serde_json::Value>,
        description: String,
    ) -> Result<(), ServiceError> {
        sqlx::query!(
            r#"
            INSERT INTO audit_logs 
            (user_id, event_type, job_id, related_user_id, metadata, description, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            "#,
            user_id,
            event_type,
            job_id,
            related_user_id,
            metadata,
            description
        )
        .execute(&self.db_client.pool)
        .await?;

        Ok(())
    }

    pub async fn get_audit_logs_for_job(
        &self,
        job_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLog>, ServiceError> {
        let logs = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT id, user_id, event_type, job_id, related_user_id, metadata, description, created_at
            FROM audit_logs 
            WHERE job_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            job_id,
            limit,
            offset
        )
        .fetch_all(&self.db_client.pool)
        .await?;

        Ok(logs)
    }

    pub async fn get_audit_logs_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLog>, ServiceError> {
        let logs = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT id, user_id, event_type, job_id, related_user_id, metadata, description, created_at
            FROM audit_logs 
            WHERE user_id = $1 OR related_user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit,
            offset
        )
        .fetch_all(&self.db_client.pool)
        .await?;

        Ok(logs)
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Uuid,
    pub event_type: String,
    pub job_id: Option<Uuid>,
    pub related_user_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub description: String,
    pub created_at: Option<DateTime<Utc>>,
}