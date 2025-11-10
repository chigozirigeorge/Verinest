use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::models::labourmodel::*;

//Worker Profile DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateWorkerProfileDto {
    
    pub category: WorkerCategory,

    #[validate(range(min = 0, max = 50, message = "Experience must be between 0 and 50 years"))]
    pub experience_years: i32,

    #[validate(length(min = 10, max = 1000, message = "Description must be between 10 and 1000 characters"))]
    pub description: String,

    #[validate(range(min = 0.0, message = "Hourly rate must be positive"))]
    pub hourly_rate: Option<f64>,

    #[validate(range(min = 0.0, message = "Daily rate must be positive"))]
    pub daily_rate: Option<f64>,

    #[validate(length(min = 1, message = "State is required"))]
    pub location_state: String,

    #[validate(length(min = 1, message = "City is required"))]
    pub location_city: String
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct WorkerProfileResponseDto {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub user_email: String,
    pub category: WorkerCategory,
    pub experience_years: i32,
    pub description: String,
    pub hourly_rate: Option<f64>,
    pub daily_rate: Option<f64>,
    pub location_state: String,
    pub location_city: String,
    pub is_available: bool,
    pub rating: f32,
    pub completed_jobs: i32,
    pub portfolio: Vec<PortfolioItemDto>,
    pub reviews: Vec<ReviewDto>,
    pub created_at: DateTime<Utc>
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreatePortfolioDto {
    #[validate(length(min = 1, max = 100, message = " Title must be between 1 and 100 characters"))]
    pub title: String,

    #[validate(length(min = 10, max = 500, message = "Description must be between 10 and 500 characters"))]
    pub description: String,

    #[validate(url(message = "Invalid image URL"))]
    pub image_url: String,

    pub project_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortfolioItemDto {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub image_url: String,
    pub project_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

//Job Dto
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateJobDto {
    pub category: WorkerCategory,

    #[validate(length(min = 1, max = 100, message = "Title must be between 1 and 100"))]
    pub title: String,

    #[validate(length(min = 20, max = 2000, message = "Description must be between 20 and 2000"))]
    pub description: String,

    #[validate(length(min = 1, message = "State is required"))]
    pub location_state: String,

    #[validate(length(min = 1, message = "City is required"))]
    pub location_city: String,

    #[validate(length(min = 1, message = "Address is required"))]
    pub location_address: String,

    #[validate(range(min = 1.0, message = "Budget must be positive"))]
    pub budget: f64,

    #[validate(range(min = 1, max = 365, message = "Duration must be between 1 and 365 days"))]
    pub estimated_duration_days: i32,

    pub partial_payment_allowed: bool,

    #[validate(range(min = 10, max = 90, message = "Partial payment percentage must be between 10% and 90%"))]
    pub partial_payment_percentage: Option<i32>,

    pub deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobResponseDto {
    pub id: Uuid,
    pub employer_id: Uuid,
    pub assigned_worker: Option<WorkerInfoDto>,
    pub category: WorkerCategory,
    pub title: String,
    pub description: String,
    pub location_state: String,
    pub location_city: String,
    pub location_address: String,
    pub budget: f64,
    pub estimated_duration_days: i32,
    pub status: JobStatus,
    pub payment_status: PaymentStatus,
    pub escrow_amount: f64,
    pub platform_fee: f64,
    pub partial_payment_allowed: bool,
    pub partial_payment_percentage: Option<i32>,
    pub application_count: i32,
    pub created_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmployerInfoDto {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub trust_score: i32,
    pub verified: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkerInfoDto {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub category: WorkerCategory,
    pub rating: f32,
    pub completed_jobs: i32,
    pub trust_score: i32,
}

//Job Application Dtos
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateJobApplicationDto {
    #[validate(range(min = 1.0, message = "Proposed rate must be positive"))]
    pub proposed_rate: f64,

    #[validate(range(min = 1, max = 365, message = "Estimated completion must be between 1 and 365 days"))]
    pub estimated_completion: i32,

    #[validate(length(min = 20, max = 2500, message = "Cover letter must be between 20 and 2500 characters"))]
    pub cover_letter: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobApplicationResponse {
    pub id: Uuid,
    pub job_id: Uuid,
    pub worker_id: Uuid,
    pub worker_user_id: Option<Uuid>,  ///added here
    pub proposed_rate: f64,
    pub estimated_completion: i32,
    pub cover_letter: String,
    pub status: String,
    pub created_at: chrono::DateTime<Utc>,
    pub worker: Option<WorkerUserResponse>,
    pub worker_profile: Option<WorkerProfileApplicationResponse>,
    pub worker_portfolio: Vec<WorkerPortfolio>,
    pub worker_reviews: Vec<JobReview>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerUserResponse {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub trust_score: i32,
    pub verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobSummaryDto {
    pub id: Uuid,
    pub title: String,
    pub category: WorkerCategory,
    pub budget: f64,
    pub location_state: String,
    pub location_city: String,
}

//Contract Dto
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateContractDto {
    pub job_id: Uuid,
    pub worker_id: Uuid,
    
    #[validate(range(min = 1.0, message = "Agreed rate must be positive"))]
    pub agreed_rate: f64,

    #[validate(range(min = 1, message = "Timeline must be positive"))]
    pub agreed_timeline: i32,

    #[validate(length(min = 20, max = 2000, message = "Terms must be between 20 and 2000 characters"))]
    pub terms: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractResponseDto {
    pub id: Uuid,
    pub job: JobSummaryDto,
    pub employer: EmployerInfoDto,
    pub worker: WorkerInfoDto,
    pub agreed_rate: f64,
    pub agreed_timeline: i32,
    pub terms: String,
    pub signed_by_employer: bool,
    pub signed_by_worker: bool,
    pub contract_date: DateTime<Utc>,
}

//Progress Tracking DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct SubmitProgressDto {
    #[validate(range(min = 0, max = 100, message = "Progress percentage must be between 0 and 100"))]
    pub progress_percentage: i32,

    #[validate(length(min = 10, max = 1000, message = "Description must be between 10 and 1000 characters"))]
    pub description: String,

    #[validate(length(min = 1, message = "At least one image is required"))]
    pub image_urls: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobProgressResponseDto {
    pub id: Uuid,
    pub job_id: Uuid,
    pub worker: WorkerInfoDto,
    pub progress_percentage: i32,
    pub description: String,
    pub image_urls: Vec<String>,
    pub submitted_at: DateTime<Utc>,
}

//Dispute DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateDisputeDto {
    #[validate(length(min = 1, message = "Reason is required"))]
    pub reason: String,

    #[validate(length(min = 20, max = 2000, message = "Description must be between 20 and  2000 characters"))]
    pub description: String,

    pub evidence_urls: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisputeResponseDto {
    pub id: Uuid,
    pub job: JobSummaryDto,
    pub raised_by: UserInfoDto,
    pub against:UserInfoDto,
    pub reason: String,
    pub description: String,
    pub evidence_urls: Vec<String>,
    pub status: DisputeStatus,
    pub assigned_verifier: Option<UserInfoDto>,
    pub resolution: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserInfoDto {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub trust_score: i32,
}

//Verification Dtos
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ResolveDisputeDto {
    #[validate(length(min = 20, max = 1000, message = "Resolution must be between 20 and 1000 characters"))]
    pub resolution: String,

    #[validate(length(min = 1, message = "Desicion is required"))]
    pub decision: String,   //favor_employer", "favor_worker", "partial_payment"

    #[validate(range(min = 0.0, max = 100.0, message = "Payment percentage must be between 0 and 100"))]
    pub payment_percentage: Option<f64>,
}

//Review DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateReviewDto {
    #[validate(range(min = 1, max = 5, message = "Rating must be between 1 and 5"))]
    pub rating: i32,

    #[validate(length(min = 10, max = 1000, message = "Comment must be between 10 and 1000 characters"))]
    pub comment: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewDto {
    pub id: Uuid,
    pub reviewer: UserInfoDto,
    pub rating: i32,
    pub comment: String,
    pub created_at: DateTime<Utc>
}

//Payment DTOs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct InitiatePaymentDto {
    pub job_id: Uuid,

    #[validate(range(min = 0.0, max = 100.0, message = "payment percentage must be between 0 and 100"))]
    pub payment_percentage: f64, // For partial payments
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EscrowTransactionDto {
    pub id: Uuid,
    pub job: JobSummaryDto,
    pub employer: EmployerInfoDto,
    pub worker: WorkerInfoDto,
    pub amount: f64,
    pub platform_fee: f64,
    pub status: PaymentStatus,
    pub transaction_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
}

// Add these DTOs
#[derive(Debug, Deserialize)]
pub struct AssignWorkerDto {
    pub worker_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct AssignWorkerResponse {
    pub job: Job,
    pub escrow: EscrowTransaction,
    pub contract: JobContract,
}

//Dashboard DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerDashBoardDto {
    pub profile: WorkerProfileResponseDto,
    pub active_jobs: Vec<JobResponseDto>,
    pub job_applications: Vec<JobApplicationResponse>,
    pub completed_jobs_count: i32,
    pub total_earnings: f64,
    pub average_rating: f32,
    pub trust_points: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmployerDashBoardDto {
    pub user_info: EmployerInfoDto,
    pub active_jobs: Vec<JobResponseDto>,
    pub completed_jobs_count: i32,
    pub total_spent: f64,
    pub trust_points: i32,
}

//Search and Filter DTos
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct SearchWorkersDto {
    pub category: Option<WorkerCategory>,
    pub location_state: Option<String>,
    pub location_city: Option<String>,
    pub min_rating: Option<f32>,
    pub max_hourly_rate: Option<f64>,
    pub max_daily_rate: Option<f64>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct SearchJobsDto {
    pub category: Option<WorkerCategory>,
    pub location_state: Option<String>,
    pub location_city: Option<String>,
    pub min_budget: Option<f64>,
    pub max_budget: Option<f64>,
    pub status: Option<JobStatus>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobsResponse {
    pub status: String,
    pub message: String,
    pub data: Vec<Job>, // Direct array, no Option
}

//Response wrappers
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub status: String,
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

impl<T> ApiResponse<T> {
    pub fn success(message: &str, data: T) -> Self {
        Self { 
            status: "success".to_string(), 
            message: message.to_string(), 
            data: Some(data)
        }
    }

    pub fn error(message: &str) -> ApiResponse<()> {
        ApiResponse { 
            status: "error".to_string(), 
            message: message.to_string(), 
            data: None,
        }
    }
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: u32, limit: u32) -> Self {
        let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;
        Self { 
            status: "success".to_string(), 
            data, 
            total, 
            page, 
            limit, 
            total_pages
        }
    }
}





#[derive(Debug, Deserialize)]
pub struct UpdateAvailabilityDto {
    pub is_available: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AddPortfolioItemDto {
    pub title: String,
    pub description: String,
    pub image_url: String,
    pub project_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyForJobDto {
    pub proposed_rate: f64,
    pub estimated_completion: i32,
    pub cover_letter: String,
}


#[derive(Debug, Deserialize)]
pub struct CreateEscrowDto {
    pub worker_id: Uuid,
    pub amount: f64,
    pub platform_fee: f64,
}

#[derive(Debug, Deserialize)]
pub struct ReleasePaymentDto {
    pub escrow_id: Uuid,
    pub release_percentage: f64,
}


// #[derive(Debug, Deserialize)]
// pub struct SignContractDto {
//     pub signer_role: String,
// }

#[derive(Debug, Serialize)]
pub struct WorkerProfileResponse {
    pub profile: WorkerProfile,
    pub portfolio: Vec<WorkerPortfolio>,
    pub reviews: Vec<JobReview>,
}

#[derive(Debug, Serialize)]
pub struct WorkerDashboard {
    pub profile: WorkerProfile,
    pub portfolio: Vec<WorkerPortfolio>,
    pub reviews: Vec<JobReview>,
    pub active_jobs: Vec<Job>,
    pub pending_applications: Vec<JobApplication>,
}

#[derive(Debug, Serialize)]
pub struct EmployerDashboard {
    pub posted_jobs: Vec<Job>,
    pub active_contracts: Vec<JobContract>,
    pub pending_applications: Vec<JobApplication>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerProfileApplicationResponse {
    pub profile_id: Uuid, // Add this field
    pub category: String,
    pub experience_years: i32,
    pub description: String,
    pub hourly_rate: f64,
    pub daily_rate: f64,
    pub location_state: String,
    pub location_city: String,
    pub is_available: bool,
    pub rating: f32,
    pub completed_jobs: i32,
    pub skills: Vec<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SignContractDto {
    pub otp_code: Option<String>, // OTP for verification
    pub request_otp: Option<bool>, // Flag to request OTP
}

#[derive(Debug, Deserialize, Validate)]
pub struct RejectApplicationDto {
    #[validate(length(min = 1, max = 500, message = "Rejection reason must be between 1 and 500 characters"))]
    pub rejection_reason: String,
}