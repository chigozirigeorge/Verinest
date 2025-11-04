// db/labourdb.rs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::{Error, types::BigDecimal};
use num_traits::ToPrimitive;

use super::db::DBClient;
use crate::{models::labourmodel::*, service::labour_service::JobAssignmentResult};

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
) -> Result<JobAssignmentResult, Error>;

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

    //Escrow management
    async fn create_escrow_transaction(
        &self,
        job_id: Uuid,
        employer_id: Uuid,
        worker_id: Uuid,
        amount: f64,
        platform_fee: f64, 
    ) -> Result<EscrowTransaction, Error>;

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
                is_available, rating, completed_jobs, 
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
                is_available, rating, completed_jobs, 
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
            is_available, rating, completed_jobs, 
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
                is_available, rating, completed_jobs, 
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
            daily_rate, location_state, location_city, is_available, rating, completed_jobs, created_at, updated_at
            FROM worker_profiles
            WHERE location_state = $1 AND category = $2 AND is_available = true
            ORDER BY rating DESC, completed_jobs DESC
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

// In labourdb.rs - FIXED VERSION
async fn assign_worker_to_job(
    &self,
    job_id: Uuid,
    employer_user_id: Uuid,
    worker_profile_id: Uuid, // This should be profile_id (from worker_profiles.id)
) -> Result<JobAssignmentResult, Error> {
    let mut tx = self.pool.begin().await?;

    // 1. Get worker profile to get user_id for contracts/escrow
    let worker_profile = self.get_worker_profile_by_id(worker_profile_id).await?;
    let worker_user_id = worker_profile.user_id;

    // 2. Update job with profile_id (matches your schema)
    let job = sqlx::query_as::<_, Job>(
        r#"
        UPDATE jobs 
        SET assigned_worker_id = $2, status = 'in_progress'::job_status, updated_at = NOW()
        WHERE id = $1 AND status = 'open'::job_status
        RETURNING id, employer_id, assigned_worker_id, category, title, description, 
        location_state, location_city, location_address, budget, estimated_duration_days, 
        status, payment_status, escrow_amount, platform_fee, partial_payment_allowed, 
        partial_payment_percentage, created_at, updated_at, deadline
        "#
    )
    .bind(job_id)
    .bind(worker_profile_id) // ✅ CORRECT: profile_id for job assignment
    .fetch_one(&mut *tx)
    .await?;

    // 3. Create contract with user_id (✅ CORRECT: contracts reference users)
    let contract = self.create_job_contract(
        job_id,
        employer_user_id,
        worker_user_id, // ✅ CORRECT: user_id for contracts
        job.budget.to_f64().unwrap_or(0.0),
        job.estimated_duration_days,
        "Standard work agreement terms".to_string(),
    ).await?;

    // 4. Create escrow with user_id (✅ CORRECT: escrow references users)
    let escrow = sqlx::query_as::<_, EscrowTransaction>(
        r#"
        INSERT INTO escrow_transactions 
        (job_id, employer_id, worker_id, amount, platform_fee, status)
        VALUES ($1, $2, $3, $4, $5, 'escrowed'::payment_status)
        RETURNING id, job_id, employer_id, worker_id, amount, platform_fee,
        status, transaction_hash, created_at, released_at
        "#
    )
    .bind(job_id.clone())
    .bind(job.employer_id.clone())
    .bind(worker_user_id) // ✅ CORRECT: user_id for escrow
    .bind(job.escrow_amount.clone())
    .bind(job.platform_fee.clone())
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    
    Ok(JobAssignmentResult {
        job,
        contract,
        escrow,
    })
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

    async fn create_escrow_transaction(
        &self,
        job_id: Uuid,
        employer_id: Uuid,
        worker_id: Uuid,
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
            status, transaction_hash, created_at, released_at
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

    async fn update_escrow_status(
        &self,
        escrow_id: Uuid,
        status: PaymentStatus,
        transaction_hash: Option<String>,
    ) -> Result<EscrowTransaction, Error> {
        sqlx::query_as::<_, EscrowTransaction>(
            r#"
            UPDATE escrow_transactions 
            SET status = $2, transaction_hash = $3
            WHERE id = $1
            RETURNING id, job_id, employer_id, worker_id, amount, platform_fee,
            status, transaction_hash, created_at, released_at
            "#
        )
        .bind(escrow_id)
        .bind(status)
        .bind(transaction_hash)
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
    let release_amount = (escrow.amount.to_f64().unwrap_or(0.0) * release_percentage / 100.0) as f64;
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
        status, transaction_hash, created_at, released_at
        "#
    )
    .bind(escrow_id)
    .bind(release_amount_bd)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(updated_escrow)
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
            status, transaction_hash, created_at, released_at
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
            status, transaction_hash, created_at, released_at
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
        employer_id: Uuid
    ) -> Result<Vec<JobContract>, Error> {
        sqlx::query_as::<_, JobContract>(
            r#"
            SELECT c.id, c.job_id, c.employer_id, c.worker_id, c.agreed_rate, 
            c.agreed_timeline, c.terms, c.signed_by_employer, c.signed_by_worker, c.contract_date
            FROM job_contracts c
            INNER JOIN jobs j ON c.job_id = j.id
            WHERE c.employer_id = $1 AND j.status = 'in_progress'::job_status
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
            WHERE j.employer_id = $1 AND ja.status = 'pending'
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
            WHERE worker_id = $1 AND status = 'pending'
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
}