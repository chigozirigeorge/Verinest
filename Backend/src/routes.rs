// routes.rs - Updated with separate verification route
use std::sync::Arc;

use axum::{middleware, routing::{get, post, put}, Extension, Json, Router};
use tower_http::trace::TraceLayer;
use serde_json::json;

use crate::{
    handler::{
        auth::auth_handler, 
        google_oauth::oauth_handler, 
        labour::labour_handler, 
        naira_wallet::{
            naira_wallet_handler, 
            public_verify_deposit,  // Add this import
            paystack_webhook,
            flutterwave_webhook
        }, 
        users::users_handler
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
            .post(crate::handler::naira_wallet::verify_deposit))  // Both GET and POST handlers
        .route("/webhook/paystack", post(paystack_webhook))
        .route("/webhook/flutterwave", post(flutterwave_webhook));

    // Combine wallet routes
    let wallet_routes = Router::new()
        .merge(protected_wallet_routes)
        .merge(public_wallet_routes);

    let api_route = Router::new()
        .nest("/auth", auth_handler())
        .nest("/oauth", oauth_handler())
        .nest(
            "/users", 
            users_handler()
                .layer(middleware::from_fn(auth))
        )
        .nest("/wallet", wallet_routes)
        .nest(
            "/labour",
            labour_handler()
                .layer(middleware::from_fn(auth))
        )
        .layer(TraceLayer::new_for_http())
        .layer(Extension(app_state));

    Router::new()
        .route("/health", get(health_check))
        .nest("/api", api_route)
}