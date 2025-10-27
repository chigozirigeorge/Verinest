// services/escrow_service.rs
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    service::{error::ServiceError, dispute_service},
    models::labourmodel::*,
    DBClient,
    db::labourdb::LaborExt,
};

// Option 1: Remove fields from enum variants for SQLx compatibility
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "escrow_state", rename_all = "snake_case")]
pub enum EscrowState {
    Created,
    Funded,
    PartialRelease,  // Remove the percentage field for SQLx compatibility
    Completed,
    Disputed,
    Refunded,
    Cancelled,
}

// Option 2: Store percentage in metadata instead of enum variant
#[derive(Debug, Clone)]
pub struct EscrowTransition {
    pub from: EscrowState,
    pub to: EscrowState,
    pub action: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct EscrowStateMachine {
    pub escrow_id: Uuid,
    pub current_state: EscrowState,
    pub transitions: Vec<EscrowTransition>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub release_percentage: Option<f64>, // Store percentage separately
}

impl EscrowStateMachine {
    pub fn new(escrow_id: Uuid) -> Self {
        Self {
            escrow_id,
            current_state: EscrowState::Created,
            transitions: Vec::new(),
            metadata: HashMap::new(),
            release_percentage: None,
        }
    }

    pub fn transition(&mut self, to: EscrowState, action: String, metadata: Option<serde_json::Value>) -> Result<EscrowTransition, ServiceError> {
        if !self.is_valid_transition(&to) {
            return Err(ServiceError::InvalidEscrowTransition(
                format!("Cannot transition from {:?} to {:?}", self.current_state, to)
            ));
        }

        // Handle percentage for partial releases
        if let EscrowState::PartialRelease = to {
            if let Some(meta) = &metadata {
                if let Some(percentage) = meta.get("release_percentage").and_then(|v| v.as_f64()) {
                    self.release_percentage = Some(percentage);
                }
            }
        }

        let transition = EscrowTransition {
            from: self.current_state.clone(),
            to: to.clone(),
            action,
            timestamp: Utc::now(),
            metadata,
        };

        self.transitions.push(transition.clone());
        self.current_state = to;

        Ok(transition)
    }

    fn is_valid_transition(&self, to: &EscrowState) -> bool {
        match (&self.current_state, to) {
            (EscrowState::Created, EscrowState::Funded) => true,
            (EscrowState::Created, EscrowState::Cancelled) => true,
            (EscrowState::Funded, EscrowState::PartialRelease) => true,
            (EscrowState::Funded, EscrowState::Completed) => true,
            (EscrowState::Funded, EscrowState::Disputed) => true,
            (EscrowState::PartialRelease, EscrowState::Completed) => true,
            (EscrowState::PartialRelease, EscrowState::Disputed) => true,
            (EscrowState::Disputed, EscrowState::Refunded) => true,
            (EscrowState::Disputed, EscrowState::Completed) => true,
            _ => false,
        }
    }

