// services/labour_service.rs
use std::sync::Arc;
use uuid::Uuid;
use serde::Serialize;
use num_traits::ToPrimitive;

use crate::{
    db::db::DBClient,
    models::labourmodel::*,
    db::labourdb::LaborExt,
    service::{
        escrow_service::EscrowService,
        trust_service::TrustService,
        notification_service::NotificationService,
        audit_service::AuditService,
        error::ServiceError,
    },
    dtos::labordtos::*,
};

#[derive(Debug, Clone)]
pub struct LabourService {
    db_client: Arc<DBClient>,
    escrow_service: Arc<EscrowService>,
    trust_service: Arc<TrustService>,
    notification_service: Arc<NotificationService>,
    audit_service: Arc<AuditService>,
}

impl LabourService {
    pub fn new(
        db_client: Arc<DBClient>,
        escrow_service: Arc<EscrowService>,
        trust_service: Arc<TrustService>,
        notification_service: Arc<NotificationService>,
        audit_service: Arc<AuditService>,
    ) -> Self {
        Self {
            db_client,
            escrow_service,
            trust_service,
            notification_service,
            audit_service,
        }
    }

    // In labour_service.rs - Update create_job_with_escrow method
pub async fn create_job_with_escrow(
    &self,
    employer_id: Uuid,
    job_data: CreateJobDto,
) -> Result<Job, ServiceError> {
    let job = self.db_client.create_job(
        employer_id,
        job_data.category,
        job_data.title,
        job_data.description,
        job_data.location_state,
        job_data.location_city,
        job_data.location_address,
        job_data.budget,
        job_data.estimated_duration_days,
        job_data.partial_payment_allowed,
        job_data.partial_payment_percentage,
        job_data.deadline,
    ).await?;

    // Audit log
    self.audit_service.log_job_creation(
        employer_id,
        &job, // No escrow yet
    ).await?;

    // Notify relevant workers
    self.notification_service.notify_new_job(&job).await?;

    Ok(job)
}


pub async fn assign_worker_to_job(
    &self,
    job_id: Uuid,
    employer_id: Uuid,
    worker_user_id: Uuid, // This should be the USER ID
) -> Result<JobAssignmentResult, ServiceError> {
    let mut tx = self.db_client.pool.begin().await?;

    // Verify job exists and belongs to employer
    let job = self.db_client.get_job_by_id(job_id)
        .await?
        .ok_or(ServiceError::JobNotFound(job_id))?;

    if job.employer_id != employer_id {
        return Err(ServiceError::UnauthorizedJobAccess(employer_id, job_id));
    }

    if job.status != Some(JobStatus::Open) {
        return Err(ServiceError::InvalidJobStatus(job_id, job.status.unwrap()));
    }

    // Verify worker exists and is available - use get_worker_profile which expects USER ID
    let worker_profile = self.db_client.get_worker_profile(worker_user_id)
        .await?;

    if !worker_profile.is_available.unwrap_or(false) {
        return Err(ServiceError::Validation("Worker is not available".to_string()));
    }

    // Update job with assigned worker AND create escrow in one transaction
    // The db_client.assign_worker_to_job expects USER ID for assignment
    let (updated_job, escrow) = self.db_client.assign_worker_to_job(job_id, worker_user_id).await?;

    // Create job contract - use worker_user_id (USER ID)
    let contract = self.db_client.create_job_contract(
        job_id,
        employer_id,
        worker_user_id, // Use USER ID here
        job.budget.to_f64().unwrap_or(0.0),
        job.estimated_duration_days,
        "Standard work agreement terms".to_string(),
    ).await?;

    // Audit log
    self.audit_service.log_job_assignment(
        employer_id,
        worker_user_id, // Use USER ID here
        &updated_job,
        &contract,
    ).await?;

    tx.commit().await?;

    // Send notifications - use worker_user_id (USER ID)
    self.notification_service.notify_job_assigned_to_worker(worker_user_id, &updated_job).await?;
    self.notification_service.notify_employer_worker_assigned(employer_id, &updated_job, &worker_profile).await?;
    self.notification_service.notify_contract_awaiting_signature(worker_user_id, &contract).await?;
    self.notification_service.notify_contract_awaiting_signature(employer_id, &contract).await?;

    Ok(JobAssignmentResult {
        job: updated_job,
        contract,
        escrow,
    })
}

