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
    
    // Add from_str method for conversion
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "painter" => Some(WorkerCategory::Painter),
            "plumber" => Some(WorkerCategory::Plumber),
            "electrician" => Some(WorkerCategory::Electrician),
            "carpenter" => Some(WorkerCategory::Carpenter),
            "mason" => Some(WorkerCategory::Mason),
            "tiler" => Some(WorkerCategory::Tiler),
            "roofer" => Some(WorkerCategory::Roofer),
            "welder" => Some(WorkerCategory::Welder),
            "steel_bender" => Some(WorkerCategory::SteelBender),
            "concrete_worker" => Some(WorkerCategory::ConcreteWorker),
            "bricklayer" => Some(WorkerCategory::Bricklayer),
            "flooring_specialist" => Some(WorkerCategory::FlooringSpecialist),
            "glazier" => Some(WorkerCategory::Glazier),
            "interior_decorator" => Some(WorkerCategory::InteriorDecorator),
            "furniture_maker" => Some(WorkerCategory::FurnitureMaker),
            "upholsterer" => Some(WorkerCategory::Upholsterer),
            "curtain_blind_installer" => Some(WorkerCategory::CurtainBlindInstaller),
            "wallpaper_specialist" => Some(WorkerCategory::WallpaperSpecialist),
            "landscaper" => Some(WorkerCategory::Landscaper),
            "gardener" => Some(WorkerCategory::Gardener),
            "fence_installer" => Some(WorkerCategory::FenceInstaller),
            "swimming_pool_technician" => Some(WorkerCategory::SwimmingPoolTechnician),
            "outdoor_lighting_specialist" => Some(WorkerCategory::OutdoorLightingSpecialist),
            "real_estate_agent" => Some(WorkerCategory::RealEstateAgent),
            "property_manager" => Some(WorkerCategory::PropertyManager),
            "facility_manager" => Some(WorkerCategory::FacilityManager),
            "building_inspector" => Some(WorkerCategory::BuildingInspector),
            "quantity_surveyor" => Some(WorkerCategory::QuantitySurveyor),
            "architect" => Some(WorkerCategory::Architect),
            "civil_engineer" => Some(WorkerCategory::CivilEngineer),
            "structural_engineer" => Some(WorkerCategory::StructuralEngineer),
            "cleaner" => Some(WorkerCategory::Cleaner),
            "handyman" => Some(WorkerCategory::Handyman),
            "hvac_technician" => Some(WorkerCategory::HVACTechnician),
            "elevator_technician" => Some(WorkerCategory::ElevatorTechnician),
            "security_system_installer" => Some(WorkerCategory::SecuritySystemInstaller),
            "pest_control_specialist" => Some(WorkerCategory::PestControlSpecialist),
            "demolition_expert" => Some(WorkerCategory::DemolitionExpert),
            "site_supervisor" => Some(WorkerCategory::SiteSupervisor),
            "construction_laborer" => Some(WorkerCategory::ConstructionLaborer),
            "safety_officer" => Some(WorkerCategory::SafetyOfficer),
            "fire_safety_officer" => Some(WorkerCategory::FireSafetyOfficer),
            "other" => Some(WorkerCategory::Other),
            _ => None,
        }
    }
}

