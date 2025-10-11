use std::sync::Arc;

use axum::{middleware, Extension, Router};
use tower_http::trace::TraceLayer;

use crate::{
    handler::{
        auth::auth_handler, google_oauth::oauth_handler, labour::labour_handler, naira_wallet::naira_wallet_handler, users::users_handler
    }, middleware::auth, AppState};

pub fn create_router(app_state: Arc<AppState>) -> Router {
    let api_route = Router::new()
        .nest("/auth", auth_handler())
        .nest("/oauth", oauth_handler())
        .nest(
            "/users", 
            users_handler()
                .layer(middleware::from_fn(auth))
        )
        .nest(
            "/wallet",
            naira_wallet_handler()
                .layer(middleware::from_fn(auth))
        )
        .nest(
            "/labour",
            labour_handler()
        )
        .layer(TraceLayer::new_for_http())
        .layer(Extension(app_state));

    Router::new().nest("/api", api_route)
}