    // pub async fn create_job_with_escrow(
    //     &self,
    //     employer_id: Uuid,
    //     job_data: CreateJobDto,
    // ) -> Result<(Job, EscrowTransaction), ServiceError> {
    //     let mut tx = self.db_client.pool.begin().await?;

    //     // Create job
    //     let job = self.db_client.create_job(
    //         employer_id,
    //         job_data.category,
    //         job_data.title,
    //         job_data.description,
    //         job_data.location_state,
    //         job_data.location_city,
    //         job_data.location_address,
    //         job_data.budget,
    //         job_data.estimated_duration_days,
    //         job_data.partial_payment_allowed,
    //         job_data.partial_payment_percentage,
    //         job_data.deadline,
    //     ).await?;

    //     // Create escrow transaction
    //     let escrow_amount = job_data.budget;
    //     let platform_fee = escrow_amount * 0.03; // 3% platform fee
        
    //     let escrow = self.escrow_service.create_escrow(
    //         job.id,
    //         employer_id,
    //         escrow_amount,
    //         platform_fee,
    //         job_data.partial_payment_allowed,
    //         job_data.partial_payment_percentage,
    //     ).await?;

    //     // Audit log
    //     self.audit_service.log_job_creation(
    //         employer_id,
    //         &job,
    //         &escrow,
    //     ).await?;

    //     tx.commit().await?;

    //     // Notify relevant workers
    //     self.notification_service.notify_new_job(&job).await?;

    //     Ok((job, escrow))
    // }

    // pub async fn assign_worker_to_job(
    //     &self,
    //     job_id: Uuid,
    //     employer_id: Uuid,
    //     worker_id: Uuid,
    // ) -> Result<JobAssignmentResult, ServiceError> {
    //     let mut tx = self.db_client.pool.begin().await?;

    //     // Verify job exists and belongs to employer
    //     let job = self.db_client.get_job_by_id(job_id)
    //         .await?
    //         .ok_or(ServiceError::JobNotFound(job_id))?;

    //     if job.employer_id != employer_id {
    //         return Err(ServiceError::UnauthorizedJobAccess(employer_id, job_id));
    //     }

    //     if job.status != Some(JobStatus::Open) {
    //         return Err(ServiceError::InvalidJobStatus(job_id, job.status.unwrap()));
    //     }

    //     // Verify worker exists and is available
    //     let worker_profile = self.db_client.get_worker_profile(worker_id)
    //         .await?;

    //     if !worker_profile.is_available.unwrap_or(false) {
    //         return Err(ServiceError::Validation("Worker is not available".to_string()));
    //     }

    //     // Update job with assigned worker
    //     let updated_job = self.db_client.assign_worker_to_job(job_id, worker_id).await?;

    //     // Create job contract
    //     let contract = self.db_client.create_job_contract(
    //         job_id,
    //         employer_id,
    //         worker_id,
    //         job.budget.to_f64().unwrap_or(0.0),
    //         job.estimated_duration_days,
    //         "Standard work agreement terms".to_string(),
    //     ).await?;

    //     // Update escrow with worker info
    //     let escrow_update = self.escrow_service.assign_worker_to_escrow(job_id, worker_id).await?;

    //     // Audit log
    //     self.audit_service.log_job_assignment(
    //         employer_id,
    //         worker_id,
    //         &updated_job,
    //         &contract,
    //     ).await?;

    //     tx.commit().await?;

    //     // Notify worker
    //     self.notification_service.notify_job_assignment(worker_id, &updated_job).await?;

    //     Ok(JobAssignmentResult {
    //         job: updated_job,
    //         contract,
    //         escrow: escrow_update,
    //     })
    // }

