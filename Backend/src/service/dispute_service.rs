use redis::aio::ConnectionManager;
// services/dispute_service.rs
use uuid::Uuid;
use serde::Serialize;
use std::sync::Arc;
use num_traits::ToPrimitive;
use chrono::Utc;
use sqlx::Row;

use crate::{
    db::{
        db::DBClient,
        labourdb::LaborExt,
        userdb::UserExt,
    }, 
    models::{
        labourmodel::*, 
        usermodel::User,
    }, 
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

        // Time-based security: Check if 1 hours have passed since dispute assignment
        if let Some(created_at) = dispute.created_at {
            let now = Utc::now();
            let time_diff = now.signed_duration_since(created_at);
            
            if time_diff.num_hours() < 1 {
                return Err(ServiceError::Validation(
                    format!("Dispute cannot be resolved until 1 hours after creation. {} hours remaining", 
                            1 - time_diff.num_hours())
                ));
            }
        }

        // Check if this is a high-value dispute (>= 100,000)
        let escrow = self.db_client.get_escrow_by_job_id(dispute.job_id).await?
            .ok_or(ServiceError::Validation("Escrow not found for job".to_string()))?;

        if escrow.amount.to_f64().unwrap_or(0.0) >= 100_000.0 {
            // High-value dispute requires multi-signature verification
            self.handle_high_value_dispute_resolution(dispute_id, verifier_id, &resolution, &decision, payment_percentage, &mut tx).await?;
        } else {
            // Regular dispute resolution
            self.finalize_dispute_resolution(dispute_id, verifier_id, &resolution, &decision, payment_percentage, &mut tx).await?;
        }

        tx.commit().await?;

        // Notify both parties
        self.notification_service.notify_dispute_resolution(
            dispute.raised_by,
            dispute.against,
            &dispute,
            &decision,
        ).await?;

        // Return updated result
        let resolved_dispute = self.db_client.get_dispute_by_id(dispute_id).await?
            .ok_or(ServiceError::DisputeNotFound(dispute_id))?;
        let updated_job = self.db_client.get_job_by_id(dispute.job_id).await?
            .ok_or(ServiceError::JobNotFound(dispute.job_id))?;
        let resolved_escrow = self.db_client.get_escrow_by_job_id(dispute.job_id).await?
            .ok_or(ServiceError::Validation("Escrow not found for job".to_string()))?;

        Ok(DisputeResolutionResult {
            dispute: resolved_dispute,
            job: updated_job,
            escrow: resolved_escrow,
        })
    }

    async fn handle_high_value_dispute_resolution(
        &self,
        dispute_id: Uuid,
        verifier_id: Uuid,
        resolution: &str,
        decision: &str,
        payment_percentage: Option<f64>,
        tx: &mut sqlx::Transaction<'static, sqlx::Postgres>,
    ) -> Result<(), ServiceError> {
        // Check if admin has already verified this dispute
        let admin_verification = self.db_client.get_admin_verification_for_dispute(dispute_id).await
            .map_err(|e| ServiceError::Database(e))?;

        if admin_verification.is_none() {
            // First stage: Verifier resolves, pending admin verification
            self.create_pending_resolution(dispute_id, verifier_id, resolution, decision, payment_percentage).await?;
            
            // Assign to admin for verification
            let admin = self.db_client.get_admin_user().await
                .map_err(|e| ServiceError::Database(e))?
                .ok_or_else(|| ServiceError::Validation("No admin user found for high-value dispute verification".to_string()))?;
            
            self.db_client.assign_admin_to_dispute_verification(dispute_id, admin.id).await
                .map_err(|e| ServiceError::Database(e))?;
            
            // Update dispute status to "pending_admin_verification"
            self.db_client.update_dispute_status(dispute_id, DisputeStatus::Escalated).await
                .map_err(|e| ServiceError::Database(e))?;
            
            return Err(ServiceError::Validation(
                "High-value dispute resolution requires admin verification. Admin has been notified.".to_string()
            ));
        } else {
            // Admin has verified, proceed with final resolution
            self.finalize_dispute_resolution(dispute_id, verifier_id, resolution, decision, payment_percentage, tx).await?;
            Ok(())
        }
    }

    async fn create_pending_resolution(
        &self,
        dispute_id: Uuid,
        verifier_id: Uuid,
        resolution: &str,
        decision: &str,
        payment_percentage: Option<f64>,
    ) -> Result<(), ServiceError> {
        // Store the pending resolution for admin review
        self.db_client.create_pending_dispute_resolution(
            dispute_id,
            verifier_id,
            resolution.to_string(),
            decision.to_string(),
            payment_percentage,
        ).await.map_err(|e| ServiceError::Database(e))?;
        
        Ok(())
    }

    async fn finalize_dispute_resolution(
        &self,
        dispute_id: Uuid,
        verifier_id: Uuid,
        resolution: &str,
        decision: &str,
        payment_percentage: Option<f64>,
        tx: &mut sqlx::Transaction<'static, sqlx::Postgres>,
    ) -> Result<(), ServiceError> {
        // Resolve dispute
        let _resolved_dispute = self.db_client.resolve_dispute(
            dispute_id,
            resolution.to_string(),
            decision.to_string(),
        ).await?;

        // Handle escrow based on decision
        let escrow = self.db_client.get_escrow_by_job_id(dispute_id).await?
            .ok_or(ServiceError::Validation("Escrow not found for job".to_string()))?;

        let dispute_resolution = match decision {
            "favor_employer" => DisputeResolution::FavorEmployer,
            "favor_worker" => DisputeResolution::FavorWorker { 
                payment_percentage: payment_percentage.unwrap_or(100.0) 
            },
            "partial_payment" => DisputeResolution::FavorWorker { 
                payment_percentage: payment_percentage.unwrap_or(50.0) 
            },
            _ => return Err(ServiceError::Validation("Invalid decision".to_string())),
        };

        let _resolved_escrow = self.escrow_service.resolve_dispute(escrow.id, dispute_resolution.to_escrow_resolution()).await?;

        // Update job status based on resolution
        let job_status = match decision {
            "favor_employer" => JobStatus::Cancelled,
            "favor_worker" | "partial_payment" => JobStatus::Completed,
            _ => JobStatus::Cancelled,
        };

        let _updated_job = self.db_client.update_job_status(dispute_id, job_status).await?;

        // Award trust points based on resolution
        self.handle_trust_points_after_dispute(&self.db_client.get_dispute_by_id(dispute_id).await?.unwrap(), &decision).await?;

        // Audit log
        self.audit_service.log_dispute_resolution(
            verifier_id,
            &self.db_client.get_dispute_by_id(dispute_id).await?.unwrap(),
            &decision,
        ).await?;

        Ok(())
    }

    async fn assign_to_verifier(&self, dispute_id: Uuid) -> Result<Dispute, ServiceError> {
        // Get available verifiers (not currently assigned to active disputes)
        let verifiers = self.db_client.get_available_verifiers().await
            .map_err(|e| ServiceError::Database(e))?;
        
        if verifiers.is_empty() {
            // If no verifiers available, assign to admin as fallback
            let admin = self.db_client.get_admin_user().await
                .map_err(|e| ServiceError::Database(e))?
                .ok_or_else(|| ServiceError::Validation("No admin user found".to_string()))?;
            
            self.db_client.assign_verifier_to_dispute(dispute_id, admin.id).await
                .map_err(|e| ServiceError::Database(e))?;
        } else {
            // Implement true round-robin with workload balancing
            let verifier_workloads = self.get_verifier_workloads(&verifiers).await?;
            
            // Sort by current dispute count (ascending) for load balancing
            let mut verifier_pairs: Vec<(Uuid, usize)> = verifiers.iter()
                .map(|v| (v.id, *verifier_workloads.get(&v.id).unwrap_or(&0)))
                .collect();
            verifier_pairs.sort_by_key(|(_, workload)| *workload);
            
            // Get the least loaded verifier
            let selected_verifier_id = verifier_pairs[0].0;
            
            // Update round-robin counter in Redis/DB
            self.update_verifier_assignment_counter(selected_verifier_id).await?;
            
            // Assign verifier to dispute
            self.db_client.assign_verifier_to_dispute(dispute_id, selected_verifier_id).await
                .map_err(|e| ServiceError::Database(e))?;
        }
        
        // Return the updated dispute
        self.db_client.get_dispute_by_id(dispute_id).await
            .map_err(|e| ServiceError::Database(e))?
            .ok_or_else(|| ServiceError::DisputeNotFound(dispute_id))
    }

    async fn get_verifier_workloads(&self, verifiers: &[User]) -> Result<std::collections::HashMap<Uuid, usize>, ServiceError> {
        let mut workloads = std::collections::HashMap::new();
        
        for verifier in verifiers {
            let active_disputes = self.db_client.get_pending_verifications_f(verifier.id).await
                .map_err(|e| ServiceError::Database(e))?;
            workloads.insert(verifier.id, active_disputes.len());
        }
        
        Ok(workloads)
    }

    async fn update_verifier_assignment_counter(&self, verifier_id: Uuid) -> Result<(), ServiceError> {
        // Store assignment count in Redis with atomic INCR operation
        let redis_key = format!("verifier_assignment_count:{}", verifier_id);
        
        // Try Redis first for performance and atomicity
        if let Some(redis_client) = self.get_redis_client() {
            match self.increment_redis_counter(redis_client, &redis_key).await {
                Ok(_) => {
                    tracing::info!("Verifier {} assignment counter updated in Redis", verifier_id);
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Redis operation failed for verifier counter: {:?}. Falling back to database.", e);
                }
            }
        }
        
        // Fallback to database if Redis is unavailable
        self.update_verifier_assignment_counter_db(verifier_id).await?;
        
        tracing::info!("Verifier {} assignment counter updated in database", verifier_id);
        Ok(())
    }

    fn get_redis_client(&self) -> Option<Arc<redis::aio::ConnectionManager>> {
        // Try to get Redis client from app state or environment
        // This assumes you have Redis available in your application
        // You may need to adjust this based on your actual Redis setup
        None // Placeholder - implement based on your Redis configuration
    }

    async fn increment_redis_counter(
        &self, 
        redis_client: Arc<redis::aio::ConnectionManager>, 
        key: &str
    ) -> Result<(), ServiceError> {
        use redis::AsyncCommands;
        
        // Get connection from ConnectionManager
        let mut conn = ConnectionManager::clone(&redis_client);
        
        // Atomic increment with expiration (30 days)
        let _: i64 = redis::cmd("INCR")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| ServiceError::Validation(format!("Redis INCR failed: {:?}", e)))?;
        
        // Set expiration to prevent memory leaks
        let _: () = redis::cmd("EXPIRE")
            .arg(key)
            .arg(30 * 24 * 60 * 60) // 30 days
            .query_async(&mut conn)
            .await
            .map_err(|e| ServiceError::Validation(format!("Redis EXPIRE failed: {:?}", e)))?;
        
        Ok(())
    }

    async fn update_verifier_assignment_counter_db(&self, verifier_id: Uuid) -> Result<(), ServiceError> {
        // Database fallback implementation
        let mut tx = self.db_client.pool.begin().await
            .map_err(|e| ServiceError::Database(e))?;
        
        // Create or update verifier assignment counter table
        sqlx::query(
            r#"
            INSERT INTO verifier_assignment_counters (verifier_id, assignment_count, last_assigned_at)
            VALUES ($1, 1, NOW())
            ON CONFLICT (verifier_id) DO UPDATE SET
                assignment_count = verifier_assignment_counters.assignment_count + 1,
                last_assigned_at = NOW()
            "#
        )
        .bind(verifier_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServiceError::Database(e))?;
        
        // Clean up old entries (older than 90 days) to prevent table bloat
        sqlx::query(
            r#"
            DELETE FROM verifier_assignment_counters 
            WHERE last_assigned_at < NOW() - INTERVAL '90 days'
            "#
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ServiceError::Database(e))?;
        
        tx.commit().await
            .map_err(|e| ServiceError::Database(e))?;
        
        Ok(())
    }

    async fn get_verifier_assignment_count(&self, verifier_id: Uuid) -> Result<i64, ServiceError> {
        // Try Redis first
        if let Some(redis_client) = self.get_redis_client() {
            let redis_key = format!("verifier_assignment_count:{}", verifier_id);
            
            match self.get_redis_counter(redis_client, &redis_key).await {
                Ok(count) => return Ok(count),
                Err(e) => {
                    tracing::warn!("Failed to get count from Redis: {:?}. Falling back to database.", e);
                }
            }
        }
        
        // Fallback to database
        self.get_verifier_assignment_count_db(verifier_id).await
    }

    async fn get_redis_counter(
        &self, 
        redis_client: Arc<redis::aio::ConnectionManager>, 
        key: &str
    ) -> Result<i64, ServiceError> {
        // Get connection from ConnectionManager
        let mut conn = ConnectionManager::clone(&redis_client);
        
        let count: Option<i64> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| ServiceError::Validation(format!("Redis GET failed: {:?}", e)))?;
        
        Ok(count.unwrap_or(0))
    }

    async fn get_verifier_assignment_count_db(&self, verifier_id: Uuid) -> Result<i64, ServiceError> {
        let result = sqlx::query(
            r#"
            SELECT assignment_count 
            FROM verifier_assignment_counters 
            WHERE verifier_id = $1
            "#
        )
        .bind(verifier_id)
        .fetch_one(&self.db_client.pool)
        .await
        .map_err(|e| ServiceError::Database(e))?;
        
        let count: i64 = result.try_get("assignment_count")
            .unwrap_or(0);
        
        Ok(count)
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
