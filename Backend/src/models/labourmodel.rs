use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::types::BigDecimal;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "worker_category", rename_all = "snake_case")]
pub enum WorkerCategory {
    // Construction & Building Trades
    Painter,
    Plumber,
    Electrician,
    Carpenter,
    Mason,
    Tiler,
    Roofer,
    Welder,
    SteelBender,
    ConcreteWorker,
    Bricklayer,
    FlooringSpecialist,
    Glazier,
    
    // Interior & Finishing
    InteriorDecorator,
    FurnitureMaker,
    Upholsterer,
    CurtainBlindInstaller,
    WallpaperSpecialist,
    
    // Landscaping & Outdoor
    Landscaper,
    Gardener,
    FenceInstaller,
    SwimmingPoolTechnician,
    OutdoorLightingSpecialist,
    
    // Specialized Real Estate Services
    RealEstateAgent,
    PropertyManager,
    FacilityManager,
    BuildingInspector,
    QuantitySurveyor,
    Architect,
    CivilEngineer,
    StructuralEngineer,
    
    // Maintenance & Repair
    Cleaner,
    Handyman,
    HVACTechnician,
    ElevatorTechnician,
    SecuritySystemInstaller,
    PestControlSpecialist,
    
    // Demolition & Site Work
    DemolitionExpert,
    SiteSupervisor,
    ConstructionLaborer,
    
    // Safety & Compliance
    SafetyOfficer,
    FireSafetyOfficer,
    
    Other,
}

impl WorkerCategory {
    pub fn to_str(&self) -> &str {
        match self {
            // Construction & Building Trades
            WorkerCategory::Painter => "painter",
            WorkerCategory::Plumber => "plumber",
            WorkerCategory::Electrician => "electrician",
            WorkerCategory::Carpenter => "carpenter",
            WorkerCategory::Mason => "mason",
            WorkerCategory::Tiler => "tiler",
            WorkerCategory::Roofer => "roofer",
            WorkerCategory::Welder => "welder",
            WorkerCategory::SteelBender => "steel_bender",
            WorkerCategory::ConcreteWorker => "concrete_worker",
            WorkerCategory::Bricklayer => "bricklayer",
            WorkerCategory::FlooringSpecialist => "flooring_specialist",
            WorkerCategory::Glazier => "glazier",
            
            // Interior & Finishing
            WorkerCategory::InteriorDecorator => "interior_decorator",
            WorkerCategory::FurnitureMaker => "furniture_maker",
            WorkerCategory::Upholsterer => "upholsterer",
            WorkerCategory::CurtainBlindInstaller => "curtain_blind_installer",
            WorkerCategory::WallpaperSpecialist => "wallpaper_specialist",
            
            // Landscaping & Outdoor
            WorkerCategory::Landscaper => "landscaper",
            WorkerCategory::Gardener => "gardener",
            WorkerCategory::FenceInstaller => "fence_installer",
            WorkerCategory::SwimmingPoolTechnician => "swimming_pool_technician",
            WorkerCategory::OutdoorLightingSpecialist => "outdoor_lighting_specialist",
            
            // Specialized Real Estate Services
            WorkerCategory::RealEstateAgent => "real_estate_agent",
            WorkerCategory::PropertyManager => "property_manager",
            WorkerCategory::FacilityManager => "facility_manager",
            WorkerCategory::BuildingInspector => "building_inspector",
            WorkerCategory::QuantitySurveyor => "quantity_surveyor",
            WorkerCategory::Architect => "architect",
            WorkerCategory::CivilEngineer => "civil_engineer",
            WorkerCategory::StructuralEngineer => "structural_engineer",
            
            // Maintenance & Repair
            WorkerCategory::Cleaner => "cleaner",
            WorkerCategory::Handyman => "handyman",
            WorkerCategory::HVACTechnician => "hvac_technician",
            WorkerCategory::ElevatorTechnician => "elevator_technician",
            WorkerCategory::SecuritySystemInstaller => "security_system_installer",
            WorkerCategory::PestControlSpecialist => "pest_control_specialist",
            
            // Demolition & Site Work
            WorkerCategory::DemolitionExpert => "demolition_expert",
            WorkerCategory::SiteSupervisor => "site_supervisor",
            WorkerCategory::ConstructionLaborer => "construction_laborer",
            
            // Safety & Compliance
            WorkerCategory::SafetyOfficer => "safety_officer",
            WorkerCategory::FireSafetyOfficer => "fire_safety_officer",
            
            WorkerCategory::Other => "other",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "job_status", rename_all = "snake_case")]
pub enum JobStatus {
    Open,
    InProgress,
    UnderReview,
    Completed,
    Disputed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "payment_status", rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Escrowed,
    PartiallyPaid,
    Completed,
    Refunded
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "dispute_status", rename_all = "snake_case")]
pub enum DisputeStatus {
    Open,
    UnderReview,
    Resolved,
    Escalated,
}

