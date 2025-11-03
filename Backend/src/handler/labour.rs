// handlers/labour.rs (Complete)
use std::sync::Arc;
use axum::{
    extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, routing::{delete, get, post, put}, Extension, Json, Router
};
use chrono::Utc;
use rand::Rng;
use uuid::Uuid;
use validator::Validate;
use num_traits::ToPrimitive;

use crate::{
    db::{
        labourdb::LaborExt::{self},
        userdb::UserExt, verificationdb::VerificationExt,
    }, dtos::{labordtos::*, userdtos::FilterUserDto}, error::HttpError, 
        mail::mails, middleware::JWTAuthMiddeware, models::{labourmodel::*, 
        usermodel::User, verificationmodels::OtpPurpose}, service::{
    }, AppState
};

pub fn labour_handler() -> Router {
    Router::new()
        // Worker profile routes
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
        .route("/worker/portfolio/:item_id", delete(delete_portfolio_item)) // NEW
        .route("/workers/:worker_id/portfolio", get(get_worker_public_portfolio))
        
        // Dispute management routes
        .route("/jobs/:job_id/dispute", post(create_dispute))
        .route("/disputes/:dispute_id/resolve", put(resolve_dispute))
        .route("/disputes/pending", get(get_pending_verifications))
        
        // Search and discovery routes
        .route("/workers/search", get(search_workers))
        // .route("/workers/:worker_id", get(get_worker_details))
        .route("/workers/:worker_identifier", get(get_worker_details_smart))
        
        // Dashboard routes
        .route("/worker/dashboard", get(get_worker_dashboard))
        .route("/employer/dashboard", get(get_employer_dashboard))
        
        // Contract management
        .route("/contracts/:contract_id/sign", put(sign_contract))
        
        // Application management
        .route("/applications/:application_id/status", put(update_application_status))
        
        // Escrow routes
        .route("/jobs/:job_id/escrow", get(get_job_escrow))
        .route("/jobs/:job_id/escrow/release", post(release_escrow_payment))
}

// Worker Profile Handlers
pub async fn create_worker_profile(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateWorkerProfileDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Check if profile already exists
    let existing_profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await;

    if existing_profile.is_ok() {
        return Err(HttpError::bad_request("Worker profile already exists"));
    }

    let worker_profile = app_state.db_client
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
        worker_profile,
    )))
}

pub async fn get_worker_profile(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Worker profile retrieved successfully",
        profile,
    )))
}

pub async fn update_worker_availability(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<UpdateAvailabilityDto>,
) -> Result<impl IntoResponse, HttpError> {
    let worker_profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let updated_profile = app_state.db_client
        .update_worker_availability(worker_profile.id, body.is_available)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Worker availability updated successfully",
        updated_profile,
    )))
}

// Portfolio Handlers - FIXED VERSION
pub async fn add_portfolio_item(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<AddPortfolioItemDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Get worker profile for the authenticated user
    let worker_profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let portfolio_item = app_state.db_client
        .add_portfolio_item(
            worker_profile.id, // Use worker_profile.id, not auth.user.id
            body.title,
            body.description,
            body.image_url,
            body.project_date,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Portfolio item added successfully",
        portfolio_item,
    )))
}


// FIXED: Get portfolio for authenticated worker
pub async fn get_worker_portfolio(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    // Get worker profile first to get the profile ID
    let worker_profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let portfolio = app_state.db_client
        .get_worker_portfolio(worker_profile.id) // Use worker_profile.id
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Worker portfolio retrieved successfully",
        portfolio,
    )))
}

// ADD THIS: Delete portfolio item handler
pub async fn delete_portfolio_item(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(item_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    // Verify the portfolio item belongs to the authenticated worker
    let worker_profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let portfolio_item = app_state.db_client
        .get_portfolio_item_by_id(item_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Portfolio item not found"))?;

    // Check ownership
    if portfolio_item.worker_id != worker_profile.id {
        return Err(HttpError::unauthorized("Not authorized to delete this portfolio item"));
    }

    app_state.db_client
        .delete_portfolio_item(item_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Portfolio item deleted successfully",
        (),
    )))
}

// ADD THIS: Get specific worker's portfolio (public endpoint)
pub async fn get_worker_public_portfolio(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(worker_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let portfolio = app_state.db_client
        .get_worker_portfolio(worker_id) // This expects worker_profile.id
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Worker portfolio retrieved successfully",
        portfolio,
    )))
}


// Job Management Handlers
pub async fn create_job(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateJobDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let result = app_state.labour_service
        .create_job_with_escrow(auth.user.id, body)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;


    let _ = app_state.notification_service.notify_new_job(&result.clone()).await;

    Ok(Json(ApiResponse::success(
        "Job created successfully",
        result,
    )))
}

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


    Ok(Json(JobsResponse {
        status: "success".to_string(),
        message: "Jobs retrieved successfully".to_string(),
        data: jobs, // Direct array
    }))
}

