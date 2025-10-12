use std::sync::Arc;
use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    routing::{get, post, put},
    Extension, Json, Router
};
use uuid::Uuid;
use validator::Validate;
use num_traits::ToPrimitive;

use crate::{
    db::labourdb::LaborExt,
    dtos::labordtos::*,
    error::HttpError,
    middleware::JWTAuthMiddeware,
    models::{labourmodel::*, usermodel::UserRole},
    AppState
};

pub fn labor_handler() -> Router {
    Router::new()
        .route("/worker/profile", post(create_worker_profile))
        .route("/worker/profile", get(get_worker_profile))
        .route("/worker/profile/availability", put(update_worker_availability))
        .route("/worker/portfolio", post(add_portfolio_item))
        .route("/worker/portfolio", get(get_worker_portfolio))
        
        // Job management routes
        .route("/jobs", post(create_job))
        .route("/jobs", get(search_jobs))
        .route("/jobs/:job_id", get(get_job_details))
        .route("/jobs/:job_id/applications", post(apply_to_job))
        .route("/jobs/:job_id/applications", get(get_job_applications))
        .route("/jobs/:job_id/assign", put(assign_worker_to_job))
        .route("/jobs/:job_id/contract", post(create_job_contract))
        .route("/jobs/:job_id/progress", post(submit_job_progress))
        .route("/jobs/:job_id/progress", get(get_job_progress))
        .route("/jobs/:job_id/complete", put(complete_job))
        .route("/jobs/:job_id/review", post(create_job_review))
        
        // Payment and escrow routes
        .route("/jobs/:job_id/escrow", post(create_escrow))
        .route("/jobs/:job_id/payment", put(release_payment))
        
        // Dispute management routes
        .route("/jobs/:job_id/dispute", post(create_dispute))
        .route("/disputes/:dispute_id/resolve", put(resolve_dispute))
        .route("/disputes/pending", get(get_pending_verifications))
        
        // Search and discovery routes
        .route("/workers/search", get(search_workers))
        .route("/workers/:worker_id", get(get_worker_details))
        
        // Dashboard routes
        .route("/worker/dashboard", get(get_worker_dashboard))
        .route("/employer/dashboard", get(get_employer_dashboard))
        
        // Contract management
        .route("/contracts/:contract_id/sign", put(sign_contract))
}

pub async fn create_worker_profile(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateWorkerProfileDto>
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Check if profile already exists
    let existing_profile = app_state
        .db_client
        .get_worker_profile(auth.user.id)
        .await;

    if let Ok(_) = existing_profile {
        return Err(HttpError::bad_request("Worker profile already exists"));
    }

    let worker_profile = app_state
        .db_client
        .create_worker_profile(
            auth.user.id,
            body.category,
            body.experience_years,
            body.description,
            body.hourly_rate,
            body.daily_rate,
            body.location_state,
            body.location_city,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Worker profile created successfully",
        worker_profile
    )))
}

pub async fn create_job(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateJobDto>
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let job = app_state.db_client
        .create_job(
            auth.user.id, 
            body.category, 
            body.title, 
            body.description, 
            body.location_state, 
            body.location_city, 
            body.location_address, 
            body.budget, 
            body.estimated_duration_days, 
            body.partial_payment_allowed, 
            body.partial_payment_percentage, 
            body.deadline
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Job created Succesfully",
        job
    )))
}

pub async fn apply_to_job(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(job_id): Path<Uuid>,
    Json(body): Json<CreateJobApplicationDto>
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    //verify job exists and is open
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::unauthorized("Job not found"))?;

   if job.status != Some(JobStatus::Open) {
        return Err(HttpError::bad_request("Job is not open for applications"));
    }

    //check if worker has a profile and is in the same state
    let worker_profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;


    if worker_profile.location_state != job.location_state {
        return Err(HttpError::bad_request(
            "Worker must be in the same state as the Job"
        ));
    }

    if worker_profile.category != job.category {
        return Err(HttpError::bad_request("Worker category does not match job category"));
    }

    let application = app_state.db_client
        .create_job_application(job_id, 
            worker_profile.id, 
            body.proposed_rate, 
            body.estimated_completion, 
        body.cover_letter
    )
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Application submitted successfully",
        application,
    )))
}

