// routes.rs - Updated with public job browsing routes
use std::sync::Arc;

use axum::{middleware, routing::{get, post, put}, Extension, Json, Router};
use tower_http::trace::TraceLayer;
use serde_json::json;

use crate::{
    handler::{
        auth::auth_handler, 
        google_oauth::oauth_handler, 
        labour::{
            search_jobs,
            get_job_details,
            search_workers,
            get_worker_details,
        }, 
        naira_wallet::{
            paystack_webhook,
            flutterwave_webhook
        }, 
        users::users_handler,
        verification::verification_handler,
    }, 
    middleware::auth, 
    AppState
};

// Health check handler
async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "message": "Server is running"
    }))
}

pub fn create_router(app_state: Arc<AppState>) -> Router {
    // Protected wallet routes (require auth)
    let protected_wallet_routes = Router::new()
        .route("/", get(crate::handler::naira_wallet::get_wallet))
        .route("/create", post(crate::handler::naira_wallet::create_wallet))
        .route("/summary", get(crate::handler::naira_wallet::get_wallet_summary))
        .route("/deposit", post(crate::handler::naira_wallet::initiate_deposit))
        .route("/withdraw", post(crate::handler::naira_wallet::withdraw_funds))
        .route("/transfer", post(crate::handler::naira_wallet::transfer_funds))
        .route("/transactions", get(crate::handler::naira_wallet::get_transaction_history))
        .route("/transaction/:reference", get(crate::handler::naira_wallet::get_transaction_by_ref))
        .route("/bank-accounts", 
        get(crate::handler::naira_wallet::get_bank_accounts)
        .post(crate::handler::naira_wallet::add_bank_account)
        )
        .route("/bank-accounts/:account_id/verify", post(crate::handler::naira_wallet::verify_bank_account))
        .route("/bank-accounts/:account_id/primary", put(crate::handler::naira_wallet::set_primary_account))
        .route("/bank-accounts/resolve", post(crate::handler::naira_wallet::resolve_account_number))
        .layer(middleware::from_fn(auth));

    // Public wallet routes (no auth required but secure)
    let public_wallet_routes = Router::new()
        .route("/deposit/verify", 
            get(crate::handler::naira_wallet::handle_paystack_redirect)
            .post(crate::handler::naira_wallet::verify_deposit))
        .route("/webhook/paystack", post(paystack_webhook))
        .route("/webhook/flutterwave", post(flutterwave_webhook));

    // Combine wallet routes
    let wallet_routes = Router::new()
        .merge(protected_wallet_routes)
        .merge(public_wallet_routes);

    // Public labour routes (no auth required - job browsing)
    let public_labour_routes = Router::new()
        .route("/jobs", get(search_jobs))
        .route("/jobs/:job_id", get(get_job_details))
        .route("/workers/search", get(search_workers))
        .route("/workers/:worker_id", get(get_worker_details));

    // Protected labour routes (require auth - job applications, profiles, etc.)
    let protected_labour_routes = Router::new()
        // Worker profile routes
        .route("/worker/profile", post(crate::handler::labour::create_worker_profile))
        .route("/worker/profile", get(crate::handler::labour::get_worker_profile))
        .route("/worker/profile/availability", put(crate::handler::labour::update_worker_availability))
        .route("/worker/portfolio", post(crate::handler::labour::add_portfolio_item))
        .route("/worker/portfolio", get(crate::handler::labour::get_worker_portfolio))
        
        // Job management routes
        .route("/jobs", post(crate::handler::labour::create_job))
        .route("/jobs/:job_id/applications", post(crate::handler::labour::apply_to_job))
        .route("/jobs/:job_id/applications", get(crate::handler::labour::get_job_applications))
        .route("/jobs/:job_id/assign", put(crate::handler::labour::assign_worker_to_job))
        .route("/jobs/:job_id/contract", post(crate::handler::labour::create_job_contract))
        .route("/jobs/:job_id/progress", post(crate::handler::labour::submit_job_progress))
        .route("/jobs/:job_id/progress", get(crate::handler::labour::get_job_progress))
        .route("/jobs/:job_id/complete", put(crate::handler::labour::complete_job))
        .route("/jobs/:job_id/review", post(crate::handler::labour::create_job_review))
        
        // Dispute management routes
        .route("/jobs/:job_id/dispute", post(crate::handler::labour::create_dispute))
        .route("/disputes/:dispute_id/resolve", put(crate::handler::labour::resolve_dispute))
        .route("/disputes/pending", get(crate::handler::labour::get_pending_verifications))
        
        // Dashboard routes
        .route("/worker/dashboard", get(crate::handler::labour::get_worker_dashboard))
        .route("/employer/dashboard", get(crate::handler::labour::get_employer_dashboard))
        
        // Contract management
        .route("/contracts/:contract_id/sign", put(crate::handler::labour::sign_contract))
        
        // Application management
        .route("/applications/:application_id/status", put(crate::handler::labour::update_application_status))
        
        // Escrow routes
        .route("/jobs/:job_id/escrow", get(crate::handler::labour::get_job_escrow))
        .route("/jobs/:job_id/escrow/release", post(crate::handler::labour::release_escrow_payment))
        .layer(middleware::from_fn(auth));

    // Combine labour routes
    let labour_routes = Router::new()
        .merge(public_labour_routes)
        .merge(protected_labour_routes);

    let notification_routes = crate::handler::notification_handler::notification_routes()
    .layer(middleware::from_fn(auth));

    // Create verification routes with auth middleware
    let verification_routes = verification_handler()
        .layer(middleware::from_fn(auth));

    let api_route = Router::new()
        .nest("/auth", auth_handler())
        .nest("/oauth", oauth_handler())
        .nest("/verification", verification_routes) // Use the created verification_routes
        .nest(
            "/users", 
            users_handler()
                .layer(middleware::from_fn(auth))
        )
        .nest("/wallet", wallet_routes)
        .nest("/labour", labour_routes)
        .nest("/notifications", notification_routes)
        .layer(TraceLayer::new_for_http())
        .layer(Extension(app_state));

    Router::new()
        .route("/health", get(health_check))
        .nest("/api", api_route)
}