pub async fn get_job_details(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    Ok(Json(ApiResponse::success(
        "Job details retrieved successfully",
        job,
    )))
}

pub async fn apply_to_job(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateJobApplicationDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Verify job exists and is open
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.status != Some(JobStatus::Open) {
        return Err(HttpError::bad_request("Job is not open for applications"));
    }

    // Check if worker has a profile
    let worker_profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Check if worker is in the same state
    if worker_profile.location_state != job.location_state {
        return Err(HttpError::bad_request(
            "Worker must be in the same state as the job"
        ));
    }

    // Check if worker category matches job category
    if worker_profile.category != job.category {
        return Err(HttpError::bad_request("Worker category does not match job category"));
    }

    let application = app_state.db_client
        .create_job_application(
            job_id,
            worker_profile.id,
            body.proposed_rate,
            body.estimated_completion,
            body.cover_letter,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

     let _ = app_state.notification_service.notify_job_application(
        job.employer_id,
        &job,
        "Applicant", // You might want to fetch user name
    ).await;

    Ok(Json(ApiResponse::success(
        "Application submitted successfully",
        application,
    )))
}

// pub async fn assign_worker_to_job(
//     Extension(app_state): Extension<Arc<AppState>>,
//     Path(job_id): Path<Uuid>,
//     Extension(auth): Extension<JWTAuthMiddeware>,
//     Json(body): Json<AssignWorkerDto>,
// ) -> Result<impl IntoResponse, HttpError> {
//     let worker_profile_id = body.worker_id; // This is worker_profile.id

//     // Verify job exists and user owns it
//     let job = app_state.db_client
//         .get_job_by_id(job_id)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?
//         .ok_or_else(|| HttpError::not_found("Job not found"))?;

//     if job.employer_id != auth.user.id {
//         return Err(HttpError::unauthorized("Not authorized to assign workers to this job"));
//     }

//     // Verify worker exists and has applied to this job
//     let applications = app_state.db_client
//         .get_job_applications(job_id)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?;

//     let worker_application = applications.iter()
//         .find(|app| app.worker_id == worker_profile_id)
//         .ok_or_else(|| HttpError::bad_request("Worker has not applied to this job"))?;

//     // Get the worker profile to get the user_id
//     let worker_profile = app_state.db_client
//         .get_worker_profile_by_id(worker_profile_id)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?;

//     // Assign worker and create escrow - use worker_profile.user_id for assignment
//     let result = app_state.labour_service
//         .assign_worker_to_job(job_id, auth.user.id, worker_profile.user_id)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?;

//     // Create contract automatically
//     let contract = app_state.db_client
//         .create_job_contract(
//             job_id,
//             auth.user.id,
//             worker_profile.user_id, // Use user_id for contract
//             worker_application.proposed_rate.to_f64().unwrap_or(0.0),
//             worker_application.estimated_completion,
//             format!("Standard contract for job: {}. Agreed rate: {}, Timeline: {} days", 
//                    job.title, 
//                    worker_application.proposed_rate.to_f64().unwrap_or(0.0),
//                    worker_application.estimated_completion),
//         )
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?;

//     // Notify worker
//     let _ = app_state.notification_service.notify_job_assigned_to_worker(worker_profile.user_id, &job).await;

//     Ok(Json(ApiResponse::success(
//         "Worker assigned successfully and contract created",
//         AssignWorkerResponse {
//             job: result.job,
//             escrow: result.escrow,
//             contract,
//         },
//     )))
//

pub async fn assign_worker_to_job(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<AssignWorkerDto>,
) -> Result<impl IntoResponse, HttpError> {
    let worker_profile_id = body.worker_id;

    println!("🔍 [assign_worker_to_job] Assigning worker profile: {} to job: {}", worker_profile_id, job_id);

    // Verify job exists and user owns it
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to assign workers to this job"));
    }

    // Step 1: Get worker profile to get the user_id
    let worker_profile = app_state.db_client
        .get_worker_profile_by_id(worker_profile_id)
        .await
        .map_err(|e| {
            println!("❌ [assign_worker_to_job] Error fetching worker profile: {}", e);
            HttpError::server_error(e.to_string())
        })?;

    let worker_user_id = worker_profile.user_id;
    println!("✅ [assign_worker_to_job] Found worker user_id: {}", worker_user_id);

    // Step 2: Verify worker has applied to this job
    let applications = app_state.db_client
        .get_job_applications(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let worker_application = applications.iter()
        .find(|app| app.worker_id == worker_profile_id)
        .ok_or_else(|| HttpError::bad_request(
            "Worker has not applied to this job. Please ask them to apply first."
        ))?;

    println!("✅ [assign_worker_to_job] Found application from worker");

    // Step 3: Assign worker using USER_ID (critical for foreign key)
    let result = app_state.labour_service
        .assign_worker_to_job(job_id, auth.user.id, worker_user_id)
        .await
        .map_err(|e| {
            println!("❌ [assign_worker_to_job] Service error: {}", e);
            HttpError::server_error(e.to_string())
        })?;

    // Step 4: Create contract using USER_ID
    let contract = app_state.db_client
        .create_job_contract(
            job_id,
            auth.user.id,
            worker_user_id, // Use USER_ID for contract
            worker_application.proposed_rate.to_f64().unwrap_or(0.0),
            worker_application.estimated_completion,
            format!(
                "Standard contract for job: {}. Agreed rate: {}, Timeline: {} days", 
                job.title, 
                worker_application.proposed_rate.to_f64().unwrap_or(0.0),
                worker_application.estimated_completion
            ),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Notify worker
    let _ = app_state.notification_service
        .notify_job_assigned_to_worker(worker_user_id, &job)
        .await;

    println!("✅ [assign_worker_to_job] Worker assigned successfully");

    Ok(Json(ApiResponse::success(
        "Worker assigned successfully and contract created",
        AssignWorkerResponse {
            job: result.job,
            escrow: result.escrow,
            contract,
        },
    )))
}

// FIXED: Enhanced job applications with proper worker data
pub async fn get_job_applications(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to view applications for this job"));
    }

    let applications = app_state.db_client
        .get_job_applications(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let mut application_responses = Vec::new();
    
    for app in applications {
        println!("🔍 Processing application: {:?}", app);

        // Get worker profile using profile_id from application
        let worker_profile_result = app_state.db_client
            .get_worker_profile_by_id(app.worker_id)
            .await;

        let (worker_user_id, worker_profile_data) = match worker_profile_result {
            Ok(profile) => {
                println!("✅ Found worker profile: user_id={}, profile_id={}", profile.user_id, profile.id);
                (Some(profile.user_id), Some(profile))
            },
            Err(e) => {
                println!("❌ Error fetching worker profile: {}", e);
                (None, None)
            }
        };

        // Get worker user details
        let worker_user = if let Some(user_id) = worker_user_id {
            match app_state.db_client
                .get_user(Some(user_id), None, None, None)
                .await
            {
                Ok(Some(user)) => Some(WorkerUserResponse {
                    id: user.id,
                    name: user.name,
                    email: user.email,
                    username: user.username,
                    avatar_url: user.avatar_url,
                    trust_score: user.trust_score,
                    verified: user.verified,
                }),
                _ => None,
            }
        } else {
            None
        };

        // Get portfolio using worker_profile.id (correct foreign key)
        let worker_portfolio = if let Some(profile) = &worker_profile_data {
            app_state.db_client
                .get_worker_portfolio(profile.id)
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };

        // Get reviews using worker_user_id
        let worker_reviews = if let Some(user_id) = worker_user_id {
            app_state.db_client
                .get_worker_reviews(user_id)
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };

        let worker_profile_response = worker_profile_data.map(|profile| WorkerProfileApplicationResponse {
            profile_id: profile.id,
            category: profile.category.to_str().to_string(),
            experience_years: profile.experience_years,
            description: profile.description,
            hourly_rate: profile.hourly_rate.as_ref().and_then(|bd| bd.to_f64()).unwrap_or(0.0),
            daily_rate: profile.daily_rate.as_ref().and_then(|bd| bd.to_f64()).unwrap_or(0.0),
            location_state: profile.location_state,
            location_city: profile.location_city,
            is_available: profile.is_available.unwrap_or(false),
            rating: profile.rating.unwrap_or(0.0),
            completed_jobs: profile.completed_jobs.unwrap_or(0),
            skills: vec![],
        });

        application_responses.push(JobApplicationResponse {
            id: app.id,
            job_id: app.job_id,
            worker_id: app.worker_id, // This is profile_id
            worker_user_id: worker_user_id, // This is user_id - important for frontend
            proposed_rate: app.proposed_rate.to_f64().unwrap_or(0.0),
            estimated_completion: app.estimated_completion,
            cover_letter: app.cover_letter,
            status: app.status.unwrap_or_default(),
            created_at: app.created_at.unwrap_or_else(Utc::now),
            worker: worker_user,
            worker_profile: worker_profile_response,
            worker_portfolio: worker_portfolio,
            worker_reviews: worker_reviews,
        });
    }

    Ok(Json(ApiResponse::success(
        "Job applications retrieved successfully",
        application_responses,
    )))
}


// Helper function to resolve worker identifier to both user_id and profile_id
async fn resolve_worker_identifiers(
    app_state: &Arc<AppState>,
    worker_identifier: Uuid,
) -> Result<(Uuid, Uuid), HttpError> {
    println!("🔍 [resolve_worker_identifiers] Resolving identifier: {}", worker_identifier);

    // Try as user_id first (get_worker_profile expects user_id)
    match app_state.db_client.get_worker_profile(worker_identifier).await {
        Ok(profile) => {
            println!("✅ [resolve_worker_identifiers] Found as user_id");
            return Ok((profile.user_id, profile.id));
        }
        Err(_) => {
            println!("🔄 [resolve_worker_identifiers] Not found as user_id, trying as profile_id...");
        }
    }

    // Try as profile_id
    match app_state.db_client.get_worker_profile_by_id(worker_identifier).await {
        Ok(profile) => {
            println!("✅ [resolve_worker_identifiers] Found as profile_id");
            Ok((profile.user_id, profile.id))
        }
        Err(e) => {
            println!("❌ [resolve_worker_identifiers] Not found as either ID type");
            Err(HttpError::not_found(
                "Worker not found. The identifier provided doesn't match any worker profile."
            ))
        }
    }
}


pub async fn create_job_contract(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateContractDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Verify user owns the job
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to create contract for this job"));
    }

    let contract = app_state.db_client
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
        
    Ok(Json(ApiResponse::success(
        "Job contract created successfully",
        contract,
    )))
}

pub async fn submit_job_progress(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<SubmitProgressDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let result = app_state.labour_service
        .submit_job_progress(job_id, auth.user.id, body)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    let _ = app_state.notification_service.notify_progress_update(
        job.employer_id,
        &result.progress,
    ).await;

    Ok(Json(ApiResponse::success(
        "Progress submitted successfully",
        result,
    )))
}

pub async fn get_job_progress(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let progress = app_state.db_client
        .get_job_progress(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Job progress retrieved successfully",
        progress,
    )))
}