    pub fn get_release_percentage(&self) -> Option<f64> {
        self.release_percentage
    }
}

#[derive(Debug, Clone)]
pub struct EscrowService {
    db_client: Arc<DBClient>,
    state_machines: Arc<tokio::sync::RwLock<HashMap<Uuid, EscrowStateMachine>>>,
}

impl EscrowService {
    pub fn new(db_client: Arc<DBClient>) -> Self {
        Self {
            db_client,
            state_machines: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_escrow(
        &self,
        job_id: Uuid,
        employer_id: Uuid,
        amount: f64,
        platform_fee: f64,
        partial_payment_allowed: bool,
        partial_payment_percentage: Option<i32>,
    ) -> Result<EscrowTransaction, ServiceError> {
        let tx = self.db_client.pool.begin().await?;

        // Create escrow transaction
        let escrow = self.db_client.create_escrow_transaction(
            job_id,
            employer_id,
            Uuid::nil(), // Worker will be assigned later
            amount,
            platform_fee,
        ).await?;

        // Initialize state machine
        let mut state_machine = EscrowStateMachine::new(escrow.id);
        state_machine.metadata.insert("amount".to_string(), serde_json::json!(amount));
        state_machine.metadata.insert("platform_fee".to_string(), serde_json::json!(platform_fee));
        state_machine.metadata.insert("partial_payment_allowed".to_string(), serde_json::json!(partial_payment_allowed));
        
        if let Some(percentage) = partial_payment_percentage {
            state_machine.metadata.insert("partial_payment_percentage".to_string(), serde_json::json!(percentage));
        }

        // Store state machine
        let mut machines = self.state_machines.write().await;
        machines.insert(escrow.id, state_machine);

        tx.commit().await?;
        Ok(escrow)
    }

    pub async fn assign_worker_to_escrow(
        &self,
        job_id: Uuid,
        _worker_id: Uuid,
    ) -> Result<EscrowTransaction, ServiceError> {
        // In a real implementation, this would update the escrow with worker info
        // For now, we'll just return the existing escrow
        let escrow = self.db_client
            .get_escrow_by_job_id(job_id)
            .await?
            .ok_or(ServiceError::Validation("Escrow not found for job".to_string()))?;

        Ok(escrow)
    }

    pub async fn release_partial_payment(
        &self,
        job_id: Uuid,
        release_percentage: f64,
    ) -> Result<EscrowTransaction, ServiceError> {
        let tx = self.db_client.pool.begin().await?;

        let escrow = self.db_client
            .get_escrow_by_job_id(job_id)
            .await?
            .ok_or(ServiceError::Validation("Escrow not found".to_string()))?;

        // Update state machine
        let mut machines = self.state_machines.write().await;
        if let Some(state_machine) = machines.get_mut(&escrow.id) {
            state_machine.transition(
                EscrowState::PartialRelease,
                "partial_release".to_string(),
                Some(serde_json::json!({ "release_percentage": release_percentage })),
            )?;
        }

        // Update escrow status
        let updated_escrow = self.db_client.update_escrow_status(
            escrow.id,
            PaymentStatus::PartiallyPaid,
            None, // No transaction hash for partial releases
        ).await?;

        // In a real implementation, this would trigger actual payment
        // For web3 integration, this would call a smart contract

        tx.commit().await?;
        Ok(updated_escrow)
    }

    pub async fn complete_escrow(
        &self,
        job_id: Uuid,
    ) -> Result<EscrowTransaction, ServiceError> {
        let tx: sqlx::Transaction<'static, sqlx::Postgres> = self.db_client.pool.begin().await?;

        let escrow = self.db_client
            .get_escrow_by_job_id(job_id)
            .await?
            .ok_or(ServiceError::Validation("Escrow not found".to_string()))?;

        // Update state machine
        let mut machines = self.state_machines.write().await;
        if let Some(state_machine) = machines.get_mut(&escrow.id) {
            state_machine.transition(
                EscrowState::Completed,
                "completion".to_string(),
                Some(serde_json::json!({ "completed_at": Utc::now() })),
            )?;
        }

        // Update escrow status to completed
        let completed_escrow = self.db_client.update_escrow_status(
            escrow.id,
            PaymentStatus::Completed,
            Some("manual_completion".to_string()), // In web3, this would be transaction hash
        ).await?;

        tx.commit().await?;
        Ok(completed_escrow)
    }

    pub async fn handle_dispute(
        &self,
        escrow_id: Uuid,
        dispute_id: Uuid,
    ) -> Result<EscrowTransaction, ServiceError> {
        let tx = self.db_client.pool.begin().await?;

        let mut machines = self.state_machines.write().await;
        if let Some(state_machine) = machines.get_mut(&escrow_id) {
            state_machine.transition(
                EscrowState::Disputed,
                "dispute_raised".to_string(),
                Some(serde_json::json!({ "dispute_id": dispute_id })),
            )?;
        }

        let disputed_escrow = self.db_client.update_escrow_status(
            escrow_id,
            PaymentStatus::Pending, // Freeze payments during dispute
            None,
        ).await?;

        tx.commit().await?;
        Ok(disputed_escrow)
    }

    pub async fn resolve_dispute(
        &self,
        escrow_id: Uuid,
        resolution: DisputeResolution,
    ) -> Result<EscrowTransaction, ServiceError> {
        let tx = self.db_client.pool.begin().await?;

        let mut machines = self.state_machines.write().await;
        if let Some(state_machine) = machines.get_mut(&escrow_id) {
            match resolution {
                DisputeResolution::FavorEmployer => {
                    state_machine.transition(
                        EscrowState::Refunded,
                        "dispute_resolved_refund".to_string(),
                        Some(serde_json::json!({ "resolution": "favor_employer" })),
                    )?;
                    
                    // Refund employer
                    // This would trigger actual refund logic
                }
                DisputeResolution::FavorWorker { payment_percentage } => {
                    state_machine.transition(
                        EscrowState::Completed,
                        "dispute_resolved_payment".to_string(),
                        Some(serde_json::json!({ 
                            "resolution": "favor_worker",
                            "payment_percentage": payment_percentage 
                        })),
                    )?;
                    
                    // Pay worker (partial or full)
                    if payment_percentage < 100.0 {
                        // Store the percentage for partial payment
                        state_machine.release_percentage = Some(payment_percentage);
                        self.release_partial_payment_by_escrow_id(escrow_id, payment_percentage).await?;
                    } else {
                        self.complete_escrow_by_id(escrow_id).await?;
                    }
                }
            }
        }

        let resolved_escrow = self.db_client.update_escrow_status(
            escrow_id,
            PaymentStatus::Completed,
            Some("dispute_resolution".to_string()),
        ).await?;

        tx.commit().await?;
        Ok(resolved_escrow)
    }

    async fn release_partial_payment_by_escrow_id(
        &self,
        escrow_id: Uuid,
        release_percentage: f64,
    ) -> Result<EscrowTransaction, ServiceError> {
        // Implementation for partial payment by escrow ID
        let escrow = self.db_client.get_escrow_by_id(escrow_id).await?
            .ok_or(ServiceError::Validation("Escrow not found".to_string()))?;

        self.release_partial_payment(escrow.job_id, release_percentage).await
    }

    async fn complete_escrow_by_id(
        &self,
        escrow_id: Uuid,
    ) -> Result<EscrowTransaction, ServiceError> {
        // Implementation for completion by escrow ID
        let escrow = self.db_client.get_escrow_by_id(escrow_id).await?
            .ok_or(ServiceError::Validation("Escrow not found".to_string()))?;

        self.complete_escrow(escrow.job_id).await
    }

    // Helper method to get current release percentage
    pub async fn get_current_release_percentage(&self, escrow_id: Uuid) -> Option<f64> {
        let machines = self.state_machines.read().await;
        machines.get(&escrow_id).and_then(|sm| sm.get_release_percentage())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DisputeResolution {
    FavorEmployer,
    FavorWorker { payment_percentage: f64 },
}

impl From<dispute_service::DisputeResolution> for DisputeResolution {
    fn from(resolution: dispute_service::DisputeResolution) -> Self {
        match resolution {
            dispute_service::DisputeResolution::FavorEmployer => Self::FavorEmployer,
            dispute_service::DisputeResolution::FavorWorker { payment_percentage } => {
                Self::FavorWorker { payment_percentage }
            }
        }
    }
}