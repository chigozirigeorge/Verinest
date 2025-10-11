// services/dispute_service.rs
use uuid::Uuid;
use serde::Serialize;
use std::sync::Arc;

use crate::{
    db::{
        db::DBClient,
        labourdb::LaborExt,
        userdb::UserExt,
    }, 
    models::labourmodel::*, 
    service::{
        audit_service::AuditService, 
        error::ServiceError,
        escrow_service::{EscrowService, DisputeResolution as EscrowDisputeResolution}, 
        notification_service::NotificationService,
        trust_service::TrustService
    }

};

#[derive(Debug, Clone)]
pub struct DisputeService {
    db_client: Arc<DBClient>,
    escrow_service: Arc<EscrowService>,
    notification_service: Arc<NotificationService>,
    audit_service: Arc<AuditService>,
    trust_service: Arc<TrustService>,
}

impl DisputeService {
    pub fn new(
        db_client: Arc<DBClient>,
        escrow_service: Arc<EscrowService>,
        notification_service: Arc<NotificationService>,
        audit_service: Arc<AuditService>,
        trust_service: Arc<TrustService>
    ) -> Self {
        Self {
            db_client,
            escrow_service,
            notification_service,
            audit_service,
            trust_service
        }
    }

    pub async fn create_dispute(
        &self,
        job_id: Uuid,
        raised_by: Uuid,
        against: Uuid,
        reason: String,
        description: String,
        evidence_urls: Vec<String>,
    ) -> Result<DisputeCreationResult, ServiceError> {
        let mut tx = self.db_client.pool.begin().await?;

        // Verify job exists and user is involved
        let job = self.db_client.get_job_by_id(job_id)
            .await?
            .ok_or(ServiceError::JobNotFound(job_id))?;

        let is_involved = job.employer_id == raised_by || 
            job.assigned_worker_id == Some(raised_by);
        
        if !is_involved {
            return Err(ServiceError::UnauthorizedJobAccess(raised_by, job_id));
        }

        // Create dispute
        let dispute = self.db_client.create_dispute(
            job_id,
            raised_by,
            against,
            reason,
            description,
            evidence_urls,
        ).await?;

        // Update job status to disputed
        let disputed_job = self.db_client.update_job_status(job_id, JobStatus::Disputed).await?;

        // Handle escrow for dispute
        let escrow = self.db_client.get_escrow_by_job_id(job_id).await?
            .ok_or(ServiceError::Validation("Escrow not found for job".to_string()))?;

        let frozen_escrow = self.escrow_service.handle_dispute(escrow.id, dispute.id).await?;

        // Assign to available verifier
        let assigned_dispute = self.assign_to_verifier(dispute.id).await?;

        // Audit log
        self.audit_service.log_dispute_creation(
            raised_by,
            against,
            &assigned_dispute,
        ).await?;

        tx.commit().await?;

        // Notify both parties and verifiers
        self.notification_service.notify_dispute_creation(
            raised_by,
            against,
            &assigned_dispute,
        ).await?;

        Ok(DisputeCreationResult {
            dispute: assigned_dispute,
            job: disputed_job,
            escrow: frozen_escrow,
        })
    }