pub async fn complete_job(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let result = app_state.labour_service
        .complete_job(job_id, auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    // Notify worker
    if let Some(worker_id) = job.assigned_worker_id {
        let _ = app_state.notification_service.notify_job_completion(
            worker_id,
            &job,
        ).await;
    }

    // Notify employer
    let _ = app_state.notification_service.notify_job_completion(
        job.employer_id,
        &job,
    ).await;

    Ok(Json(ApiResponse::success(
        "Job completed successfully",
        result,
    )))
}

pub async fn create_job_review(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateReviewDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Determine who is being reviewed (employer or worker)
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    let reviewee_id = if auth.user.id == job.employer_id {
        // Employer is reviewing the worker
        job.assigned_worker_id.ok_or_else(|| HttpError::bad_request("No worker assigned to job"))?
    } else {
        // Worker is reviewing the employer
        job.employer_id
    };

    let review = app_state.db_client
        .create_job_review(
            job_id,
            auth.user.id,
            reviewee_id,
            body.rating,
            body.comment,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Update worker rating if worker was reviewed
    if auth.user.id == job.employer_id {
        let _ = app_state.db_client.update_worker_rating(reviewee_id).await;
    }

    Ok(Json(ApiResponse::success(
        "Review created successfully",
        review,
    )))
}

// Dispute Handlers
pub async fn create_dispute(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateDisputeDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    // Determine who the dispute is against
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    let against = if auth.user.id == job.employer_id {
        // Employer is raising dispute against worker
        job.assigned_worker_id.ok_or_else(|| HttpError::bad_request("No worker assigned to job"))?
    } else {
        // Worker is raising dispute against employer
        job.employer_id
    };

    let result = app_state.dispute_service
        .create_dispute(
            job_id,
            auth.user.id,
            against,
            body.reason,
            body.description,
            body.evidence_urls,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

     // ADD: Notify both parties about dispute
    let _ = app_state.notification_service.notify_dispute_creation(
        auth.user.id,
        against,
        &result.dispute,
    ).await;


    Ok(Json(ApiResponse::success(
        "Dispute created successfully",
        result,
    )))
}

pub async fn resolve_dispute(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(dispute_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<ResolveDisputeDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let result = app_state.dispute_service
        .resolve_dispute(
            dispute_id,
            auth.user.id,
            body.resolution,
            body.decision.clone(),
            body.payment_percentage,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // ADD: Notify both parties about dispute resolution
    let _ = app_state.notification_service.notify_dispute_resolution(
        result.dispute.raised_by,
        result.dispute.against,
        &result.dispute,
        &body.decision,
    ).await;


    Ok(Json(ApiResponse::success(
        "Dispute resolved successfully",
        result,
    )))
}

pub async fn get_pending_verifications(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let disputes = app_state.db_client
        .get_pending_verifications_f(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Pending verifications retrieved successfully",
        disputes,
    )))
}

// Search and Discovery Handlers
pub async fn search_workers(
    Extension(app_state): Extension<Arc<AppState>>,
    Query(params): Query<SearchWorkersDto>,
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
                limit as i64,
                offset,
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
    } else {
        vec![]
    };

    // Convert to response objects with additional data
    let worker_responses = futures::future::join_all(
        workers.into_iter().map(|worker| async {
            // Get portfolio and reviews for each worker
            let portfolio = app_state.db_client
                .get_worker_portfolio(worker.user_id)
                .await
                .unwrap_or_default();
            let reviews = app_state.db_client
                .get_worker_reviews(worker.user_id)
                .await
                .unwrap_or_default();

            
            // Get the user info
            match app_state.db_client
                .get_user(Some(worker.user_id), None, None, None)
                .await
            {
                
                Ok(Some(worker_user)) => Ok(WorkerProfileResponses {
                    user: worker_user,
                    profile: worker,
                    portfolio,
                    reviews,
                }),
                Ok(None) => Err(HttpError::not_found("Worker user not found")),
                Err(e) => Err(HttpError::server_error(e.to_string())),
            }
        })
    ).await;

    // Filter out any errors from the responses
    let worker_responses: Vec<_> = worker_responses
        .into_iter()
        .filter_map(Result::ok)
        .collect();

    let total = worker_responses.len() as i64;
    let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;

    Ok(Json(PaginatedResponse {
        status: "success".to_string(),
        data: worker_responses,
        total,
        page,
        limit,
        total_pages,
    }))
}

pub async fn get_worker_details(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(worker_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let worker_profile = app_state.db_client
        .get_worker_profile(worker_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let portfolio = app_state.db_client
        .get_worker_portfolio(worker_profile.id.clone())
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let reviews = app_state.db_client
        .get_worker_reviews(worker_profile.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let worker_user = app_state.db_client
        .get_user(Some(worker_profile.user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Worker user not found"))?;

    let filtered_user = FilterUserDto::filter_user(&worker_user);

    let response = WorkerProfileResponse {
        user: filtered_user,
        profile: worker_profile,
        portfolio,
        reviews,
    };

    Ok(Json(ApiResponse::success(
        "Worker details retrieved successfully",
        response,
    )))
}

// In labour.rs - FIXED smart worker details endpoint
pub async fn get_worker_details_smart(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(worker_identifier): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    println!("🔍 [get_worker_details_smart] Smart lookup for: {}", worker_identifier);
    
    let worker_profile;
    let mut found_as = "unknown";
    
    // First try: Assume it's a user_id and look up worker profile
    match app_state.db_client.get_worker_profile(worker_identifier).await {
        Ok(profile) => {
            println!("✅ [get_worker_details_smart] Found by user_id");
            worker_profile = profile;
            found_as = "user_id";
        },
        Err(_) => {
            // Second try: Assume it's a profile_id and look up worker profile by ID
            println!("🔄 [get_worker_details_smart] Trying as profile_id...");
            match app_state.db_client.get_worker_profile_by_id(worker_identifier).await {
                Ok(profile) => {
                    println!("✅ [get_worker_details_smart] Found by profile_id");
                    worker_profile = profile;
                    found_as = "profile_id";
                },
                Err(e) => {
                    println!("❌ [get_worker_details_smart] Not found as user_id or profile_id: {}", e);
                    return Err(HttpError::not_found("Worker not found with the provided identifier"));
                }
            }
        }
    }

    println!("🔍 [get_worker_details_smart] Found worker profile - ID: {}, User ID: {}, Found as: {}", 
        worker_profile.id, worker_profile.user_id, found_as);

    let worker_profile = app_state.db_client
        .get_worker_profile_by_id(worker_identifier)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Get portfolio using worker_profile.id (this is the CORRECT foreign key)
    let portfolio = match app_state.db_client.get_worker_portfolio(worker_profile.id).await {
        Ok(portfolio_items) => {
            println!("✅ [get_worker_details_smart] Found {} portfolio items", portfolio_items.len());
            portfolio_items
        },
        Err(e) => {
            println!("⚠️ [get_worker_details_smart] Error fetching portfolio: {}", e);
            vec![]
        }
    };

    // Get reviews using worker_profile.user_id (reviews are linked to user_id, not profile_id)
    let reviews = match app_state.db_client.get_worker_reviews(worker_profile.user_id).await {
        Ok(worker_reviews) => {
            println!("✅ [get_worker_details_smart] Found {} reviews", worker_reviews.len());
            worker_reviews
        },
        Err(e) => {
            println!("⚠️ [get_worker_details_smart] Error fetching reviews: {}", e);
            vec![]
        }
    };

    // Get worker user details using worker_profile.user_id
    let worker_user = app_state.db_client
        .get_user(Some(worker_profile.user_id), None, None, None)
        .await
        .map_err(|e| {
            println!("❌ [get_worker_details_smart] Error fetching user: {}", e);
            HttpError::server_error(e.to_string())
        })?
        .ok_or_else(|| {
            println!("❌ [get_worker_details_smart] User not found for user_id: {}", worker_profile.user_id);
            HttpError::not_found("Worker user not found")
        })?;

    let filtered_user = FilterUserDto::filter_user(&worker_user);

    let response = WorkerProfileResponse {
        user: filtered_user,
        profile: worker_profile,
        portfolio,
        reviews,
    };

    println!("✅ [get_worker_details_smart] Successfully built response for worker");
    Ok(Json(ApiResponse::success(
        "Worker details retrieved successfully",
        response,
    )))
}

// Dashboard Handlers
pub async fn get_worker_dashboard(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let profile = app_state.db_client
        .get_worker_profile(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let portfolio = app_state.db_client
        .get_worker_portfolio(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let reviews = app_state.db_client
        .get_worker_reviews(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Get active jobs (jobs where worker is assigned and status is in_progress)
    let active_jobs = app_state.db_client
        .get_worker_active_jobs(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Get pending applications
    let pending_applications = app_state.db_client
        .get_worker_pending_applications(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let dashboard = WorkerDashboard {
        profile,
        portfolio,
        reviews,
        active_jobs,
        pending_applications,
    };

    Ok(Json(ApiResponse::success(
        "Worker dashboard retrieved successfully",
        dashboard,
    )))
}

pub async fn get_employer_dashboard(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    // Get jobs posted by this employer
    let posted_jobs = app_state.db_client
        .get_employer_jobs(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Get active contracts
    let active_contracts = app_state.db_client
        .get_employer_active_contracts(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Get pending applications across all jobs
    let pending_applications = app_state.db_client
        .get_employer_pending_applications(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let dashboard = EmployerDashboard {
        posted_jobs,
        active_contracts,
        pending_applications,
    };

    Ok(Json(ApiResponse::success(
        "Employer dashboard retrieved successfully",
        dashboard,
    )))
}

// // Contract Management
// pub async fn sign_contract(
//     Extension(app_state): Extension<Arc<AppState>>,
//     Path(contract_id): Path<Uuid>,
//     Extension(auth): Extension<JWTAuthMiddeware>,
//     Json(body): Json<SignContractDto>,
// ) -> Result<impl IntoResponse, HttpError> {
//     let contract = app_state.db_client
//         .sign_contract(contract_id, body.signer_role)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?;

//     Ok(Json(ApiResponse::success(
//         "Contract signed successfully",
//         contract,
//     )))
// }

pub async fn sign_contract(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(contract_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<SignContractDto>,
) -> Result<impl IntoResponse, HttpError> {
    // Get contract and verify user is a participant
    let contract_result = sqlx::query_as::<_, JobContract>(
        "SELECT * FROM job_contracts WHERE id = $1"
    )
    .bind(contract_id)
    .fetch_optional(&app_state.db_client.pool)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?
    .ok_or_else(|| HttpError::not_found("Contract not found"))?;

    // Determine user role
    let signer_role = if contract_result.employer_id == auth.user.id {
        "employer"
    } else if contract_result.worker_id == auth.user.id {
        "worker"
    } else {
        return Err(HttpError::unauthorized("Not authorized to sign this contract"));
    };

    // Check if already signed
    let already_signed = match signer_role {
        "employer" => contract_result.signed_by_employer.unwrap_or(false),
        "worker" => contract_result.signed_by_worker.unwrap_or(false),
        _ => false,
    };

    if already_signed {
        return Err(HttpError::bad_request("Contract already signed by you"));
    }

    // If requesting OTP, send it and return
    if body.request_otp == Some(true) {
        let otp_code = format!("{:06}", rand::rng().random_range(0..1_000_000));
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);
        
        // Store OTP
        let _ = app_state.db_client
            .create_otp(
                auth.user.id,
                auth.user.email.clone(),
                otp_code.clone(),
                OtpPurpose::SensitiveAction,
                expires_at,
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        // Send email
        let _ = mails::send_contract_signature_otp_email(
            &auth.user.email,
            &auth.user.name,
            &otp_code,
            &contract_result.agreed_rate.to_f64().unwrap_or(0.0),
            contract_result.agreed_timeline,
        ).await;

        return Ok((
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "status": "success",
                "message": "OTP sent to your email. Please verify to sign contract."
            }))
        ).into_response());
    }

    // Verify OTP before signing
    let otp_code = body.otp_code.ok_or_else(|| 
        HttpError::bad_request("OTP code required to sign contract")
    )?;

    let otp = app_state.db_client
        .get_valid_otp(&auth.user.email, &otp_code, OtpPurpose::SensitiveAction)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::unauthorized("Invalid or expired OTP"))?;

    // Mark OTP as used
    let _ = app_state.db_client
        .mark_otp_used(otp.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Sign the contract
    let signed_contract = app_state.db_client
        .sign_contract(contract_id, signer_role.to_string())
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    // Get job details for notification
    let job = app_state.db_client
        .get_job_by_id(signed_contract.job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    // Notify other party if both haven't signed
    let both_signed = signed_contract.signed_by_employer.unwrap_or(false) 
        && signed_contract.signed_by_worker.unwrap_or(false);

    if !both_signed {
        let other_user_id = if signer_role == "employer" {
            signed_contract.worker_id
        } else {
            signed_contract.employer_id
        };

        let _ = app_state.notification_service
            .notify_contract_awaiting_signature(other_user_id, &signed_contract)
            .await;
    } else {
        // Both signed - contract is now active, create escrow if not exists
        let escrow_exists = app_state.db_client
            .get_escrow_by_job_id(job.id)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .is_some();

        if !escrow_exists && job.assigned_worker_id.is_some() {
            // Create escrow now that contract is fully signed
            let platform_fee = job.budget.to_f64().unwrap_or(0.0) * 0.03;
            let _ = app_state.db_client.create_escrow_transaction(
                job.id,
                job.employer_id,
                job.assigned_worker_id.unwrap(),
                job.budget.to_f64().unwrap_or(0.0),
                platform_fee,
            ).await;
        }

        // Notify both parties contract is active
        let _ = app_state.notification_service
            .notify_contract_fully_signed(
                signed_contract.employer_id,
                signed_contract.worker_id,
                &job,
            )
            .await;
    }

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(
            "Contract signed successfully",
            signed_contract,
        ))
    ).into_response())
}


// Application Management
pub async fn update_application_status(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(application_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let status = body["status"]
        .as_str()
        .ok_or_else(|| HttpError::bad_request("Status is required"))?
        .to_string();

    // Verify user owns the job associated with this application
    let application = app_state.db_client
        .get_job_application_by_id(application_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Application not found"))?;

    let job = app_state.db_client
        .get_job_by_id(application.job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to update this application"));
    }

    let updated_application = app_state.db_client
        .update_application_status(application_id, status)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Application status updated successfully",
        updated_application,
    )))
}

// Escrow Handlers
pub async fn get_job_escrow(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    // Verify user is involved in the job
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    let is_involved = job.employer_id == auth.user.id || 
        job.assigned_worker_id == Some(auth.user.id);
    
    if !is_involved {
        return Err(HttpError::unauthorized("Not authorized to view escrow for this job"));
    }

    let escrow = app_state.db_client
        .get_escrow_by_job_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Escrow not found for this job"))?;

    Ok(Json(ApiResponse::success(
        "Escrow details retrieved successfully",
        escrow,
    )))
}

pub async fn release_escrow_payment(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<ReleasePaymentDto>,
) -> Result<impl IntoResponse, HttpError> {
    // Verify user owns the job
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if job.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to release payment for this job"));
    }

    let escrow_release = app_state.escrow_service
        .release_partial_payment(job_id, body.release_percentage)
        .await?;

    // ADD: Notify worker about payment release
    let job = app_state.db_client
        .get_job_by_id(job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;

    if let Some(worker_id) = job.assigned_worker_id {
        let _ = app_state.notification_service.notify_payment_release(
            worker_id,
            job_id,
            escrow_release.amount.to_f64().unwrap_or(0.0),
        ).await;
    }

    Ok(Json(ApiResponse::success(
        "Escrow payment released successfully",
        escrow_release,
    )))
}

#[derive(Debug, serde::Serialize)]
pub struct WorkerProfileResponse {
    pub user: FilterUserDto,
    pub profile: WorkerProfile,
    pub portfolio: Vec<WorkerPortfolio>,
    pub reviews: Vec<JobReview>,
}

#[derive(Debug, serde::Serialize)]
pub struct WorkerProfileResponses {
    pub user: User,
    pub profile: WorkerProfile,
    pub portfolio: Vec<WorkerPortfolio>,
    pub reviews: Vec<JobReview>,
}

#[derive(Debug, serde::Serialize)]
pub struct WorkerDashboard {
    pub profile: WorkerProfile,
    pub portfolio: Vec<WorkerPortfolio>,
    pub reviews: Vec<JobReview>,
    pub active_jobs: Vec<Job>,
    pub pending_applications: Vec<JobApplication>,
}

#[derive(Debug, serde::Serialize)]
pub struct EmployerDashboard {
    pub posted_jobs: Vec<Job>,
    pub active_contracts: Vec<JobContract>,
    pub pending_applications: Vec<JobApplication>,
}

#[derive(Debug, serde::Serialize)]
pub struct PaginatedResponse<T> {
    pub status: String,
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, serde::Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub message: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn success(message: &str, data: T) -> Self {
        Self {
            status: "success".to_string(),
            message: message.to_string(),
            data,
        }
    }
}



// pub async fn get_job_applications(
//     Extension(app_state): Extension<Arc<AppState>>,
//     Path(job_id): Path<Uuid>,
//     Extension(auth): Extension<JWTAuthMiddeware>,
// ) -> Result<impl IntoResponse, HttpError> {
//     // Verify user owns the job
//     let job = app_state.db_client
//         .get_job_by_id(job_id)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?
//         .ok_or_else(|| HttpError::not_found("Job not found"))?;

//     if job.employer_id != auth.user.id {
//         return Err(HttpError::unauthorized("Not authorized to view applications for this job"));
//     }

//     let applications = app_state.db_client
//         .get_job_applications(job_id)
//         .await
//         .map_err(|e| HttpError::server_error(e.to_string()))?;

//     let mut application_responses = Vec::new();
    
//     for app in applications {
//         println!("🔍 Processing application: {:?}", app);

//         // Get worker profile first - this gives us the user_id
//         let worker_profile_result = app_state.db_client
//             .get_worker_profile_by_id(app.worker_id) // This is worker_profile.id
//             .await;

//         let (worker_user_id, worker_profile_data) = match worker_profile_result {
//             Ok(profile) => {
//                 println!("✅ Found worker profile: user_id={}, profile_id={}", profile.user_id, profile.id);
//                 (Some(profile.user_id), Some(profile))
//             },
//             Err(e) => {
//                 println!("❌ Error fetching worker profile for worker_id {}: {}", app.worker_id, e);
//                 (None, None)
//             }
//         };

//         // Get worker user details using the user_id from the profile
//         let worker_user = if let Some(user_id) = worker_user_id {
//             match app_state.db_client
//                 .get_user(Some(user_id), None, None, None)
//                 .await
//             {
//                 Ok(Some(user)) => {
//                     println!("✅ Found worker user: {}", user.email);
//                     Some(WorkerUserResponse {
//                         id: user.id,
//                         name: user.name,
//                         email: user.email,
//                         username: user.username,
//                         avatar_url: user.avatar_url,
//                         trust_score: user.trust_score,
//                         verified: user.verified,
//                     })
//                 },
//                 Ok(None) => {
//                     println!("⚠️ User not found for user_id: {}", user_id);
//                     None
//                 },
//                 Err(e) => {
//                     println!("⚠️ Error fetching user for user_id {}: {}", user_id, e);
//                     None
//                 },
//             }
//         } else {
//             None
//         };

//         // Get worker portfolio
//         let worker_portfolio = if let Some(profile) = &worker_profile_data {
//             app_state.db_client
//                 .get_worker_portfolio(profile.id)
//                 .await
//                 .unwrap_or_default()
//         } else {
//             vec![]
//         };

//         // Get worker reviews
//         let worker_reviews = if let Some(profile) = &worker_profile_data {
//             app_state.db_client
//                 .get_worker_reviews(profile.user_id) // Use user_id for reviews
//                 .await
//                 .unwrap_or_default()
//         } else {
//             vec![]
//         };

//         // Convert worker profile data to response format
//         let worker_profile_response = worker_profile_data.map(|profile| WorkerProfileApplicationResponse {
//             profile_id: profile.id,
//             category: profile.category.to_str().to_string(),
//             experience_years: profile.experience_years,
//             description: profile.description,
//             hourly_rate: profile.hourly_rate.as_ref().and_then(|bd| bd.to_f64()).unwrap_or(0.0),
//             daily_rate: profile.daily_rate.as_ref().and_then(|bd| bd.to_f64()).unwrap_or(0.0),
//             location_state: profile.location_state,
//             location_city: profile.location_city,
//             is_available: profile.is_available.unwrap_or(false),
//             rating: profile.rating.unwrap_or(0.0),
//             completed_jobs: profile.completed_jobs.unwrap_or(0),
//             skills: vec![],
//         });

//         application_responses.push(JobApplicationResponse {
//             id: app.id,
//             job_id: app.job_id,
//             worker_id: app.worker_id, // This is worker_profile.id
//             worker_user_id: worker_user_id, // The actual user.id for frontend
//             proposed_rate: app.proposed_rate.to_f64().unwrap_or(0.0),
//             estimated_completion: app.estimated_completion,
//             cover_letter: app.cover_letter,
//             status: app.status.unwrap_or_default(),
//             created_at: app.created_at.unwrap_or_else(Utc::now),
//             worker: worker_user,
//             worker_profile: worker_profile_response,
//             worker_portfolio: worker_portfolio,
//             worker_reviews: worker_reviews,
//         });
//     }

//     Ok(Json(ApiResponse::success(
//         "Job applications retrieved successfully",
//         application_responses,
//     )))
// }