pub async fn search_workers(
    Query(params): Query<SearchWorkersDto>,
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpError> {
    params.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let offset = ((page - 1) * limit) as i64;

   let workers = if let (Some(state), Some(category)) = (&params.location_state, params.category) {
    app_state.db_client
        .get_workers_by_location_and_category(
            state, 
            category, 
            params.limit.unwrap_or(10) as i64, 
            offset
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
} else {
    // Return empty vector instead of Json
    vec![]
};

    // convert to response objects (profile + portfolio + reviews)
    let worker_responses: Vec<crate::dtos::labordtos::WorkerProfileResponse> = workers
        .into_iter()
        .map(|worker| {
            // Convert BigDecimal to f64 for the response using ToPrimitive
            let hourly_rate = worker.hourly_rate.as_ref().and_then(|bd| bd.to_f64());
            let daily_rate = worker.daily_rate.as_ref().and_then(|bd| bd.to_f64());

            // Build a WorkerProfile with the original worker but keep Option fields as-is
            let profile = worker;

            // For now, portfolio and reviews are empty; callers can fetch full profile if needed
            crate::dtos::labordtos::WorkerProfileResponse {
                profile,
                portfolio: vec![],
                reviews: vec![],
            }
        })
        .collect();

    Ok(Json(crate::dtos::labordtos::PaginatedResponse {
        status: "success".to_string(),
        data: worker_responses,
        total: 0,
        page,
        limit,
        total_pages: 0,
    }))
}

pub async fn assign_worker_to_job(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let worker_id: Uuid = body["worker_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| HttpError::bad_request("Invalid worker_id"))?;

    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::unauthorized("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not your Job"));
    }

    let updated_job = app_state.db_client
        .assign_worker_to_job(job_id, worker_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let _job = app_state.db_client
        .update_job_status(job_id, JobStatus::InProgress)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Worker assigned successfully",
        updated_job,
    )))
}

pub async fn submit_job_progress(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<SubmitProgressDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    //Verify the worker is assigned to the job
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    let worker_profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    if job.assigned_worker_id != Some(worker_profile.id) {
        return Err(HttpError::unauthorized("Not assigned to this Job"));
    }

    let progress = app_state.db_client
        .submit_job_progress(job_id, 
            worker_profile.id, 
            body.progress_percentage, 
            body.description, 
            body.image_urls
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    //if 100% complete, update job status
    if body.progress_percentage == 100 {
        let _update_job = app_state.db_client
            .update_job_status(job_id, JobStatus::UnderReview)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
    }

    Ok(Json(ApiResponse::success(
        "Progress submitted succesfully",
        progress,
    )))
}

pub async fn complete_job(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    //Verify Job belongs to employer
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not your Job"));
    }

    if job.status != Some(JobStatus::UnderReview) {
        return Err(HttpError::bad_request("Job is not ready for completion"));
    }

    //Update job status
    let updated_job = app_state.db_client
        .update_job_status(job_id, JobStatus::Completed)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Release escrow payment
    // This would integrate with your payment system
    
    // Award trust points
    if let Some(worker_id) = job.assigned_worker_id {
        let worker_profile = app_state.db_client
            .get_worker_profile(worker_id)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        
            let _ = app_state.db_client
                .award_job_completion_points(worker_profile.user_id, 
                    auth.user.id, 
                    5, 
                    true
                )
                .await;
        
    }

    Ok(Json(ApiResponse::success(
        "Job completed successfully", 
        updated_job,
    )))
}