// Implement Default for WorkerCategory
impl Default for WorkerCategory {
    fn default() -> Self {
        WorkerCategory::Other
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

impl JobStatus {
    pub fn to_str(&self) -> &str {
        match self {
            JobStatus::Open => "open",
            JobStatus::InProgress => "in_progress",
            JobStatus::UnderReview => "under_review",
            JobStatus::Completed => "completed",
            JobStatus::Disputed => "disputed",
            JobStatus::Cancelled => "cancelled",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(JobStatus::Open),
            "in_progress" => Some(JobStatus::InProgress),
            "under_review" => Some(JobStatus::UnderReview),
            "completed" => Some(JobStatus::Completed),
            "disputed" => Some(JobStatus::Disputed),
            "cancelled" => Some(JobStatus::Cancelled),
            _ => None,
        }
    }
}

impl Default for JobStatus {
    fn default() -> Self {
        JobStatus::Open
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "payment_status", rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Escrowed,
    Funded,
    PartiallyPaid,
    Completed,
    Refunded,
}

impl PaymentStatus {
    pub fn to_str(&self) -> &str {
        match self {
            PaymentStatus::Pending => "pending",
            PaymentStatus::Escrowed => "escrowed",
            PaymentStatus::Funded => "funded",
            PaymentStatus::PartiallyPaid => "partially_paid",
            PaymentStatus::Completed => "completed",
            PaymentStatus::Refunded => "refunded",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(PaymentStatus::Pending),
            "escrowed" => Some(PaymentStatus::Escrowed),
            "funded" => Some(PaymentStatus::Funded),
            "partially_paid" => Some(PaymentStatus::PartiallyPaid),
            "completed" => Some(PaymentStatus::Completed),
            "refunded" => Some(PaymentStatus::Refunded),
            _ => None,
        }
    }
}

impl Default for PaymentStatus {
    fn default() -> Self {
        PaymentStatus::Pending
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "dispute_status", rename_all = "snake_case")]
pub enum DisputeStatus {
    Open,
    UnderReview,
    Resolved,
    Escalated,
}

impl DisputeStatus {
    pub fn to_str(&self) -> &str {
        match self {
            DisputeStatus::Open => "open",
            DisputeStatus::UnderReview => "under_review",
            DisputeStatus::Resolved => "resolved",
            DisputeStatus::Escalated => "escalated",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(DisputeStatus::Open),
            "under_review" => Some(DisputeStatus::UnderReview),
            "resolved" => Some(DisputeStatus::Resolved),
            "escalated" => Some(DisputeStatus::Escalated),
            _ => None,
        }
    }
}

impl Default for DisputeStatus {
    fn default() -> Self {
        DisputeStatus::Open
    }
}

// Add ApplicationStatus enum
#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "application_status", rename_all = "snake_case")]
pub enum ApplicationStatus {
    Applied,
    Reviewed,
    Accepted,
    Rejected,
    Withdrawn,
}

impl ApplicationStatus {
    pub fn to_str(&self) -> &str {
        match self {
            ApplicationStatus::Applied => "applied",
            ApplicationStatus::Reviewed => "reviewed",
            ApplicationStatus::Accepted => "accepted",
            ApplicationStatus::Rejected => "rejected",
            ApplicationStatus::Withdrawn => "withdrawn",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "applied" => Some(ApplicationStatus::Applied),
            "reviewed" => Some(ApplicationStatus::Reviewed),
            "accepted" => Some(ApplicationStatus::Accepted),
            "rejected" => Some(ApplicationStatus::Rejected),
            "withdrawn" => Some(ApplicationStatus::Withdrawn),
            _ => None,
        }
    }
}

impl Default for ApplicationStatus {
    fn default() -> Self {
        ApplicationStatus::Applied
    }
}

// Add ContractStatus enum
#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq)]
#[sqlx(type_name = "contract_status", rename_all = "snake_case")]
pub enum ContractStatus {
    Draft,
    Pending,
    Active,
    Completed,
    Cancelled,
    Disputed,
}

impl ContractStatus {
    pub fn to_str(&self) -> &str {
        match self {
            ContractStatus::Draft => "draft",
            ContractStatus::Pending => "pending",
            ContractStatus::Active => "active",
            ContractStatus::Completed => "completed",
            ContractStatus::Cancelled => "cancelled",
            ContractStatus::Disputed => "disputed",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(ContractStatus::Draft),
            "pending" => Some(ContractStatus::Pending),
            "active" => Some(ContractStatus::Active),
            "completed" => Some(ContractStatus::Completed),
            "cancelled" => Some(ContractStatus::Cancelled),
            "disputed" => Some(ContractStatus::Disputed),
            _ => None,
        }
    }
}

impl Default for ContractStatus {
    fn default() -> Self {
        ContractStatus::Draft
    }
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
    pub is_available: Option<bool>,
    pub rating: Option<f32>,
    pub completed_jobs: Option<i64>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkerPortfolio {
    pub id: Uuid,
    pub worker_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub image_url: String,
    pub project_date: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
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
    pub status: Option<JobStatus>,
    pub payment_status: Option<PaymentStatus>,
    pub escrow_amount: BigDecimal,
    pub platform_fee: BigDecimal,
    pub partial_payment_allowed: Option<bool>,
    pub partial_payment_percentage: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobApplication {
    pub id: Uuid,
    pub job_id: Uuid,
    pub worker_id: Uuid,
    pub proposed_rate: BigDecimal,
    pub estimated_completion: i32,
    pub cover_letter: String,
    pub status: Option<ApplicationStatus>,
    pub created_at: Option<DateTime<Utc>>,
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
    pub signed_by_employer: Option<bool>,
    pub signed_by_worker: Option<bool>,
    pub status: Option<ContractStatus>, 
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub contract_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EscrowTransaction {
    pub id: Uuid,
    pub job_id: Uuid,
    pub employer_id: Uuid,
    pub worker_id: Option<Uuid>,
    pub amount: BigDecimal,
    pub platform_fee: BigDecimal,
    pub status: Option<PaymentStatus>,
    pub transaction_hash: Option<String>,
    pub wallet_hold_id: Option<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
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
    pub submitted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobReview {
    pub id: Uuid,
    pub job_id: Uuid,
    pub reviewer_id: Uuid,
    pub reviewee_id: Uuid,
    pub rating: i32,
    pub comment: String,
    pub created_at: Option<DateTime<Utc>>,
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
    pub status: Option<String>,
    pub notes: Option<String>,
    pub decision: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AdminDisputeVerification {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub admin_id: Uuid,
    pub verifier_resolution_id: Uuid,
    pub admin_decision: String,
    pub admin_notes: Option<String>,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PendingDisputeResolution {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub verifier_id: Uuid,
    pub resolution: String,
    pub decision: String,
    pub payment_percentage: Option<f64>,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub admin_verified_at: Option<DateTime<Utc>>,
}

// Add these helper functions to handle Option enums safely
pub trait SafeEnumConversion {
    fn safe_unwrap(&self) -> String;
}

impl SafeEnumConversion for Option<JobStatus> {
    fn safe_unwrap(&self) -> String {
        self.map(|s| s.to_str().to_string())
            .unwrap_or_else(|| "open".to_string())
    }
}

impl SafeEnumConversion for Option<PaymentStatus> {
    fn safe_unwrap(&self) -> String {
        self.map(|s| s.to_str().to_string())
            .unwrap_or_else(|| "pending".to_string())
    }
}

impl SafeEnumConversion for Option<ApplicationStatus> {
    fn safe_unwrap(&self) -> String {
        self.map(|s| s.to_str().to_string())
            .unwrap_or_else(|| "applied".to_string())
    }
}

impl SafeEnumConversion for Option<ContractStatus> {
    fn safe_unwrap(&self) -> String {
        self.map(|s| s.to_str().to_string())
            .unwrap_or_else(|| "draft".to_string())
    }
}

impl SafeEnumConversion for Option<DisputeStatus> {
    fn safe_unwrap(&self) -> String {
        self.map(|s| s.to_str().to_string())
            .unwrap_or_else(|| "open".to_string())
    }
}