// In labourmodel.rs - Fix all structs to match database schema

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct WorkerProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category: WorkerCategory,
    pub experience_years: i32,
    pub description: String,
    pub hourly_rate: Option<BigDecimal>,
    pub daily_rate: Option<BigDecimal>,
    pub location_state: String,
    pub location_city: String,
    pub is_available: Option<bool>,  // Database has DEFAULT TRUE, can be NULL
    pub rating: Option<f32>,         // Database has DEFAULT 0.0, can be NULL
    pub completed_jobs: Option<i32>, // Database has DEFAULT 0, can be NULL
    pub created_at: Option<DateTime<Utc>>, // Database has DEFAULT NOW(), can be NULL
    pub updated_at: Option<DateTime<Utc>>, // Database has DEFAULT NOW(), can be NULL
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkerPortfolio {
    pub id: Uuid,
    pub worker_id: Uuid,
    pub title: String,
    pub description: String,
    pub image_url: String,
    pub project_date: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>, // Database has DEFAULT NOW(), can be NULL
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Job {
    pub id: Uuid,
    pub employer_id: Uuid,
    pub assigned_worker_id: Option<Uuid>,
    pub category: WorkerCategory,
    pub title: String,
    pub description: String,
    pub location_state: String,
    pub location_city: String,
    pub location_address: String,
    pub budget: BigDecimal,
    pub estimated_duration_days: i32,
    pub status: Option<JobStatus>,          // Database has DEFAULT 'open', can be NULL
    pub payment_status: Option<PaymentStatus>, // Database has DEFAULT 'pending', can be NULL
    pub escrow_amount: BigDecimal,
    pub platform_fee: BigDecimal,
    pub partial_payment_allowed: Option<bool>, // Database has DEFAULT FALSE, can be NULL
    pub partial_payment_percentage: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,  // Database has DEFAULT NOW(), can be NULL
    pub updated_at: Option<DateTime<Utc>>,  // Database has DEFAULT NOW(), can be NULL
    pub deadline: Option<DateTime<Utc>>
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobApplication {
    pub id: Uuid,
    pub job_id: Uuid,
    pub worker_id: Uuid,
    pub proposed_rate: BigDecimal,
    pub estimated_completion: i32,
    pub cover_letter: String,
    pub status: Option<String>,             // Database has DEFAULT 'applied', can be NULL
    pub created_at: Option<DateTime<Utc>>,  // Database has DEFAULT NOW(), can be NULL
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobContract {
    pub id: Uuid,
    pub job_id: Uuid,
    pub employer_id: Uuid,
    pub worker_id: Uuid,
    pub agreed_rate: BigDecimal,
    pub agreed_timeline: i32,
    pub terms: String,
    pub signed_by_employer: Option<bool>,   // Database has DEFAULT FALSE, can be NULL
    pub signed_by_worker: Option<bool>,     // Database has DEFAULT FALSE, can be NULL
    pub contract_date: Option<DateTime<Utc>>, // Database has DEFAULT NOW(), can be NULL
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EscrowTransaction {
    pub id: Uuid,
    pub job_id: Uuid,
    pub employer_id: Uuid,
    pub worker_id: Uuid,
    pub amount: BigDecimal,
    pub platform_fee: BigDecimal,
    pub status: Option<PaymentStatus>,      // Database has DEFAULT 'pending', can be NULL
    pub transaction_hash: Option<String>,
    pub created_at: Option<DateTime<Utc>>,  // Database has DEFAULT NOW(), can be NULL
    pub released_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobProgress {
    pub id: Uuid,
    pub job_id: Uuid,
    pub worker_id: Uuid,
    pub progress_percentage: i32,
    pub description: String,
    pub image_urls: Option<Vec<String>>,
    pub submitted_at: Option<DateTime<Utc>>, // Database has DEFAULT NOW(), can be NULL
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobReview {
    pub id: Uuid,
    pub job_id: Uuid,
    pub reviewer_id: Uuid,
    pub reviewee_id: Uuid,
    pub rating: i32,
    pub comment: String,
    pub created_at: Option<DateTime<Utc>>,  // Database has DEFAULT NOW(), can be NULL
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Dispute {
    pub id: Uuid,
    pub job_id: Uuid,
    pub raised_by: Uuid,
    pub against: Uuid,
    pub reason: String,
    pub description: String,
    pub evidence_urls: Option<Vec<String>>,
    pub status: Option<DisputeStatus>,      
    pub assigned_verifier: Option<Uuid>,
    pub resolution: Option<String>,
    pub created_at: Option<DateTime<Utc>>,  
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct VerificationTask {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub verifier_id: Uuid,
    pub status: Option<String>,             // Database has DEFAULT 'pending', can be NULL
    pub notes: Option<String>,
    pub decision: Option<String>,
    pub created_at: Option<DateTime<Utc>>,  // Database has DEFAULT NOW(), can be NULL
    pub completed_at: Option<DateTime<Utc>>,
}