    pub async fn submit_job_progress(
        &self,
        job_id: Uuid,
        worker_id: Uuid,
        progress_data: SubmitProgressDto,
    ) -> Result<ProgressSubmissionResult, ServiceError> {
        let mut tx = self.db_client.pool.begin().await?;

        // Verify job and worker assignment
        let job = self.db_client.get_job_by_id(job_id)
            .await?
            .ok_or(ServiceError::JobNotFound(job_id))?;

        let worker_profile = self.db_client.get_worker_profile(worker_id).await?;

        if job.assigned_worker_id != Some(worker_profile.id) {
            return Err(ServiceError::UnauthorizedJobAccess(worker_id, job_id));
        }

        // Submit progress
        let progress = self.db_client.submit_job_progress(
            job_id,
            worker_id,
            progress_data.progress_percentage,
            progress_data.description,
            progress_data.image_urls,
        ).await?;

        // Handle partial payments if applicable
        let payment_release = if progress_data.progress_percentage >= 100 {
            // Job completed, release full payment
            Some(self.escrow_service.complete_escrow(job_id).await?)
        } else if let Some(partial_percentage) = job.partial_payment_percentage {
            // Check if milestone reached for partial payment
            if progress_data.progress_percentage >= partial_percentage as i32 {
                Some(self.escrow_service.release_partial_payment(
                    job_id,
                    (partial_percentage as f64) / 100.0,
                ).await?)
            } else {
                None
            }
        } else {
            None
        };

        // Update job status if completed
        let updated_job = if progress_data.progress_percentage >= 100 {
            self.db_client.update_job_status(job_id, JobStatus::UnderReview).await?
        } else {
            job.clone()
        };

        // Audit log
        self.audit_service.log_progress_submission(
            worker_id,
            &progress,
            payment_release.as_ref(),
        ).await?;

        tx.commit().await?;

        // Notify employer
        self.notification_service.notify_progress_update(
            job.employer_id,
            &progress,
        ).await?;

        Ok(ProgressSubmissionResult {
            progress,
            job: updated_job,
            payment_release,
        })
    }

    pub async fn complete_job(
        &self,
        job_id: Uuid,
        employer_id: Uuid,
    ) -> Result<JobCompletionResult, ServiceError> {
        let mut tx = self.db_client.pool.begin().await?;

        // Verify job ownership and status
        let job = self.db_client.get_job_by_id(job_id)
            .await?
            .ok_or(ServiceError::JobNotFound(job_id))?;

        if job.employer_id != employer_id {
            return Err(ServiceError::UnauthorizedJobAccess(employer_id, job_id));
        }

        if job.status != Some(JobStatus::UnderReview) {
            return Err(ServiceError::InvalidJobStatus(job_id, job.status.unwrap()));
        }

        // Update job status to completed
        let completed_job = self.db_client.update_job_status(job_id, JobStatus::Completed).await?;

        // Release final payment if not already done
        let final_payment = self.escrow_service.complete_escrow(job_id).await?;

        // Award trust points
        if let Some(worker_id) = job.assigned_worker_id {
            let worker_profile = self.db_client.get_worker_profile(worker_id).await?;
            
            self.trust_service.award_job_completion_points(
                worker_profile.user_id,
                employer_id,
                job_id,
                5, // Default rating for completion
                true, // Assume completed on time
            ).await?;
        }

        // Update worker stats
        if let Some(worker_id) = job.assigned_worker_id {
            let _ = self.db_client.update_worker_rating(worker_id).await;
        }

        // Audit log
        self.audit_service.log_job_completion(
            employer_id,
            &completed_job,
            &final_payment,
        ).await?;

        tx.commit().await?;

        // Notify worker
        if let Some(worker_id) = job.assigned_worker_id {
            self.notification_service.notify_job_completion(worker_id, &completed_job).await?;
        }

        Ok(JobCompletionResult {
            job: completed_job,
            payment: final_payment,
        })
    }
}

// Result types for service methods
#[derive(Debug, Serialize)]
pub struct JobAssignmentResult {
    pub job: Job,
    pub contract: JobContract,
    pub escrow: EscrowTransaction,
}

#[derive(Debug, Serialize)]
pub struct ProgressSubmissionResult {
    pub progress: JobProgress,
    pub job: Job,
    pub payment_release: Option<EscrowTransaction>,
}

#[derive(Debug, Serialize)]
pub struct JobCompletionResult {
    pub job: Job,
    pub payment: EscrowTransaction,
}