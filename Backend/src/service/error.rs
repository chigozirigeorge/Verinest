use thiserror::Error;
use uuid::Uuid;
use crate::{
    models::labourmodel::*,
    error::HttpError,
};

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Worker profile not found for user {0}")]
    WorkerProfileNotFound(Uuid),
    
    #[error("Job {0} not found")]
    JobNotFound(Uuid),
    
    #[error("Job {0} is not in status {1:?}")]
    InvalidJobStatus(Uuid, JobStatus),
    
    #[error("User {0} is not authorized to perform this action on job {1}")]
    UnauthorizedJobAccess(Uuid, Uuid),
    
    #[error("Insufficient funds for escrow: required {required}, available {available}")]
    InsufficientEscrowFunds { required: f64, available: f64 },
    
    #[error("Invalid escrow state transition: {0}")]
    InvalidEscrowTransition(String),
    
    #[error("Dispute {0} not found")]
    DisputeNotFound(Uuid),
    
    #[error("Dispute {0} is not in status {1:?}")]
    InvalidDisputeStatus(Uuid, DisputeStatus),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Notification error: {0}")]
    Notification(String),
}

impl From<ServiceError> for HttpError {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::WorkerProfileNotFound(_) 
            | ServiceError::JobNotFound(_) 
            | ServiceError::DisputeNotFound(_) => HttpError::not_found(error.to_string()),
            
            ServiceError::InvalidJobStatus(_, _)
            | ServiceError::InvalidEscrowTransition(_)
            | ServiceError::InvalidDisputeStatus(_, _)
            | ServiceError::Validation(_) => HttpError::bad_request(error.to_string()),
            
            ServiceError::UnauthorizedJobAccess(_, _) => HttpError::unauthorized(error.to_string()),
            
            ServiceError::InsufficientEscrowFunds { .. } => HttpError::payment_required(error.to_string()),
            
            _ => HttpError::server_error(error.to_string()),
        }
    }
}
