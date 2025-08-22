//8
mod models;
mod service;
mod config;
mod dtos;
mod error;
mod db;
mod utils;
mod middleware;
mod mail;
mod handler;
mod routes;

use std::sync::Arc;

use axum::http::{header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE}, HeaderValue, Method};
use config::Config;
use db::DBClient;
use dotenv::dotenv;
use routes::create_router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::filter::LevelFilter;

#[derive(Debug, Clone)]
pub struct AppState {
    pub env: Config,
    pub db_client: DBClient,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
    .with_max_level(LevelFilter::DEBUG)
    .init();

    dotenv().ok();

    let config = Config::init();

    let pool = match PgPoolOptions::new()
            .max_connections(10)
            .connect(&config.database_url)
            .await
    {
        Ok(pool) => {
            println!("✅Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("🔥 Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    let allowed_origins = vec![
    "https://verinest-frontend.vercel.app".parse::<HeaderValue>().unwrap(),
    "http://localhost:5173".parse::<HeaderValue>().unwrap(),
    "http://localhost:8000".parse::<HeaderValue>().unwrap(),
];

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE])
        .allow_credentials(true) //change to true
        .allow_methods([Method::GET, Method::POST,Method::PUT]);

    let db_client = DBClient::new(pool);
    let app_state = AppState {
        env: config.clone(),
        db_client,
    };

    let app = create_router(Arc::new(app_state.clone())).layer(cors.clone());

    
    println!(
        "{}",
        format!("🚀 Server is running on http://localhost:{}", config.port)
    );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", &config.port))
    .await
    .unwrap();

    axum::serve(listener, app).await.unwrap();
}