pub async fn create_dispute(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateDisputeDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    //Verify user is involved in the job
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    let worker_profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let is_employer = job.employer_id == auth.user.id;
    let is_worker = job.assigned_worker_id == Some(worker_profile.id);

    if !is_employer && !is_worker {
        return Err(HttpError::unauthorized("Not involved in this job"));
    }

    let against_id = if is_employer {
        //Get workers user id
        if let Some(worker_id) = job.assigned_worker_id {
            worker_id
        } else {
            return Err(HttpError::bad_request("No worker assigned"));
        }
    } else {
        job.employer_id
    };

    let dispute = app_state.db_client
        .create_dispute(job_id, 
            auth.user.id, 
            against_id, 
            body.reason, 
            body.description, 
            body.evidence_urls,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    //update job status to disputed
        let _update_job = app_state.db_client
            .update_job_status(job_id, JobStatus::Disputed)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        Ok(Json(ApiResponse::success(
            "Dispute created successfully", 
            dispute
    )))
}

pub async fn resolve_dispute(
    Path(dispute_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<ResolveDisputeDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    //Verify user is a verifier
    if !matches!(auth.user.role, UserRole::Verifier | UserRole::Admin) {
        return Err(HttpError::unauthorized("Only verifiers can resolve disputes"));
    }

    let decision = body.decision.clone();

    let dispute = app_state.db_client
        .resolve_dispute(
            dispute_id, 
            body.resolution, 
            body.decision
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;


    //Handle payment based on desicion
    match decision.as_str() {
        "favor_employer" => {
            //Refund to employer
            ()
        },
        "favor_worker" => {
            // Pay worker full amount
        }
        "partial_payment" => {
            // Pay worker partial amount based on percentage
        }
        _ => return Err(HttpError::bad_request("Invalid decision")),
    }

    Ok(Json(ApiResponse::success(
        "Dispute resolved successfully",
        dispute
    )))
}














// Worker Profile Handlers
pub async fn get_worker_profile(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<Json<WorkerProfile>, HttpError> {
    let profile = app_state
        .db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(profile))
}

pub async fn update_worker_availability(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<UpdateAvailabilityDto>,
) -> Result<Json<WorkerProfile>, HttpError> {
    // Get worker profile to ensure user is a worker
    let worker_profile = app_state
        .db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let updated_profile = app_state
        .db_client
        .update_worker_availability(worker_profile.id, body.is_available)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(updated_profile))
}

// Portfolio Handlers
pub async fn add_portfolio_item(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<AddPortfolioItemDto>,
) -> Result<Json<WorkerPortfolio>, HttpError> {
    // Get worker profile to ensure user is a worker
    let worker_profile = app_state
        .db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let portfolio_item = app_state
        .db_client
        .add_portfolio_item(
            worker_profile.id,
            body.title,
            body.description,
            body.image_url,
            body.project_date,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(portfolio_item))
}

pub async fn get_worker_portfolio(
    Path(worker_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<Json<Vec<WorkerPortfolio>>, HttpError> {
    let portfolio = app_state
        .db_client
        .get_worker_portfolio(worker_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(portfolio))
}

// Job Search and Listing Handlers
// In labour.rs - Add detailed debugging
pub async fn search_jobs(
    Extension(app_state): Extension<Arc<AppState>>,
    Query(params): Query<SearchJobsDto>,
) -> Result<impl IntoResponse, HttpError> {
    params.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    println!("🔍 Search jobs called with params: {:?}", params);

    // Debug: Check what happens with each query method
    let jobs_result = if let Some(state) = &params.location_state {
        if let Some(category) = params.category {
            println!("🔍 Calling get_jobs_by_location_and_category: state={}, category={:?}", state, category);
            app_state.db_client
                .get_jobs_by_location_and_category(state, category, JobStatus::Open)
                .await
        } else {
            println!("🔍 Calling get_jobs_by_location: state={}", state);
            app_state.db_client
                .get_jobs_by_location(state, JobStatus::Open)
                .await
        }
    } else if let Some(category) = params.category {
        println!("🔍 Calling get_jobs_by_category: category={:?}", category);
        app_state.db_client
            .get_jobs_by_category(category, JobStatus::Open)
            .await
    } else {
        println!("🔍 Calling get_open_jobs");
        app_state.db_client
            .get_open_jobs()
            .await
    };

    let jobs = match jobs_result {
        Ok(jobs) => {
            println!("✅ Database query successful, found {} jobs", jobs.len());
            jobs
        }
        Err(e) => {
            println!("❌ Database query failed: {}", e);
            return Err(HttpError::server_error(e.to_string()));
        }
    };

    // Debug each job
    println!("📋 Jobs details:");
    for (i, job) in jobs.iter().enumerate() {
        println!("  {}. ID: {}", i + 1, job.id);
        println!("     Title: {}", job.title);
        println!("     Category: {:?}", job.category);
        println!("     Status: {:?}", job.status);
        println!("     State: {}", job.location_state);
        println!("     City: {}", job.location_city);
    }

    Ok(Json(ApiResponse {
        status: "success".to_string(),
        message: "Jobs retrieved successfully".to_string(),
        data: jobs,
    }))
}

pub async fn get_job_details(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<Json<Job>, HttpError> {
    let job = app_state
        .db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    Ok(Json(job))
}

pub async fn get_job_applications(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<Json<Vec<JobApplication>>, HttpError> {
    // Verify user owns the job
    let job = app_state
        .db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to view applications for this job"));
    }

    let applications = app_state
        .db_client
        .get_job_applications(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(applications))
}

pub async fn create_job_contract(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateContractDto>,
) -> Result<Json<JobContract>, HttpError> {
    // Verify user owns the job
    let job = app_state
        .db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to create contract for this job"));
    }

    let contract = app_state
        .db_client
        .create_job_contract(
            job_id,
            auth.user.id,
            body.worker_id,
            body.agreed_rate,
            body.agreed_timeline,
            body.terms,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(contract))
}

pub async fn get_job_progress(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<Json<Vec<JobProgress>>, HttpError> {
    let progress = app_state
        .db_client
        .get_job_progress(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(progress))
}

pub async fn create_job_review(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateReviewDto>,
) -> Result<Json<JobReview>, HttpError> {
    let review = app_state
        .db_client
        .create_job_review(
            job_id,
            auth.user.id,
            auth.user.id,
            body.rating,
            body.comment,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(review))
}

// Escrow Handlers
pub async fn create_escrow(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateEscrowDto>,
) -> Result<Json<EscrowTransaction>, HttpError> {
    // Verify user owns the job
    let job = app_state
        .db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to create escrow for this job"));
    }

    let escrow = app_state
        .db_client
        .create_escrow_transaction(
            job_id,
            auth.user.id,
            body.worker_id,
            body.amount,
            body.platform_fee,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(escrow))
}

pub async fn release_payment(
    Path(job_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<ReleasePaymentDto>,
) -> Result<Json<EscrowTransaction>, HttpError> {
    // Verify user owns the job
    let job = app_state
        .db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to release payment for this job"));
    }

    let escrow = app_state
        .db_client
        .release_escrow_payment(body.escrow_id, body.release_percentage)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(escrow))
}

// Dispute Handlers
pub async fn get_pending_verifications(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<Json<Vec<Dispute>>, HttpError> {
    let disputes = app_state
        .db_client
        .get_pending_verifications(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(disputes))
}

// Worker Details Handler
pub async fn get_worker_details(
    Path(worker_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<Json<WorkerProfileResponse>, HttpError> {
    let worker_profile = app_state
        .db_client
        .get_worker_profile(worker_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let portfolio = app_state
        .db_client
        .get_worker_portfolio(worker_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let reviews = app_state
        .db_client
        .get_worker_reviews(worker_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response = WorkerProfileResponse {
        profile: worker_profile,
        portfolio,
        reviews,
    };

    Ok(Json(response))
}

// Dashboard Handlers
pub async fn get_worker_dashboard(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<Json<WorkerDashboard>, HttpError> {
    let profile = app_state
        .db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let portfolio = app_state
        .db_client
        .get_worker_portfolio(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let reviews = app_state
        .db_client
        .get_worker_reviews(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let dashboard = WorkerDashboard {
        profile,
        portfolio,
        reviews,
        active_jobs: vec![], // You'll need to implement this
        pending_applications: vec![], // You'll need to implement this
    };

    Ok(Json(dashboard))
}

pub async fn get_employer_dashboard(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<Json<EmployerDashboard>, HttpError> {
    // Get jobs posted by this employer
    // You'll need to implement a method to get jobs by employer
    let posted_jobs = vec![]; // Placeholder

    let dashboard = EmployerDashboard {
        posted_jobs,
        active_contracts: vec![], // You'll need to implement this
        pending_applications: vec![], // You'll need to implement this
    };

    Ok(Json(dashboard))
}

// Contract Signing Handler
pub async fn sign_contract(
    Path(contract_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<SignContractDto>,
) -> Result<Json<JobContract>, HttpError> {
    let contract = app_state
        .db_client
        .sign_contract(contract_id, body.signer_role)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(contract))
}