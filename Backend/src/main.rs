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
mod recommendation_models;
mod services;

use std::sync::Arc;

use axum::http::{header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE}, HeaderValue, Method};
use config::Config;
use crate::db::db::DBClient;
use dotenv::dotenv;
use routes::create_router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::filter::LevelFilter;
use crate::service::subscriptions::start_vendor_expiry_checker;
use crate::services::behavior_tracking::BehaviorTracker;

// Import the services
use service::{
    labour_service::LabourService,
    escrow_service::EscrowService,
    dispute_service::DisputeService,
    trust_service::TrustService,
    notification_service::NotificationService,
    audit_service::AuditService,
    matching_service::MatchingService,
    verification_service::VerificationService,
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub env: Config,
    pub db_client: Arc<DBClient>,
    // Services
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

    // Connect to PostgreSQL
    let pool = match PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .connect(&config.database_url)
            .await
    {
        Ok(pool) => {
            println!("✅ Connection to the database is successful!");
            
            // Log connection pool stats for monitoring
            println!("📊 Connection Pool Stats:");
            println!("   - Max connections: 20");
            println!("   - Min connections: 5");
            
            // Store max connections for monitoring
            let max_connections = 20;
            
            // Start a background task to monitor pool health
            let pool_for_monitoring = pool.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    let size = pool_for_monitoring.size();
                    let idle = pool_for_monitoring.num_idle();
                    tracing::debug!("🔍 Pool Status - Active: {}, Idle: {}, Total: {}", 
                        size - idle as u32, idle, size);
                    
                    // Warning if pool is getting full
                    if size >= max_connections * 8 / 10 {
                        tracing::warn!("⚠️  Connection pool at 80% capacity! Consider increasing max_connections");
                    }
                }
            });
            
            pool
        }
        Err(err) => {
            println!("🔥 Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    // Initialize DBClient with optional Redis
    let db_client = if let Some(ref redis_url) = config.redis_url {
        match DBClient::with_redis(pool.clone(), redis_url).await {
            Ok(client) => {
                if client.is_redis_available() {
                    println!("✅ Redis caching is ACTIVE - Performance boosted! 🚀");
                } else {
                    println!("⚠️  Redis connection failed - Running without cache");
                }
                client
            }
            Err(e) => {
                println!("⚠️  Redis initialization error: {} - Running without cache", e);
                DBClient::new(pool)
            }
        }
    } else {
        println!("ℹ️  Redis not configured - Running without cache (set REDIS_URL to enable)");
        DBClient::new(pool)
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

    let app_state = Arc::new(AppState::new(db_client, config.clone()));

    let app = create_router(app_state.clone()).layer(cors);

    println!(
        "🚀 Server is running on http://localhost:{}",
        config.port
    );
    println!("📊 Cache status: {}", app_state.db_client.cache_status());

    // Start background jobs
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        service::background_jobs::start_auto_confirmation_job(app_state_clone).await;
    });

    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        service::background_jobs::start_service_expiry_job(app_state_clone).await;
    });

    // Start vendor subscription expiry checker
    tokio::spawn(start_vendor_expiry_checker(app_state.clone()));

    // Start recommendation behavior tracker (consumes reco:events_list)
    let tracker_db_client = app_state.db_client.clone();
    let tracker = BehaviorTracker::new(tracker_db_client, "reco:events_list");
    tokio::spawn(async move {
        // Shutdown when the process receives CTRL+C
        tracker.run_forever(async { let _ = tokio::signal::ctrl_c().await; }).await;
    });

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", &config.port))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}