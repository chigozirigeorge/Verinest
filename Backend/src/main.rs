// main.rs (Updated with Service Integration)
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
use crate::db::db::DBClient;
use dotenv::dotenv;
use routes::create_router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::filter::LevelFilter;

// Import the services we created
use service::{
    labour_service::LabourService,
    escrow_service::EscrowService,
    dispute_service::DisputeService,
    trust_service::TrustService,
    notification_service::NotificationService,
    audit_service::AuditService,
    matching_service::MatchingService,
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub env: Config,
    pub db_client: Arc<DBClient>,
    // Add all the services
    pub labour_service: Arc<LabourService>,
    pub escrow_service: Arc<EscrowService>,
    pub dispute_service: Arc<DisputeService>,
    pub trust_service: Arc<TrustService>,
    pub notification_service: Arc<NotificationService>,
    pub audit_service: Arc<AuditService>,
    pub matching_service: Arc<MatchingService>,
    pub verification_service: Arc<VerificationService>,
}

impl AppState {
    pub fn new(db_client: DBClient, config: Config) -> Self {
        let db_client_arc = Arc::new(db_client);
        
        // Initialize all services
        let trust_service = Arc::new(TrustService::new(db_client_arc.clone()));
        let notification_service = Arc::new(NotificationService::new(db_client_arc.clone()));
        let audit_service = Arc::new(AuditService::new(db_client_arc.clone()));
        let matching_service = Arc::new(MatchingService::new(db_client_arc.clone()));
        let escrow_service = Arc::new(EscrowService::new(db_client_arc.clone()));
        let verification_service = Arc::new(VerificationService::new(db_client_arc.clone()));

        let verification_service_clone = verification_service.clone();
        tokio::spawn(async move {
            verification_service_clone.start_cleanup_task().await;
        });

        let labour_service = Arc::new(LabourService::new(
            db_client_arc.clone(),
            escrow_service.clone(),
            trust_service.clone(),
            notification_service.clone(),
            audit_service.clone(),
        ));

        let dispute_service = Arc::new(DisputeService::new(
            db_client_arc.clone(),
            escrow_service.clone(),
            notification_service.clone(),
            audit_service.clone(),
            trust_service.clone(),
        ));

        Self {
            env: config,
            db_client: db_client_arc,
            labour_service,
            escrow_service,
            dispute_service,
            trust_service,
            notification_service,
            audit_service,
            matching_service,
            verification_service,
        }
    }
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
    "https://verinestorg.vercel.app".parse::<HeaderValue>().unwrap(),
    "https://verinest.up.railway.app".parse::<HeaderValue>().unwrap(),
    "http://localhost:5173".parse::<HeaderValue>().unwrap(),
    "http://localhost:8000".parse::<HeaderValue>().unwrap(),
];

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE])
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH]);

    let db_client = DBClient::new(pool);
    let app_state = AppState::new(db_client, config.clone());

    let app = create_router(Arc::new(app_state)).layer(cors);

    println!(
        "🚀 Server is running on http://localhost:{}",
        config.port
    );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", &config.port))
    .await
    .unwrap();

    axum::serve(listener, app).await.unwrap();
}