    pub async fn resolve_dispute(
        &self,
        dispute_id: Uuid,
        verifier_id: Uuid,
        resolution: String,
        decision: String,
        payment_percentage: Option<f64>,
    ) -> Result<DisputeResolutionResult, ServiceError> {
        let mut tx = self.db_client.pool.begin().await?;

        // Verify dispute exists and is assigned to verifier
        let dispute = self.db_client.get_dispute_by_id(dispute_id)
            .await?
            .ok_or(ServiceError::DisputeNotFound(dispute_id))?;

        if dispute.assigned_verifier != Some(verifier_id) {
            return Err(ServiceError::Validation("Dispute not assigned to this verifier".to_string()));
        }

        if dispute.status != Some(DisputeStatus::UnderReview) {
            return Err(ServiceError::InvalidDisputeStatus(dispute_id, dispute.status.unwrap()));
        }

        // Resolve dispute
        let resolved_dispute = self.db_client.resolve_dispute(
            dispute_id,
            resolution,
            decision.clone(),
        ).await?;

        // Handle escrow based on decision
        let escrow = self.db_client.get_escrow_by_job_id(dispute.job_id).await?
            .ok_or(ServiceError::Validation("Escrow not found".to_string()))?;

        let dispute_resolution = match decision.as_str() {
            "favor_employer" => DisputeResolution::FavorEmployer,
            "favor_worker" => DisputeResolution::FavorWorker { 
                payment_percentage: payment_percentage.unwrap_or(100.0) 
            },
            "partial_payment" => DisputeResolution::FavorWorker { 
                payment_percentage: payment_percentage.unwrap_or(50.0) 
            },
            _ => return Err(ServiceError::Validation("Invalid decision".to_string())),
        };

        let resolved_escrow = self.escrow_service.resolve_dispute(escrow.id, dispute_resolution.to_escrow_resolution()).await?;

        // Update job status based on resolution
        let job_status = match decision.as_str() {
            "favor_employer" => JobStatus::Cancelled,
            "favor_worker" | "partial_payment" => JobStatus::Completed,
            _ => JobStatus::Cancelled,
        };

        let updated_job = self.db_client.update_job_status(dispute.job_id, job_status).await?;

        // Award trust points based on resolution
        self.handle_trust_points_after_dispute(&dispute, &decision).await?;

        // Audit log
        self.audit_service.log_dispute_resolution(
            verifier_id,
            &resolved_dispute,
            &decision,
        ).await?;

        tx.commit().await?;

        // Notify both parties
        self.notification_service.notify_dispute_resolution(
            dispute.raised_by,
            dispute.against,
            &resolved_dispute,
            &decision,
        ).await?;

        Ok(DisputeResolutionResult {
            dispute: resolved_dispute,
            job: updated_job,
            escrow: resolved_escrow,
        })
    }

    async fn assign_to_verifier(&self, dispute_id: Uuid) -> Result<Dispute, ServiceError> {
        // Simple round-robin assignment - in production, use more sophisticated algorithm
        let verifiers = self.db_client.get_available_verifiers().await
            .map_err(|e| ServiceError::Database(e))?;
        
        // Assign verifier and return updated dispute
        if let Some(verifier) = verifiers.first() {
            self.db_client.assign_verifer_to_dispute(dispute_id, verifier.id).await
                .map_err(|e| ServiceError::Database(e))?;
        } else {
            // If no verifiers available, assign to admin (fallback)
            let admin = self.db_client.get_admin_user().await
                .map_err(|e| ServiceError::Database(e))?
                .ok_or_else(|| ServiceError::Validation("No admin user found".to_string()))?;
            
            self.db_client.assign_verifier_to_dispute(dispute_id, admin.id).await
                .map_err(|e| ServiceError::Database(e))?;
        }
        
        // Return the updated dispute
        self.db_client.get_dispute_by_id(dispute_id).await
            .map_err(|e| ServiceError::Database(e))?
            .ok_or_else(|| ServiceError::DisputeNotFound(dispute_id))
    }

    async fn handle_trust_points_after_dispute(
        &self,
        dispute: &Dispute,
        decision: &str,
    ) -> Result<(), ServiceError> {
        match decision {
            "favor_employer" => {
                // Worker loses points for causing dispute
                self.trust_service.deduct_trust_points(
                    dispute.against,
                    10,
                    "Lost dispute".to_string(),
                ).await?;
            }
            "favor_worker" => {
                // Employer loses points for false dispute
                self.trust_service.deduct_trust_points(
                    dispute.raised_by,
                    10,
                    "False dispute raised".to_string(),
                ).await?;
            }
            "partial_payment" => {
                // Both parties share responsibility
                self.trust_service.deduct_trust_points(
                    dispute.raised_by,
                    5,
                    "Partial responsibility in dispute".to_string(),
                ).await?;
                self.trust_service.deduct_trust_points(
                    dispute.against,
                    5,
                    "Partial responsibility in dispute".to_string(),
                ).await?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct DisputeCreationResult {
    pub dispute: Dispute,
    pub job: Job,
    pub escrow: EscrowTransaction,
}

#[derive(Debug, Serialize)]
pub struct DisputeResolutionResult {
    pub dispute: Dispute,
    pub job: Job,
    pub escrow: EscrowTransaction,
}

pub enum DisputeResolution {
    FavorEmployer,
    FavorWorker { payment_percentage: f64 },
}

impl DisputeResolution {
    fn to_escrow_resolution(self) -> EscrowDisputeResolution {
        match self {
            DisputeResolution::FavorEmployer => EscrowDisputeResolution::FavorEmployer,
            DisputeResolution::FavorWorker { payment_percentage } => EscrowDisputeResolution::FavorWorker { payment_percentage },
        }
    }
}
