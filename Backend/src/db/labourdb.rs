// db/labourdb.rs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::{Error, types::BigDecimal, Row};
use num_traits::ToPrimitive;
use sqlx::Error as SqlxError;

use super::db::DBClient;
use crate::{models::labourmodel::*};
use crate::models::walletmodels::naira_to_kobo;

#[async_trait]
pub trait LaborExt {
    async fn create_worker_profile(
        &self,
        user_id: Uuid,
        category: WorkerCategory,
        experience_years: i32,
        description: String,
        hourly_rate: Option<f64>,
        daily_rate: Option<f64>,
        location_state: String,
        location_city: String,
    ) -> Result<WorkerProfile, Error>;

    async fn get_worker_profile(
        &self,
        user_id: Uuid
    ) -> Result<WorkerProfile, Error>;

    async fn get_worker_profile_by_id(
    &self,
    profile_id: Uuid
) -> Result<WorkerProfile, Error>;

    async fn update_worker_availability(
        &self,
        worker_id: Uuid,
        is_available: bool,
    ) -> Result<WorkerProfile, Error>;

    async fn get_workers_by_location_and_category(
        &self,
        state: &str,
        category: WorkerCategory,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkerProfile>, Error>;

    async fn get_workers_by_state_only(
        &self,
        state: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkerProfile>, Error>;

    //Portfolio management
    async fn add_portfolio_item(
        &self,
        worker_id: Uuid,
        title: String,
        description: String,
        image_url: String,
        project_date: Option<DateTime<Utc>>
    ) -> Result<WorkerPortfolio, Error>;

    async fn get_worker_portfolio(
        &self,
        worker_id: Uuid,
    ) -> Result<Vec<WorkerPortfolio>, Error>;

    //Job management
    async fn create_job(
        &self,
        employer_id: Uuid,
        category: WorkerCategory,
        title: String,
        description: String,
        location_state: String,
        location_city: String,
        location_address: String,
        budget: f64,
        estimated_duration_days: i32,
        partial_payment_allowed: bool,
        partial_payment_percentage: Option<i32>,
        deadline: Option<DateTime<Utc>>,
    ) -> Result<Job, Error>;

    async fn get_jobs_by_location_and_category(
        &self,
        state: &str,
        category: WorkerCategory,
        status: JobStatus,
    ) -> Result<Vec<Job>, Error>;

    async fn get_open_jobs(&self) -> Result<Vec<Job>, Error>;

    async fn get_jobs_by_location(
        &self,
        state: &str,
        status: JobStatus,
    ) -> Result<Vec<Job>, Error>;

    async fn get_jobs_by_category(
        &self,
        category: WorkerCategory,
        status: JobStatus,
    ) -> Result<Vec<Job>, Error>;

    async fn get_job_by_id(&self, job_id: Uuid) -> Result<Option<Job>, Error>;

    async fn update_job_status(
        &self,
        job_id: Uuid,
        status: JobStatus,
    ) -> Result<Job, Error>;

//     async fn assign_worker_to_job(
//     &self,
//     job_id: Uuid,
//     worker_id: Uuid
// ) -> Result<(Job, EscrowTransaction), Error>;

async fn assign_worker_to_job(
    &self,
    job_id: Uuid,
    employer_user_id: Uuid,
    worker_profile_id: Uuid, // This should be profile_id (from worker_profiles.id)
) -> Result<Job, Error>;

    //Job application
    async fn create_job_application(
        &self,
        job_id: Uuid,
        worker_id: Uuid,
        proposed_rate: f64,
        estimated_completion: i32,
        cover_letter: String,
    ) -> Result<JobApplication, Error>;

    async fn get_job_applications(
        &self,
        job_id: Uuid
    ) -> Result<Vec<JobApplication>, Error>;

    async fn update_application_status(
        &self,
        appliaction_id: Uuid,
        status: String,
    ) -> Result<JobApplication, Error>;

    //Contract Management
    async fn create_job_contract(
        &self,
        job_id: Uuid,
        employer_id: Uuid,
        worker_id: Uuid,
        agreed_rate: f64,
        agreed_timeline: i32,
        terms: String,
    ) -> Result<JobContract, Error>;

    async fn sign_contract(
        &self,
        contract_id: Uuid,
        signer_role: String,    //employer or worker
    ) -> Result<JobContract, Error>;

    async fn sign_contract_tx(
        &self,
        contract_id: Uuid,
        signer_role: String,    //employer or worker
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<JobContract, Error>;

    //Escrow management
    async fn create_escrow_transaction(
        &self,
        job_id: Uuid,
        employer_id: Uuid,
        worker_id: Option<Uuid>,
        amount: f64,
        platform_fee: f64, 
    ) -> Result<EscrowTransaction, Error>;

    // Atomically create an escrow transaction and place a wallet hold for the employer.
    // This inserts the escrow row, creates a wallet_holds entry and persists the hold id
    // on the escrow row in a single DB transaction to avoid transient inconsistencies.
    async fn create_escrow_with_hold(
        &self,
        job_id: Uuid,
        employer_id: Uuid,
        amount: f64,
        platform_fee: f64,
    ) -> Result<EscrowTransaction, Error>;


    // Persist wallet_hold_id for an escrow
    async fn update_escrow_wallet_hold_id(&self, escrow_id: Uuid, wallet_hold_id: Uuid) -> Result<EscrowTransaction, Error>;

    // Update escrow worker when worker signs the contract
    async fn update_escrow_worker(&self, escrow_id: Uuid, worker_id: Uuid) -> Result<EscrowTransaction, Error>;

    async fn update_escrow_status(
        &self,
        escrow_id: Uuid,
        status: PaymentStatus,
        transaction_hash: Option<String>,
    ) -> Result<EscrowTransaction, Error>;

    async fn release_escrow_payment(
        &self,
        escrow_id: Uuid,
        release_percentage: f64,
    ) -> Result<EscrowTransaction, Error>;

    async fn get_escrow_transaction(
        &self,
        escrow_id: Uuid,
    ) -> Result<Option<EscrowTransaction>, Error>;


    //Job Progress Tracking
    async fn submit_job_progress(
        &self,
        job_id: Uuid,
        worker_id: Uuid,
        progress_percentage: i32,
        description: String,
        image_urls: Vec<String>,
    ) -> Result<JobProgress, Error>;

    async fn get_job_progress(
        &self,
        job_id: Uuid,
    ) -> Result<Vec<JobProgress>, Error>;

    async fn create_dispute(
        &self,
        job_id: Uuid,
        raised_by: Uuid,
        against: Uuid,
        reason: String,
        description: String,
        evidence_urls: Vec<String>,
    ) -> Result<Dispute, Error>;

    async fn get_dispute_by_id(
        &self, 
        dispute_id: Uuid
    ) -> Result<Option<Dispute>, Error>;

    async fn assign_verifer_to_dispute(
        &self,
        dispute_id: Uuid,
        verifier_id: Uuid
    ) -> Result<Dispute, Error>;

    async fn resolve_dispute(
        &self,
        dispute_id: Uuid,
        resolution: String,
        decision: String,
    ) -> Result<Dispute, Error>;

    async fn get_pending_verifications_f(
        &self,
        verifier_id: Uuid,
    ) -> Result<Vec<Dispute>, Error>;

    // High-value dispute verification methods
    async fn get_admin_verification_for_dispute(
        &self,
        dispute_id: Uuid,
    ) -> Result<Option<AdminDisputeVerification>, Error>;

    async fn create_pending_dispute_resolution(
        &self,
        dispute_id: Uuid,
        verifier_id: Uuid,
        resolution: String,
        decision: String,
        payment_percentage: Option<f64>,
    ) -> Result<PendingDisputeResolution, Error>;

    async fn assign_admin_to_dispute_verification(
        &self,
        dispute_id: Uuid,
        admin_id: Uuid,
    ) -> Result<(), Error>;

    async fn update_dispute_status(
        &self,
        dispute_id: Uuid,
        status: DisputeStatus,
    ) -> Result<Dispute, Error>;

    //Review Sysytem
    async fn create_job_review(
        &self,
        job_id: Uuid,
        reviewer_id: Uuid,
        reviewee_id: Uuid,
        rating: i32,
        comment: String
    ) -> Result<JobReview, Error>;

    async fn get_worker_reviews(
        &self,
        worker_id: Uuid,
    ) -> Result<Vec<JobReview>, Error>;

    async fn update_worker_rating(
        &self,
        worker_id: Uuid,
    ) -> Result<(), Error>;

    //Trust Points for labor
    async fn award_job_completion_points(
        &self,
        worker_id: Uuid,
        employer_id: Uuid,
        job_rating: i32,
        completed_on_time: bool,
    ) -> Result<(), Error>;

    async fn get_jobs_by_employer_and_status(
        &self,
        employer_id: Uuid,
        status: JobStatus,
    ) -> Result<Vec<Job>, Error>;

    async fn get_escrow_by_job_id(
        &self, 
        job_id: Uuid
    ) -> Result<Option<EscrowTransaction>, Error>;

    async fn get_escrow_by_id(
        &self, 
        escrow_id: Uuid
    ) -> Result<Option<EscrowTransaction>, Error>;

    async fn get_employer_jobs(
        &self, 
        employer_id: Uuid
    ) -> Result<Vec<Job>, Error>;
    
    async fn get_employer_active_contracts(
        &self, 
        employer_id: Uuid
    ) -> Result<Vec<JobContract>, Error>;
    
    async fn get_employer_pending_applications(
        &self, 
        employer_id: Uuid
    ) -> Result<Vec<JobApplication>, Error>;
    
    async fn get_job_application_by_id(
        &self, 
        application_id: Uuid
    ) -> Result<Option<JobApplication>, Error>;
    
    async fn get_worker_active_jobs(
        &self, 
        worker_id: Uuid
    ) -> Result<Vec<Job>, Error>;
    
    async fn get_worker_pending_applications(
        &self, 
        worker_id: Uuid
    ) -> Result<Vec<JobApplication>, Error>;

    async fn get_portfolio_item_by_id(
        &self,
        item_id: Uuid,
    ) -> Result<Option<WorkerPortfolio>, Error>;

    async fn delete_portfolio_item(
        &self,
        item_id: Uuid,
    ) -> Result<(), Error>;
}


#[async_trait]
impl LaborExt for DBClient {
    async fn create_worker_profile(
        &self,
        user_id: Uuid,
        category: WorkerCategory,
        experience_years: i32,
        description: String,
        hourly_rate: Option<f64>,
        daily_rate: Option<f64>,
        location_state: String,
        location_city: String,
    ) -> Result<WorkerProfile, Error> {
        // Use BigDecimal::try_from instead of BigDecimal::from
        let hourly_rate_bd = hourly_rate.and_then(|rate| {
            BigDecimal::try_from(rate).ok()
        });
        let daily_rate_bd = daily_rate.and_then(|rate| {
            BigDecimal::try_from(rate).ok()
        });

        sqlx::query_as::<_, WorkerProfile>(
            r#"
            INSERT INTO worker_profiles
            (user_id, category, experience_years, description, hourly_rate, daily_rate, location_state, location_city)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING 
                id, user_id, 
                category, 
                experience_years, description, 
                hourly_rate, daily_rate, 
                location_state, location_city, 
                is_available, rating, completed_jobs::BIGINT as completed_jobs, 
                created_at, updated_at
            "#
        )
        .bind(user_id)
        .bind(category)
        .bind(experience_years)
        .bind(description)
        .bind(hourly_rate_bd)
        .bind(daily_rate_bd)
        .bind(location_state)
        .bind(location_city)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_worker_profile(
        &self,
        user_id: Uuid
    ) -> Result<WorkerProfile, Error> {
        let profile = sqlx::query_as::<_, WorkerProfile>(
            r#"
            SELECT 
                id, user_id, 
                category, 
                experience_years, description, 
                hourly_rate, daily_rate, 
                location_state, location_city, 
                is_available, rating, completed_jobs::BIGINT as completed_jobs, 
                created_at, updated_at
            FROM worker_profiles
            WHERE user_id = $1
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        
        // Convert Option to Result
        profile.ok_or_else(|| sqlx::Error::RowNotFound)
    }

    async fn get_worker_profile_by_id(
    &self,
    profile_id: Uuid
) -> Result<WorkerProfile, Error> {
    let profile = sqlx::query_as::<_, WorkerProfile>(
        r#"
        SELECT 
            id, user_id, 
            category, 
            experience_years, description, 
            hourly_rate, daily_rate, 
            location_state, location_city, 
            is_available, rating, completed_jobs::BIGINT as completed_jobs, 
            created_at, updated_at
        FROM worker_profiles
        WHERE id = $1
        "#
    )
    .bind(profile_id)
    .fetch_optional(&self.pool)
    .await?;
    
    profile.ok_or_else(|| sqlx::Error::RowNotFound)
}

    async fn update_worker_availability(
        &self,
        worker_id: Uuid,
        is_available: bool,
    ) -> Result<WorkerProfile, Error> {
        sqlx::query_as::<_, WorkerProfile>(
            r#"
            UPDATE worker_profiles
            SET is_available = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, user_id, 
                category, 
                experience_years, description, 
                hourly_rate, daily_rate, 
                location_state, location_city, 
                is_available, rating, completed_jobs::BIGINT as completed_jobs, 
                created_at, updated_at
            "#
        )
        .bind(worker_id)
        .bind(is_available)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_workers_by_location_and_category(
        &self,
        state: &str,
        category: WorkerCategory,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkerProfile>, Error> {
        sqlx::query_as::<_, WorkerProfile>(
            r#"
            SELECT id, user_id, category, experience_years, description, hourly_rate, 
            daily_rate, location_state, location_city, is_available, rating, completed_jobs::BIGINT as completed_jobs, created_at, updated_at
            FROM worker_profiles
            WHERE location_state = $1 AND category = $2 AND is_available = true
            ORDER BY rating DESC, completed_jobs::BIGINT DESC
            LIMIT $3 OFFSET $4
            "#
        )
        .bind(state)
        .bind(category)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_workers_by_state_only(
        &self,
        state: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkerProfile>, Error> {
        sqlx::query_as::<_, WorkerProfile>(
            r#"
            SELECT id, user_id, category, experience_years, description, hourly_rate, 
            daily_rate, location_state, location_city, is_available, rating, completed_jobs::BIGINT as completed_jobs, created_at, updated_at
            FROM worker_profiles
            WHERE location_state = $1 AND is_available = true
            ORDER BY rating DESC, completed_jobs::BIGINT DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(state)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    async fn add_portfolio_item(
        &self,
        worker_id: Uuid,
        title: String,
        description: String,
        image_url: String,
        project_date: Option<DateTime<Utc>>
    ) -> Result<WorkerPortfolio, Error> {
        sqlx::query_as::<_, WorkerPortfolio>(
            r#"
            INSERT INTO worker_portfolios 
            (worker_id, title, description, image_url, project_date)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, worker_id, title, description, image_url, project_date, created_at
            "#
        )
        .bind(worker_id)
        .bind(title)
        .bind(description)
        .bind(image_url)
        .bind(project_date)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_worker_portfolio(
        &self,
        worker_id: Uuid,
    ) -> Result<Vec<WorkerPortfolio>, Error> {
        sqlx::query_as::<_, WorkerPortfolio>(
            r#"
            SELECT id, worker_id, title, description, image_url, project_date, created_at
            FROM worker_portfolios 
            WHERE worker_id = $1
            ORDER BY project_date DESC NULLS LAST, created_at DESC
            "#
        )
        .bind(worker_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn create_job(
    &self,
    employer_id: Uuid,
    category: WorkerCategory,
    title: String,
    description: String,
    location_state: String,
    location_city: String,
    location_address: String,
    budget: f64,
    estimated_duration_days: i32,
    partial_payment_allowed: bool,
    partial_payment_percentage: Option<i32>,
    deadline: Option<DateTime<Utc>>,
) -> Result<Job, Error> {
    let platform_fee = budget * 0.03;
    let escrow_amount = budget + platform_fee;

    let platform_fee_bd = BigDecimal::try_from(platform_fee)
        .map_err(|_| sqlx::Error::Decode("Invalid platform fee".into()))?;

    let escrow_fee_bd = BigDecimal::try_from(escrow_amount)
        .map_err(|_| sqlx::Error::Decode("Invalid escrow fee".into()))?;

    let budget_bd = BigDecimal::try_from(budget)
        .map_err(|_| sqlx::Error::Decode("Invalid budget".into()))?;

    sqlx::query_as::<_, Job>(
        r#"
        INSERT INTO jobs 
        (employer_id, category, title, description, location_state, location_city, location_address,
        budget, estimated_duration_days, platform_fee, escrow_amount, partial_payment_allowed, 
        partial_payment_percentage, deadline) 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) 
        RETURNING 
            id, employer_id, 
            assigned_worker_id,
            category, 
            title, description,
            location_state, location_city, location_address, 
            budget,
            estimated_duration_days, 
            status, 
            payment_status, 
            escrow_amount, platform_fee,
            partial_payment_allowed, 
            partial_payment_percentage, 
            created_at, updated_at, 
            deadline
        "#
    )
    .bind(employer_id)
    .bind(category)
    .bind(title)
    .bind(description)
    .bind(location_state)
    .bind(location_city)
    .bind(location_address)
    .bind(budget_bd)
    .bind(estimated_duration_days)
    .bind(platform_fee_bd)
    .bind(escrow_fee_bd)
    .bind(partial_payment_allowed)
    .bind(partial_payment_percentage)
    .bind(deadline)
    .fetch_one(&self.pool)
    .await
}

async fn get_jobs_by_location(
    &self,
    state: &str,
    status: JobStatus,
) -> Result<Vec<Job>, Error> {
    sqlx::query_as::<_, Job>(
        r#"
        SELECT 
            id, employer_id, 
            assigned_worker_id,
            category,
            title, description, 
            location_state, location_city, location_address, 
            budget,
            estimated_duration_days, 
            status, 
            payment_status, 
            escrow_amount, platform_fee,
            partial_payment_allowed, 
            partial_payment_percentage,
            created_at, updated_at, 
            deadline
        FROM jobs 
        WHERE location_state = $1 AND status = $2
        ORDER BY created_at DESC
        "#
    )
    .bind(state)
    .bind(status)
    .fetch_all(&self.pool)
    .await
}

async fn get_jobs_by_category(
    &self,
    category: WorkerCategory,
    status: JobStatus,
) -> Result<Vec<Job>, Error> {
    sqlx::query_as::<_, Job>(
        r#"
        SELECT 
            id, employer_id, 
            assigned_worker_id,
            category,
            title, description, 
            location_state, location_city, location_address, 
            budget,
            estimated_duration_days, 
            status, 
            payment_status, 
            escrow_amount, platform_fee,
            partial_payment_allowed, 
            partial_payment_percentage,
            created_at, updated_at, 
            deadline
        FROM jobs 
        WHERE category = $1 AND status = $2
        ORDER BY created_at DESC
        "#
    )
    .bind(category)
    .bind(status)
    .fetch_all(&self.pool)
    .await
}

async fn get_open_jobs(&self) -> Result<Vec<Job>, Error> {
    sqlx::query_as::<_, Job>(
        r#"
        SELECT 
            id, employer_id, 
            assigned_worker_id,
            category,
            title, description, 
            location_state, location_city, location_address, 
            budget,
            estimated_duration_days, 
            status, 
            payment_status, 
            escrow_amount, platform_fee,
            partial_payment_allowed, 
            partial_payment_percentage,
            created_at, updated_at, 
            deadline
        FROM jobs 
        WHERE status = 'open'::job_status
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(&self.pool)
    .await
}

async fn get_jobs_by_location_and_category(
    &self,
    state: &str,
    category: WorkerCategory,
    status: JobStatus,
) -> Result<Vec<Job>, Error> {
    sqlx::query_as::<_, Job>(
        r#"
        SELECT 
            id, employer_id, 
            assigned_worker_id,
            category,
            title, description, 
            location_state, location_city, location_address, 
            budget,
            estimated_duration_days, 
            status, 
            payment_status, 
            escrow_amount, platform_fee,
            partial_payment_allowed, 
            partial_payment_percentage,
            created_at, updated_at, 
            deadline
        FROM jobs 
        WHERE location_state = $1 AND category = $2 AND status = $3
        ORDER BY created_at DESC
        "#
    )
    .bind(state)
    .bind(category)
    .bind(status)
    .fetch_all(&self.pool)
    .await
}

   async fn get_job_by_id(&self, job_id: Uuid) -> Result<Option<Job>, Error> {
    sqlx::query_as::<_, Job>(
        r#"
        SELECT 
            id, employer_id, 
            assigned_worker_id,
            category,
            title, description, 
            location_state, location_city, location_address, 
            budget,
            estimated_duration_days, 
            status, 
            payment_status, 
            escrow_amount, platform_fee,
            partial_payment_allowed, 
            partial_payment_percentage,
            created_at, updated_at, 
            deadline
        FROM jobs WHERE id = $1
        "#
    )
    .bind(job_id)
    .fetch_optional(&self.pool)
    .await
}

    async fn update_job_status(
        &self,
        job_id: Uuid,
        status: JobStatus,
    ) -> Result<Job, Error> {
        sqlx::query_as::<_, Job>(
            r#"
            UPDATE jobs 
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING id, employer_id, assigned_worker_id, category,
            title, description, location_state, location_city, location_address, budget,
            estimated_duration_days, status, 
            payment_status, escrow_amount, platform_fee,
            partial_payment_allowed, partial_payment_percentage, created_at, updated_at, deadline
            "#
        )
        .bind(job_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }

    async fn assign_worker_to_job(
        &self,
        job_id: Uuid,
        employer_user_id: Uuid,
        worker_profile_id: Uuid,
    ) -> Result<Job, SqlxError> {
        let mut tx = self.pool.begin().await?;

        println!("🔍 [assign_worker_to_job] Starting assignment - Job: {}, Worker Profile: {}", job_id, worker_profile_id);

        // 1. Check if job is still open and get current state
        let job = sqlx::query_as::<_, Job>(
            r#"
            SELECT * FROM jobs 
            WHERE id = $1 AND status = 'open'::job_status
            FOR UPDATE
            "#
        )
        .bind(job_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            println!("❌ [assign_worker_to_job] Job not found or not open: {}", job_id);
            SqlxError::RowNotFound
        })?;

        println!("✅ [assign_worker_to_job] Job found and is open");

        // 2. Check if contract already exists for this job
        let existing_contract = sqlx::query_as::<_, JobContract>(
            r#"
            SELECT * FROM job_contracts WHERE job_id = $1
            "#
        )
        .bind(job_id)
        .fetch_optional(&mut *tx)
        .await?;

        if existing_contract.is_some() {
            println!("❌ [assign_worker_to_job] Contract already exists for job: {}", job_id);
            return Err(SqlxError::Protocol("contract_already_exists".into()));
        }

        println!("✅ [assign_worker_to_job] No existing contract found");

        // 3. Get worker profile
        let worker_profile = self.get_worker_profile_by_id(worker_profile_id).await?;
        let worker_user_id = worker_profile.user_id;

        println!("✅ [assign_worker_to_job] Worker profile found - User ID: {}", worker_user_id);

        // 4. Update job with assigned worker
        let updated_job = sqlx::query_as::<_, Job>(
            r#"
            UPDATE jobs 
            SET assigned_worker_id = $2, status = 'in_progress'::job_status, updated_at = NOW()
            WHERE id = $1
            RETURNING id, employer_id, assigned_worker_id, category, title, description, 
            location_state, location_city, location_address, budget, estimated_duration_days, 
            status, payment_status, escrow_amount, platform_fee, partial_payment_allowed, 
            partial_payment_percentage, created_at, updated_at, deadline
            "#
        )
        .bind(job_id)
        .bind(worker_profile_id)
        .fetch_one(&mut *tx)
        .await?;

        println!("✅ [assign_worker_to_job] Job updated successfully");

        // We no longer create contract or escrow at assignment time.
        // Assignment only updates the job with the assigned worker.
        tx.commit().await?;
        println!("✅ [assign_worker_to_job] Transaction committed successfully");

        Ok(updated_job)
    }

    async fn create_job_application(
        &self,
        job_id: Uuid,
        worker_id: Uuid,
        proposed_rate: f64,
        estimated_completion: i32,
        cover_letter: String,
    ) -> Result<JobApplication, Error> {
        let proposed_rate_bd = BigDecimal::try_from(proposed_rate)
            .map_err(|_| sqlx::Error::Decode("Invalid proposed rate".into()))?;

        sqlx::query_as::<_, JobApplication>(
            r#"
            INSERT INTO job_applications 
            (job_id, worker_id, proposed_rate, estimated_completion, cover_letter)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, job_id, worker_id, proposed_rate, estimated_completion, 
            cover_letter, status, created_at
            "#
        )
        .bind(job_id)
        .bind(worker_id)
        .bind(proposed_rate_bd)
        .bind(estimated_completion)
        .bind(cover_letter)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_job_applications(
        &self,
        job_id: Uuid
    ) -> Result<Vec<JobApplication>, Error> {
        sqlx::query_as::<_, JobApplication>(
            r#"
            SELECT id, job_id, worker_id, proposed_rate, estimated_completion, 
            cover_letter, status, created_at
            FROM job_applications 
            WHERE job_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn update_application_status(
        &self,
        application_id: Uuid,
        status: String,
    ) -> Result<JobApplication, Error> {
        sqlx::query_as::<_, JobApplication>(
            r#"
            UPDATE job_applications 
            SET status = $2
            WHERE id = $1
            RETURNING id, job_id, worker_id, proposed_rate, estimated_completion, 
            cover_letter, status, created_at
            "#
        )
        .bind(application_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }

    async fn create_job_contract(
        &self,
        job_id: Uuid,
        employer_id: Uuid,
        worker_id: Uuid,
        agreed_rate: f64,
        agreed_timeline: i32,
        terms: String,
    ) -> Result<JobContract, Error> {
        let agreed_rate_bd = BigDecimal::try_from(agreed_rate)
            .map_err(|_| sqlx::Error::Decode("Invalid agreed rate".into()))?;

        sqlx::query_as::<_, JobContract>(
            r#"
            INSERT INTO job_contracts 
            (job_id, employer_id, worker_id, agreed_rate, agreed_timeline, terms)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, job_id, employer_id, worker_id, agreed_rate, agreed_timeline,
            terms, signed_by_employer, signed_by_worker, contract_date
            "#
        )
        .bind(job_id)
        .bind(employer_id)
        .bind(worker_id)
        .bind(agreed_rate_bd)
        .bind(agreed_timeline)
        .bind(terms)
        .fetch_one(&self.pool)
        .await
    }

    async fn sign_contract(
        &self,
        contract_id: Uuid,
        signer_role: String,
    ) -> Result<JobContract, Error> {
        if signer_role == "employer" {
            sqlx::query_as::<_, JobContract>(
                r#"
                UPDATE job_contracts 
                SET signed_by_employer = true
                WHERE id = $1
                RETURNING id, job_id, employer_id, worker_id, agreed_rate, agreed_timeline,
                terms, signed_by_employer, signed_by_worker, contract_date
                "#
            )
            .bind(contract_id)
            .fetch_one(&self.pool)
            .await
        } else if signer_role == "worker" {
            sqlx::query_as::<_, JobContract>(
                r#"
                UPDATE job_contracts 
                SET signed_by_worker = true
                WHERE id = $1
                RETURNING id, job_id, employer_id, worker_id, agreed_rate, agreed_timeline,
                terms, signed_by_employer, signed_by_worker, contract_date
                "#
            )
            .bind(contract_id)
            .fetch_one(&self.pool)
            .await
        } else {
            Err(sqlx::Error::Decode("Invalid signer role".into()))
        }
    }

    async fn sign_contract_tx(
        &self,
        contract_id: Uuid,
        signer_role: String,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<JobContract, Error> {
        if signer_role == "employer" {
            sqlx::query_as::<_, JobContract>(
                r#"
                UPDATE job_contracts 
                SET signed_by_employer = true
                WHERE id = $1
                RETURNING id, job_id, employer_id, worker_id, agreed_rate, agreed_timeline,
                terms, signed_by_employer, signed_by_worker, contract_date
                "#
            )
            .bind(contract_id)
            .fetch_one(&mut **tx)
            .await
        } else if signer_role == "worker" {
            sqlx::query_as::<_, JobContract>(
                r#"
                UPDATE job_contracts 
                SET signed_by_worker = true
                WHERE id = $1
                RETURNING id, job_id, employer_id, worker_id, agreed_rate, agreed_timeline,
                terms, signed_by_employer, signed_by_worker, contract_date
                "#
            )
            .bind(contract_id)
            .fetch_one(&mut **tx)
            .await
        } else {
            Err(sqlx::Error::Decode("Invalid signer role".into()))
        }
    }

    async fn create_escrow_transaction(
        &self,
        job_id: Uuid,
        employer_id: Uuid,
        worker_id: Option<Uuid>,
        amount: f64,
        platform_fee: f64, 
    ) -> Result<EscrowTransaction, Error> {
        let amount_bd = BigDecimal::try_from(amount)
            .map_err(|_| sqlx::Error::Decode("Invalid amount".into()))?;
        let platform_fee_bd = BigDecimal::try_from(platform_fee)
            .map_err(|_| sqlx::Error::Decode("Invalid platform fee".into()))?;

        sqlx::query_as::<_, EscrowTransaction>(
            r#"
            INSERT INTO escrow_transactions 
            (job_id, employer_id, worker_id, amount, platform_fee, status)
            VALUES ($1, $2, $3, $4, $5, 'escrowed'::payment_status)
            RETURNING id, job_id, employer_id, worker_id, amount, platform_fee,
            status, transaction_hash, wallet_hold_id, created_at, released_at
            "#
        )
        .bind(job_id)
        .bind(employer_id)
        .bind(worker_id)
        .bind(amount_bd)
        .bind(platform_fee_bd)
        .fetch_one(&self.pool)
        .await
    }

    async fn create_escrow_with_hold(
        &self,
        job_id: Uuid,
        employer_id: Uuid,
        amount: f64,
        platform_fee: f64,
    ) -> Result<EscrowTransaction, Error> {
        use crate::models::walletmodels::WalletHold;

        let mut tx = self.pool.begin().await?;

        let amount_bd = BigDecimal::try_from(amount)
            .map_err(|_| sqlx::Error::Decode("Invalid amount".into()))?;
        let platform_fee_bd = BigDecimal::try_from(platform_fee)
            .map_err(|_| sqlx::Error::Decode("Invalid platform fee".into()))?;

        // 1) create escrow row (without wallet_hold_id yet)
        let escrow: EscrowTransaction = sqlx::query_as::<_, EscrowTransaction>(
            r#"
            INSERT INTO escrow_transactions 
            (job_id, employer_id, worker_id, amount, platform_fee, status)
            VALUES ($1, $2, $3, $4, $5, 'escrowed'::payment_status)
            RETURNING id, job_id, employer_id, worker_id, amount, platform_fee,
            status, transaction_hash, wallet_hold_id, created_at, released_at
            "#
        )
        .bind(job_id)
        .bind(employer_id)
        .bind(None::<Uuid>)
        .bind(amount_bd.clone())
        .bind(platform_fee_bd.clone())
        .fetch_one(&mut *tx)
        .await?;

        // 2) lock employer wallet and ensure available balance
        let wallet_row = sqlx::query(
            "SELECT id, available_balance FROM naira_wallets WHERE user_id = $1 FOR UPDATE"
        )
        .bind(employer_id)
        .fetch_optional(&mut *tx)
        .await?;

        let wallet_row = wallet_row.ok_or_else(|| sqlx::Error::RowNotFound)?;
        let wallet_id: Uuid = wallet_row.get::<Uuid, _>("id");
        let available_balance: i64 = wallet_row.get::<i64, _>("available_balance");

        // convert amount to kobo
        let amount_kobo = naira_to_kobo(amount);

        if available_balance < amount_kobo {
            return Err(sqlx::Error::Protocol("insufficient_available_balance".into()));
        }

        // 3) reduce available balance
        sqlx::query(
            "UPDATE naira_wallets SET available_balance = available_balance - $2 WHERE id = $1"
        )
        .bind(wallet_id)
        .bind(amount_kobo)
        .execute(&mut *tx)
        .await?;

        // 4) create wallet hold
        let hold: WalletHold = sqlx::query_as::<_, WalletHold>(
            r#"
            INSERT INTO wallet_holds (wallet_id, job_id, amount, reason, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, wallet_id, job_id, amount, reason, status, created_at, expires_at, released_at
            "#
        )
        .bind(wallet_id)
        .bind(Some(job_id))
        .bind(amount_kobo)
        .bind(format!("Escrow hold for job {}", job_id))
        .bind(None::<chrono::DateTime<Utc>>)
        .fetch_one(&mut *tx)
        .await?;

        // 5) persist hold id on escrow row and return updated escrow
        let updated_escrow: EscrowTransaction = sqlx::query_as::<_, EscrowTransaction>(
            r#"
            UPDATE escrow_transactions
            SET wallet_hold_id = $2
            WHERE id = $1
            RETURNING id, job_id, employer_id, worker_id, amount, platform_fee, status, transaction_hash, wallet_hold_id, created_at, released_at
            "#
        )
        .bind(escrow.id)
        .bind(hold.id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated_escrow)
    }

    async fn update_escrow_wallet_hold_id(&self, escrow_id: Uuid, wallet_hold_id: Uuid) -> Result<EscrowTransaction, Error> {
        sqlx::query_as::<_, EscrowTransaction>(
            r#"
            UPDATE escrow_transactions
            SET wallet_hold_id = $2
            WHERE id = $1
            RETURNING id, job_id, employer_id, worker_id, amount, platform_fee, status, transaction_hash, wallet_hold_id, created_at, released_at
            "#
        )
        .bind(escrow_id)
        .bind(wallet_hold_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn update_escrow_worker(&self, escrow_id: Uuid, worker_id: Uuid) -> Result<EscrowTransaction, Error> {
        sqlx::query_as::<_, EscrowTransaction>(
            r#"
            UPDATE escrow_transactions
            SET worker_id = $2
            WHERE id = $1
            RETURNING id, job_id, employer_id, worker_id, amount, platform_fee, status, transaction_hash, wallet_hold_id, created_at, released_at
            "#
        )
        .bind(escrow_id)
        .bind(worker_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn release_escrow_payment(
    &self,
    escrow_id: Uuid,
    release_percentage: f64,
) -> Result<EscrowTransaction, Error> {
    let mut tx = self.pool.begin().await?;

    // Get escrow details
    let escrow = sqlx::query_as::<_, EscrowTransaction>(
        r#"
        SELECT id, job_id, employer_id, worker_id, amount, platform_fee,
        status, transaction_hash, created_at, released_at
        FROM escrow_transactions 
        WHERE id = $1 AND status = 'escrowed'::payment_status
        FOR UPDATE
        "#
    )
    .bind(escrow_id)
    .fetch_one(&mut *tx)
    .await?;

    // Calculate release amount
    // release_percentage is a fraction between 0.0 and 1.0 (e.g., 0.3 for 30%)
    let release_amount = (escrow.amount.to_f64().unwrap_or(0.0) * release_percentage) as f64;
    let release_amount_bd = BigDecimal::try_from(release_amount)
        .map_err(|_| sqlx::Error::Decode("Invalid release amount".into()))?;

    // Update escrow status and release amount
    let updated_escrow = sqlx::query_as::<_, EscrowTransaction>(
        r#"
        UPDATE escrow_transactions 
        SET 
            amount = amount - $2,
            status = CASE 
                WHEN amount - $2 <= 0 THEN 'completed'::payment_status 
                ELSE 'partially_paid'::payment_status 
            END,
            released_at = CASE 
                WHEN amount - $2 <= 0 THEN NOW() 
                ELSE released_at 
            END
        WHERE id = $1
        RETURNING id, job_id, employer_id, worker_id, amount, platform_fee,
        status, transaction_hash, wallet_hold_id, created_at, released_at
        "#
    )
    .bind(escrow_id)
    .bind(release_amount_bd)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(updated_escrow)
}

async fn get_escrow_transaction(
    &self,
    escrow_id: Uuid,
) -> Result<Option<EscrowTransaction>, Error> {
    sqlx::query_as::<_, EscrowTransaction>(
        r#"
        SELECT id, job_id, employer_id, worker_id, amount, platform_fee,
        status, transaction_hash, created_at, released_at
        FROM escrow_transactions 
        WHERE id = $1
        "#
    )
    .bind(escrow_id)
    .fetch_optional(&self.pool)
    .await
}

    async fn submit_job_progress(
        &self,
        job_id: Uuid,
        worker_id: Uuid,
        progress_percentage: i32,
        description: String,
        image_urls: Vec<String>,
    ) -> Result<JobProgress, Error> {
        sqlx::query_as::<_, JobProgress>(
            r#"
            INSERT INTO job_progress 
            (job_id, worker_id, progress_percentage, description, image_urls)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, job_id, worker_id, progress_percentage, description, 
            image_urls, submitted_at
            "#
        )
        .bind(job_id)
        .bind(worker_id)
        .bind(progress_percentage)
        .bind(description)
        .bind(image_urls)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_job_progress(
        &self,
        job_id: Uuid,
    ) -> Result<Vec<JobProgress>, Error> {
        sqlx::query_as::<_, JobProgress>(
            r#"
            SELECT id, job_id, worker_id, progress_percentage, description, 
            image_urls, submitted_at
            FROM job_progress 
            WHERE job_id = $1
            ORDER BY submitted_at DESC
            "#
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn create_dispute(
        &self,
        job_id: Uuid,
        raised_by: Uuid,
        against: Uuid,
        reason: String,
        description: String,
        evidence_urls: Vec<String>,
    ) -> Result<Dispute, Error> {
        sqlx::query_as::<_, Dispute>(
            r#"
            INSERT INTO disputes 
            (job_id, raised_by, against, reason, description, evidence_urls)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, job_id, raised_by, against, reason, description, 
            evidence_urls, status, assigned_verifier, 
            resolution, created_at, resolved_at
            "#
        )
        .bind(job_id)
        .bind(raised_by)
        .bind(against)
        .bind(reason)
        .bind(description)
        .bind(evidence_urls)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_dispute_by_id(&self, dispute_id: Uuid) -> Result<Option<Dispute>, Error> {
        sqlx::query_as::<_, Dispute>(
            r#"
            SELECT id, job_id, raised_by, against, reason, description, 
                   evidence_urls, status, assigned_verifier, resolution, created_at, resolved_at
            FROM disputes 
            WHERE id = $1
            "#
        )
        .bind(dispute_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn assign_verifer_to_dispute(
        &self,
        dispute_id: Uuid,
        verifier_id: Uuid
    ) -> Result<Dispute, Error> {
        sqlx::query_as::<_, Dispute>(
            r#"
            UPDATE disputes 
            SET assigned_verifier = $2, status = 'under_review'::dispute_status
            WHERE id = $1
            RETURNING id, job_id, raised_by, against, reason, description, 
            evidence_urls, status, assigned_verifier, 
            resolution, created_at, resolved_at
            "#
        )
        .bind(dispute_id)
        .bind(verifier_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn resolve_dispute(
        &self,
        dispute_id: Uuid,
        resolution: String,
        decision: String,
    ) -> Result<Dispute, Error> {
        sqlx::query_as::<_, Dispute>(
            r#"
            UPDATE disputes 
            SET resolution = $2, status = 'resolved'::dispute_status, resolved_at = NOW()
            WHERE id = $1
            RETURNING id, job_id, raised_by, against, reason, description, 
            evidence_urls, status, assigned_verifier, 
            resolution, created_at, resolved_at
            "#
        )
        .bind(dispute_id)
        .bind(resolution)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_pending_verifications_f(
        &self,
        verifier_id: Uuid,
    ) -> Result<Vec<Dispute>, Error> {
        sqlx::query_as::<_, Dispute>(
            r#"
            SELECT id, job_id, raised_by, against, reason, description, 
            evidence_urls, status, assigned_verifier, 
            resolution, created_at, resolved_at
            FROM disputes 
            WHERE assigned_verifier = $1 AND status = 'under_review'::dispute_status
            ORDER BY created_at DESC
            "#
        )
        .bind(verifier_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn create_job_review(
        &self,
        job_id: Uuid,
        reviewer_id: Uuid,
        reviewee_id: Uuid,
        rating: i32,
        comment: String
    ) -> Result<JobReview, Error> {
        sqlx::query_as::<_, JobReview>(
            r#"
            INSERT INTO job_reviews 
            (job_id, reviewer_id, reviewee_id, rating, comment)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, job_id, reviewer_id, reviewee_id, rating, comment, created_at
            "#
        )
        .bind(job_id)
        .bind(reviewer_id)
        .bind(reviewee_id)
        .bind(rating)
        .bind(comment)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_worker_reviews(
        &self,
        worker_id: Uuid,
    ) -> Result<Vec<JobReview>, Error> {
        sqlx::query_as::<_, JobReview>(
            r#"
            SELECT id, job_id, reviewer_id, reviewee_id, rating, comment, created_at
            FROM job_reviews 
            WHERE reviewee_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(worker_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn update_worker_rating(
        &self,
        worker_id: Uuid,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
            UPDATE worker_profiles 
            SET rating = (
                SELECT COALESCE(AVG(rating), 0) 
                FROM job_reviews 
                WHERE reviewee_id = $1
            )
            WHERE user_id = $1
            "#
        )
        .bind(worker_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    async fn award_job_completion_points(
    &self,
    worker_id: Uuid,
    employer_id: Uuid,
    job_rating: i32,
    completed_on_time: bool,
) -> Result<(), Error> {
    let mut tx = self.pool.begin().await?;

    let mut worker_points = 20; // Base points for worker
    let mut employer_points = 10; // Base points for employer
    
    // Bonus points for high ratings
    if job_rating >= 4 {
        worker_points += 10;
        employer_points += 5;
    }
    
    // Bonus for timely completion
    if completed_on_time {
        worker_points += 5;
        employer_points += 5;
    }

    // Award points to worker
    sqlx::query(
        r#"
        UPDATE users 
        SET trust_score = trust_score + $1, updated_at = NOW()
        WHERE id = $2
        "#
    )
    .bind(worker_points)
    .bind(worker_id)
    .execute(&mut *tx)
    .await?;

    // Award points to employer
    sqlx::query(
        r#"
        UPDATE users 
        SET trust_score = trust_score + $1, updated_at = NOW()
        WHERE id = $2
        "#
    )
    .bind(employer_points)
    .bind(employer_id)
    .execute(&mut *tx)
    .await?;

    // Update worker's completed jobs count
    sqlx::query(
        r#"
        UPDATE worker_profiles 
        SET completed_jobs = COALESCE(completed_jobs, 0) + 1, 
            updated_at = NOW()
        WHERE user_id = $1
        "#
    )
    .bind(worker_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

    async fn get_jobs_by_employer_and_status(
        &self,
        employer_id: Uuid,
        status: JobStatus,
    ) -> Result<Vec<Job>, Error> {
        sqlx::query_as::<_, Job>(
            r#"
            SELECT 
                id, employer_id, 
                assigned_worker_id,
                category,
                title, description, 
                location_state, location_city, location_address, 
                budget,
                estimated_duration_days, 
                status, 
                payment_status, 
                escrow_amount, platform_fee,
                partial_payment_allowed, 
                partial_payment_percentage,
                created_at, updated_at, 
                deadline
            FROM jobs 
            WHERE employer_id = $1 AND status = $2
            ORDER BY created_at DESC
            "#
        )
        .bind(employer_id)
        .bind(status)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_escrow_by_job_id(
        &self, 
        job_id: Uuid
    ) -> Result<Option<EscrowTransaction>, Error> {
        sqlx::query_as::<_, EscrowTransaction>(
            r#"
            SELECT id, job_id, employer_id, worker_id, amount, platform_fee,
            status, transaction_hash, wallet_hold_id, created_at, released_at
            FROM escrow_transactions 
            WHERE job_id = $1
            "#
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn get_escrow_by_id(
        &self, 
        escrow_id: Uuid
    ) -> Result<Option<EscrowTransaction>, Error> {
        sqlx::query_as::<_, EscrowTransaction>(
            r#"
            SELECT id, job_id, employer_id, worker_id, amount, platform_fee,
            status, transaction_hash, wallet_hold_id, created_at, released_at
            FROM escrow_transactions 
            WHERE id = $1
            "#
        )
        .bind(escrow_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn get_employer_jobs(
        &self, 
        employer_id: Uuid
    ) -> Result<Vec<Job>, Error> {
        sqlx::query_as::<_, Job>(
            r#"
            SELECT 
                id, employer_id, 
                assigned_worker_id,
                category,
                title, description, 
                location_state, location_city, location_address, 
                budget,
                estimated_duration_days, 
                status, 
                payment_status, 
                escrow_amount, platform_fee,
                partial_payment_allowed, 
                partial_payment_percentage,
                created_at, updated_at, 
                deadline
            FROM jobs 
            WHERE employer_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(employer_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_employer_active_contracts(
        &self,
        employer_id: Uuid,
    ) -> Result<Vec<JobContract>, sqlx::Error> {
        sqlx::query_as::<_, JobContract>(
            r#"
            SELECT 
                c.id, c.job_id, c.employer_id, c.worker_id,
                c.agreed_rate, c.agreed_timeline, c.terms,
                c.signed_by_employer, c.signed_by_worker, c.status,
                c.created_at, c.updated_at, c.contract_date
            FROM job_contracts c
            INNER JOIN jobs j ON c.job_id = j.id
            WHERE c.employer_id = $1 
            AND j.status = 'in_progress'::job_status
            ORDER BY c.contract_date DESC
            "#
        )
        .bind(employer_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_employer_pending_applications(
        &self, 
        employer_id: Uuid
    ) -> Result<Vec<JobApplication>, Error> {
        sqlx::query_as::<_, JobApplication>(
            r#"
            SELECT ja.id, ja.job_id, ja.worker_id, ja.proposed_rate, ja.estimated_completion, 
            ja.cover_letter, ja.status, ja.created_at
            FROM job_applications ja
            INNER JOIN jobs j ON ja.job_id = j.id
            WHERE j.employer_id = $1 AND ja.status = 'applied'
            ORDER BY ja.created_at DESC
            "#
        )
        .bind(employer_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_job_application_by_id(
        &self, 
        application_id: Uuid
    ) -> Result<Option<JobApplication>, Error> {
        sqlx::query_as::<_, JobApplication>(
            r#"
            SELECT id, job_id, worker_id, proposed_rate, estimated_completion, 
            cover_letter, status, created_at
            FROM job_applications 
            WHERE id = $1
            "#
        )
        .bind(application_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn get_worker_active_jobs(
        &self, 
        worker_id: Uuid
    ) -> Result<Vec<Job>, Error> {
        sqlx::query_as::<_, Job>(
            r#"
            SELECT 
                id, employer_id, 
                assigned_worker_id,
                category,
                title, description, 
                location_state, location_city, location_address, 
                budget,
                estimated_duration_days, 
                status, 
                payment_status, 
                escrow_amount, platform_fee,
                partial_payment_allowed, 
                partial_payment_percentage,
                created_at, updated_at, 
                deadline
            FROM jobs 
            WHERE assigned_worker_id = $1 AND status = 'in_progress'::job_status
            ORDER BY created_at DESC
            "#
        )
        .bind(worker_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_worker_pending_applications(
        &self, 
        worker_id: Uuid
    ) -> Result<Vec<JobApplication>, Error> {
        sqlx::query_as::<_, JobApplication>(
            r#"
            SELECT id, job_id, worker_id, proposed_rate, estimated_completion, 
            cover_letter, status, created_at
            FROM job_applications 
            WHERE worker_id = $1 AND status = 'applied'
            ORDER BY created_at DESC
            "#
        )
        .bind(worker_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_portfolio_item_by_id(
        &self,
        item_id: Uuid,
    ) -> Result<Option<WorkerPortfolio>, Error> {
    sqlx::query_as::<_, WorkerPortfolio>(
        r#"
        SELECT id, worker_id, title, description, image_url, project_date, created_at
        FROM worker_portfolios 
        WHERE id = $1
        "#
    )
    .bind(item_id)
    .fetch_optional(&self.pool)
    .await
    }

    async fn delete_portfolio_item(
        &self,
        item_id: Uuid,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
            DELETE FROM worker_portfolios 
            WHERE id = $1
            "#
        )
        .bind(item_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_escrow_status(
        &self,
        escrow_id: Uuid,
        status: crate::models::labourmodel::PaymentStatus,
        transaction_hash: Option<String>,
    ) -> Result<EscrowTransaction, Error> {
        let mut update_fields = vec!["status = $2"];
        
        if status == crate::models::labourmodel::PaymentStatus::Funded {
            update_fields.push("funded_at = NOW()");
        } else if status == crate::models::labourmodel::PaymentStatus::Completed {
            update_fields.push("completed_at = NOW()");
        } else if status == crate::models::labourmodel::PaymentStatus::PartiallyPaid {
            update_fields.push("released_at = NOW()");
        }
        
        if let Some(ref _hash) = transaction_hash {
            update_fields.push("transaction_hash = $3");
        }
        
        let query_str = format!(
            r#"
            UPDATE escrow_transactions SET {}, updated_at = NOW()
            WHERE id = $1
            RETURNING id, job_id, employer_id, worker_id, amount, platform_fee,
            status, transaction_hash, created_at, released_at
            "#,
            update_fields.join(", ")
        );
        
        let mut query = sqlx::query_as::<_, EscrowTransaction>(&query_str)
            .bind(escrow_id)
            .bind(status);
        
        if transaction_hash.is_some() {
            query = query.bind(transaction_hash);
        }
        
        query.fetch_one(&self.pool).await
    }

    // High-value dispute verification implementations
    async fn get_admin_verification_for_dispute(
        &self,
        dispute_id: Uuid,
    ) -> Result<Option<AdminDisputeVerification>, Error> {
        sqlx::query_as::<_, AdminDisputeVerification>(
            r#"
            SELECT id, dispute_id, admin_id, verifier_resolution_id, admin_decision, 
                   admin_notes, status, created_at, verified_at
            FROM admin_dispute_verifications 
            WHERE dispute_id = $1 AND status = 'approved'
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .bind(dispute_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn create_pending_dispute_resolution(
        &self,
        dispute_id: Uuid,
        verifier_id: Uuid,
        resolution: String,
        decision: String,
        payment_percentage: Option<f64>,
    ) -> Result<PendingDisputeResolution, Error> {
        sqlx::query_as::<_, PendingDisputeResolution>(
            r#"
            INSERT INTO pending_dispute_resolutions 
            (dispute_id, verifier_id, resolution, decision, payment_percentage, status)
            VALUES ($1, $2, $3, $4, $5, 'pending_admin_verification')
            RETURNING id, dispute_id, verifier_id, resolution, decision, payment_percentage, 
                      status, created_at, admin_verified_at
            "#
        )
        .bind(dispute_id)
        .bind(verifier_id)
        .bind(resolution)
        .bind(decision)
        .bind(payment_percentage)
        .fetch_one(&self.pool)
        .await
    }

    async fn assign_admin_to_dispute_verification(
        &self,
        dispute_id: Uuid,
        admin_id: Uuid,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
            INSERT INTO admin_dispute_verifications 
            (dispute_id, admin_id, status)
            VALUES ($1, $2, 'pending_review')
            ON CONFLICT (dispute_id) DO UPDATE SET
                admin_id = EXCLUDED.admin_id,
                status = EXCLUDED.status,
                created_at = NOW()
            "#
        )
        .bind(dispute_id)
        .bind(admin_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_dispute_status(
        &self,
        dispute_id: Uuid,
        status: DisputeStatus,
    ) -> Result<Dispute, Error> {
        sqlx::query_as::<_, Dispute>(
            r#"
            UPDATE disputes 
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING id, job_id, raised_by, against, reason, description, 
                   evidence_urls, status, assigned_verifier, 
                   resolution, created_at, resolved_at
            "#
        )
        .bind(dispute_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }
}