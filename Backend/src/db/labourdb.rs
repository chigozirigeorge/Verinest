use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::{Error, types::BigDecimal};

use super::db::DBClient;
use crate::models::labourmodel::*;

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

    async fn get_job_by_id(&self, job_id: Uuid) -> Result<Option<Job>, Error>;

    async fn update_job_status(
        &self,
        job_id: Uuid,
        status: JobStatus,
    ) -> Result<Job, Error>;

    async fn assign_worker_to_job(
    &self,
    job_id: Uuid,
    worker_id: Uuid
) -> Result<(Job, EscrowTransaction), Error>;

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

    async fn get_pending_verifications(
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

        sqlx::query_as!(
            WorkerProfile,
            r#"
            INSERT INTO worker_profiles
            (user_id, category, experience_years, description, hourly_rate, daily_rate, location_state, location_city)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING 
                id, user_id, 
                category as "category: WorkerCategory", 
                experience_years, description, 
                hourly_rate, daily_rate, 
                location_state, location_city, 
                is_available, rating, completed_jobs, 
                created_at, updated_at
            "#,
            user_id,
            category as WorkerCategory,
            experience_years,
            description,
            hourly_rate_bd,
            daily_rate_bd,
            location_state,
            location_city
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_worker_profile(
        &self,
        user_id: Uuid
    ) -> Result<WorkerProfile, Error> {
        let profile = sqlx::query_as!(
            WorkerProfile,
            r#"
            SELECT 
                id, user_id, 
                category as "category: WorkerCategory", 
                experience_years, description, 
                hourly_rate, daily_rate, 
                location_state, location_city, 
                is_available, rating, completed_jobs, 
                created_at, updated_at
            FROM worker_profiles
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        // Convert Option to Result
        profile.ok_or_else(|| sqlx::Error::RowNotFound)
    }

    async fn update_worker_availability(
        &self,
        worker_id: Uuid,
        is_available: bool,
    ) -> Result<WorkerProfile, Error> {
        sqlx::query_as!(
            WorkerProfile,
            r#"
            UPDATE worker_profiles
            SET is_available = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, user_id, 
                category as "category: WorkerCategory", 
                experience_years, description, 
                hourly_rate, daily_rate, 
                location_state, location_city, 
                is_available, rating, completed_jobs, 
                created_at, updated_at
            "#,
            worker_id,
            is_available  
        )
        .fetch_one(&self.pool)
        .await
    }

    // Add stub implementations for all other trait methods to satisfy the compiler
    async fn get_workers_by_location_and_category(
        &self,
        state: &str,
        category: WorkerCategory,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkerProfile>, Error> {
        sqlx::query_as!(
            WorkerProfile,
            r#"
            SELECT id, user_id, category as "category: WorkerCategory", experience_years, description, hourly_rate, 
            daily_rate, location_state, location_city, is_available, rating, completed_jobs, created_at, updated_at
            FROM worker_profiles
            WHERE location_state = $1 AND category = $2 AND is_available = true
            ORDER BY rating DESC, completed_jobs DESC
            LIMIT $3 OFFSET $4
            "#,
            state,
            category as WorkerCategory,
            limit,
            offset
        )
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
        sqlx::query_as!(
            WorkerPortfolio,
            r#"
            INSERT INTO worker_portfolios 
            (worker_id, title, description, image_url, project_date)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, worker_id, title, description, image_url, project_date, created_at
            "#,
            worker_id,
            title,
            description,
            image_url,
            project_date
        )
        .fetch_one(&self.pool)
        .await
    }


    async fn get_worker_portfolio(
        &self,
        worker_id: Uuid,
    ) -> Result<Vec<WorkerPortfolio>, Error> {
        sqlx::query_as!(
            WorkerPortfolio,
            r#"
            SELECT id, worker_id, title, description, image_url, project_date, created_at
            FROM worker_portfolios 
            WHERE worker_id = $1
            ORDER BY project_date DESC NULLS LAST, created_at DESC
            "#,
            worker_id
        )
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

    sqlx::query_as!(
        Job,
        r#"
        INSERT INTO jobs 
        (employer_id, category, title, description, location_state, location_city, location_address,
        budget, estimated_duration_days, platform_fee, escrow_amount, partial_payment_allowed, 
        partial_payment_percentage, deadline) 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) 
        RETURNING 
            id, employer_id, 
            assigned_worker_id,
            category as "category: WorkerCategory", 
            title, description,
            location_state, location_city, location_address, 
            budget,
            estimated_duration_days, 
            status as "status: JobStatus", 
            payment_status as "payment_status: PaymentStatus", 
            escrow_amount, platform_fee,
            partial_payment_allowed, 
            partial_payment_percentage, 
            created_at, updated_at, 
            deadline
        "#,
        employer_id,
        category as WorkerCategory,
        title,
        description,
        location_state,
        location_city,
        location_address,
        budget_bd,
        estimated_duration_days,
        platform_fee_bd,
        escrow_fee_bd,
        partial_payment_allowed,
        partial_payment_percentage,
        deadline
    )
    .fetch_one(&self.pool)
    .await
}

    async fn get_jobs_by_location_and_category(
    &self,
    state: &str,
    category: WorkerCategory,
    status: JobStatus,
) -> Result<Vec<Job>, Error> {
    sqlx::query_as!(
        Job,
        r#"
        SELECT 
            id, employer_id, 
            assigned_worker_id,
            category as "category: WorkerCategory",
            title, description, 
            location_state, location_city, location_address, 
            budget,
            estimated_duration_days, 
            status as "status: JobStatus", 
            payment_status as "payment_status: PaymentStatus", 
            escrow_amount, platform_fee,
            partial_payment_allowed, 
            partial_payment_percentage,
            created_at, updated_at, 
            deadline
        FROM jobs 
        WHERE location_state = $1 AND category = $2 AND status = $3
        ORDER BY created_at DESC
        "#,
        state,
        category as WorkerCategory,
        status as JobStatus
    )
    .fetch_all(&self.pool)
    .await
}


   async fn get_job_by_id(&self, job_id: Uuid) -> Result<Option<Job>, Error> {
    sqlx::query_as!(
        Job,
        r#"
        SELECT 
            id, employer_id, 
            assigned_worker_id,
            category as "category: WorkerCategory",
            title, description, 
            location_state, location_city, location_address, 
            budget,
            estimated_duration_days, 
            status as "status: JobStatus", 
            payment_status as "payment_status: PaymentStatus", 
            escrow_amount, platform_fee,
            partial_payment_allowed, 
            partial_payment_percentage,
            created_at, updated_at, 
            deadline
        FROM jobs WHERE id = $1
        "#,
        job_id
    )
    .fetch_optional(&self.pool)
    .await
}

    //Not sure
    async fn update_job_status(
        &self,
        job_id: Uuid,
        status: JobStatus,
    ) -> Result<Job, Error> {
        sqlx::query_as!(
            Job,
            r#"
            UPDATE jobs 
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING id, employer_id, assigned_worker_id, category as "category: WorkerCategory",
            title, description, location_state, location_city, location_address, budget,
            estimated_duration_days, status as "status: JobStatus", 
            payment_status as "payment_status: PaymentStatus", escrow_amount, platform_fee,
            partial_payment_allowed, partial_payment_percentage, created_at, updated_at, deadline
            "#,
            job_id,
            status as JobStatus
        )
        .fetch_one(&self.pool)
        .await
    }


    // In labourdb.rs - Update assign_worker_to_job method
async fn assign_worker_to_job(
    &self,
    job_id: Uuid,
    worker_id: Uuid
) -> Result<(Job, EscrowTransaction), Error> {
    let mut tx = self.pool.begin().await?;

    // First update the job
    let job = sqlx::query_as!(
        Job,
        r#"
        UPDATE jobs 
        SET assigned_worker_id = $2, status = 'in_progress'::job_status, updated_at = NOW()
        WHERE id = $1 AND status = 'open'::job_status
        RETURNING 
            id, employer_id, 
            assigned_worker_id,
            category as "category: WorkerCategory",
            title, description, 
            location_state, location_city, location_address, 
            budget,
            estimated_duration_days, 
            status as "status: JobStatus", 
            payment_status as "payment_status: PaymentStatus", 
            escrow_amount, platform_fee,
            partial_payment_allowed, 
            partial_payment_percentage, 
            created_at, updated_at, 
            deadline
        "#,
        job_id,
        worker_id
    )
    .fetch_one(&mut *tx)
    .await?;

    // Now create the escrow transaction with the assigned worker
    let escrow = sqlx::query_as!(
        EscrowTransaction,
        r#"
        INSERT INTO escrow_transactions 
        (job_id, employer_id, worker_id, amount, platform_fee, status)
        VALUES ($1, $2, $3, $4, $5, 'escrowed'::payment_status)
        RETURNING id, job_id, employer_id, worker_id, amount, platform_fee,
        status as "status: PaymentStatus", transaction_hash, created_at, released_at
        "#,
        job_id,
        job.employer_id,
        worker_id,
        job.escrow_amount,
        job.platform_fee
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok((job, escrow))
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

        sqlx::query_as!(
            JobApplication,
            r#"
            INSERT INTO job_applications 
            (job_id, worker_id, proposed_rate, estimated_completion, cover_letter)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, job_id, worker_id, proposed_rate, estimated_completion, 
            cover_letter, status, created_at
            "#,
            job_id,
            worker_id,
            proposed_rate_bd,
            estimated_completion,
            cover_letter
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_job_applications(
        &self,
        job_id: Uuid
    ) -> Result<Vec<JobApplication>, Error> {
        sqlx::query_as!(
            JobApplication,
            r#"
            SELECT id, job_id, worker_id, proposed_rate, estimated_completion, 
            cover_letter, status, created_at
            FROM job_applications 
            WHERE job_id = $1
            ORDER BY created_at DESC
            "#,
            job_id
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn update_application_status(
        &self,
        application_id: Uuid,
        status: String,
    ) -> Result<JobApplication, Error> {
        sqlx::query_as!(
            JobApplication,
            r#"
            UPDATE job_applications 
            SET status = $2
            WHERE id = $1
            RETURNING id, job_id, worker_id, proposed_rate, estimated_completion, 
            cover_letter, status, created_at
            "#,
            application_id,
            status
        )
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

        sqlx::query_as!(
            JobContract,
            r#"
            INSERT INTO job_contracts 
            (job_id, employer_id, worker_id, agreed_rate, agreed_timeline, terms)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, job_id, employer_id, worker_id, agreed_rate, agreed_timeline,
            terms, signed_by_employer, signed_by_worker, contract_date
            "#,
            job_id,
            employer_id,
            worker_id,
            agreed_rate_bd,
            agreed_timeline,
            terms
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn sign_contract(
        &self,
        contract_id: Uuid,
        signer_role: String,
    ) -> Result<JobContract, Error> {
        if signer_role == "employer" {
            sqlx::query_as!(
                JobContract,
                r#"
                UPDATE job_contracts 
                SET signed_by_employer = true
                WHERE id = $1
                RETURNING id, job_id, employer_id, worker_id, agreed_rate, agreed_timeline,
                terms, signed_by_employer, signed_by_worker, contract_date
                "#,
                contract_id
            )
            .fetch_one(&self.pool)
            .await
        } else if signer_role == "worker" {
            sqlx::query_as!(
                JobContract,
                r#"
                UPDATE job_contracts 
                SET signed_by_worker = true
                WHERE id = $1
                RETURNING id, job_id, employer_id, worker_id, agreed_rate, agreed_timeline,
                terms, signed_by_employer, signed_by_worker, contract_date
                "#,
                contract_id
            )
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

        sqlx::query_as!(
            EscrowTransaction,
            r#"
            INSERT INTO escrow_transactions 
            (job_id, employer_id, worker_id, amount, platform_fee, status)
            VALUES ($1, $2, $3, $4, $5, 'escrowed'::payment_status)
            RETURNING id, job_id, employer_id, worker_id, amount, platform_fee,
            status as "status: PaymentStatus", transaction_hash, created_at, released_at
            "#,
            job_id,
            employer_id,
            worker_id,
            amount_bd,
            platform_fee_bd
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn update_escrow_status(
        &self,
        escrow_id: Uuid,
        status: PaymentStatus,
        transaction_hash: Option<String>,
    ) -> Result<EscrowTransaction, Error> {
        sqlx::query_as!(
            EscrowTransaction,
            r#"
            UPDATE escrow_transactions 
            SET status = $2, transaction_hash = $3
            WHERE id = $1
            RETURNING id, job_id, employer_id, worker_id, amount, platform_fee,
            status as "status: PaymentStatus", transaction_hash, created_at, released_at
            "#,
            escrow_id,
            status as PaymentStatus,
            transaction_hash
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn release_escrow_payment(
        &self,
        _escrow_id: Uuid,
        _release_percentage: f64,
    ) -> Result<EscrowTransaction, Error> {
        todo!()
    }

    async fn submit_job_progress(
        &self,
        job_id: Uuid,
        worker_id: Uuid,
        progress_percentage: i32,
        description: String,
        image_urls: Vec<String>,
    ) -> Result<JobProgress, Error> {
        sqlx::query_as!(
            JobProgress,
            r#"
            INSERT INTO job_progress 
            (job_id, worker_id, progress_percentage, description, image_urls)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, job_id, worker_id, progress_percentage, description, 
            image_urls, submitted_at
            "#,
            job_id,
            worker_id,
            progress_percentage,
            description,
            &image_urls
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_job_progress(
        &self,
        job_id: Uuid,
    ) -> Result<Vec<JobProgress>, Error> {
        sqlx::query_as!(
            JobProgress,
            r#"
            SELECT id, job_id, worker_id, progress_percentage, description, 
            image_urls, submitted_at
            FROM job_progress 
            WHERE job_id = $1
            ORDER BY submitted_at DESC
            "#,
            job_id
        )
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
        sqlx::query_as!(
            Dispute,
            r#"
            INSERT INTO disputes 
            (job_id, raised_by, against, reason, description, evidence_urls)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, job_id, raised_by, against, reason, description, 
            evidence_urls, status as "status: DisputeStatus", assigned_verifier, 
            resolution, created_at, resolved_at
            "#,
            job_id,
            raised_by,
            against,
            reason,
            description,
            &evidence_urls
        )
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
        "#,
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
        sqlx::query_as!(
            Dispute,
            r#"
            UPDATE disputes 
            SET assigned_verifier = $2, status = 'under_review'::dispute_status
            WHERE id = $1
            RETURNING id, job_id, raised_by, against, reason, description, 
            evidence_urls, status as "status: DisputeStatus", assigned_verifier, 
            resolution, created_at, resolved_at
            "#,
            dispute_id,
            verifier_id
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn resolve_dispute(
        &self,
        dispute_id: Uuid,
        resolution: String,
        decision: String,
    ) -> Result<Dispute, Error> {
        sqlx::query_as!(
            Dispute,
            r#"
            UPDATE disputes 
            SET resolution = $2, status = 'resolved'::dispute_status, resolved_at = NOW()
            WHERE id = $1
            RETURNING id, job_id, raised_by, against, reason, description, 
            evidence_urls, status as "status: DisputeStatus", assigned_verifier, 
            resolution, created_at, resolved_at
            "#,
            dispute_id,
            resolution
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_pending_verifications(
        &self,
        verifier_id: Uuid,
    ) -> Result<Vec<Dispute>, Error> {
        sqlx::query_as!(
            Dispute,
            r#"
            SELECT id, job_id, raised_by, against, reason, description, 
            evidence_urls, status as "status: DisputeStatus", assigned_verifier, 
            resolution, created_at, resolved_at
            FROM disputes 
            WHERE assigned_verifier = $1 AND status = 'under_review'::dispute_status
            ORDER BY created_at DESC
            "#,
            verifier_id
        )
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
        sqlx::query_as!(
            JobReview,
            r#"
            INSERT INTO job_reviews 
            (job_id, reviewer_id, reviewee_id, rating, comment)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, job_id, reviewer_id, reviewee_id, rating, comment, created_at
            "#,
            job_id,
            reviewer_id,
            reviewee_id,
            rating,
            comment
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_worker_reviews(
        &self,
        worker_id: Uuid,
    ) -> Result<Vec<JobReview>, Error> {
        sqlx::query_as!(
            JobReview,
            r#"
            SELECT id, job_id, reviewer_id, reviewee_id, rating, comment, created_at
            FROM job_reviews 
            WHERE reviewee_id = $1
            ORDER BY created_at DESC
            "#,
            worker_id
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn update_worker_rating(
        &self,
        worker_id: Uuid,
    ) -> Result<(), Error> {
    // Call the PostgreSQL function to update worker rating
        sqlx::query!(
            "SELECT update_worker_rating($1)",
            worker_id
        )
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
        let mut worker_points = 20; // Base points for worker
        let mut employer_points = 30; // Base points for employer
        
        // Bonus points for high ratings
        if job_rating >= 4 {
            worker_points += 10;
        }
        
        // Bonus for timely completion
        if completed_on_time {
            worker_points += 5;
            employer_points += 5;
        }

        // Award points to worker
        sqlx::query!(
            "UPDATE users SET trust_score = trust_score + $1 WHERE id = $2",
            worker_points,
            worker_id
        )
        .execute(&self.pool)
        .await?;

        // Award points to employer
        sqlx::query!(
            "UPDATE users SET trust_score = trust_score + $1 WHERE id = $2",
            employer_points,
            employer_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_jobs_by_employer_and_status(
        &self,
        employer_id: Uuid,
        status: JobStatus,
    ) -> Result<Vec<Job>, Error> {
        sqlx::query_as!(
            Job,
            r#"
            SELECT id, employer_id, assigned_worker_id, category as "category: WorkerCategory",
            title, description, location_state, location_city, location_address, budget,
            estimated_duration_days, status as "status: JobStatus", 
            payment_status as "payment_status: PaymentStatus", escrow_amount, platform_fee,
            partial_payment_allowed, partial_payment_percentage, created_at, updated_at, deadline
            FROM jobs 
            WHERE employer_id = $1 AND status = $2
            "#,
            employer_id,
            status as JobStatus
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn get_escrow_by_job_id(&self, job_id: Uuid) -> Result<Option<EscrowTransaction>, Error> {
        sqlx::query_as::<_, EscrowTransaction>(
            r#"
            SELECT id, job_id, employer_id, worker_id, amount, platform_fee,
                   status as "status: PaymentStatus", transaction_hash, created_at, released_at
            FROM escrow_transactions 
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn get_escrow_by_id(&self, escrow_id: Uuid) -> Result<Option<EscrowTransaction>, Error> {
        sqlx::query_as::<_, EscrowTransaction>(
            r#"
            SELECT id, job_id, employer_id, worker_id, amount, platform_fee,
                   status as "status: PaymentStatus", transaction_hash, created_at, released_at
            FROM escrow_transactions 
            WHERE id = $1
            "#,
        )
        .bind(escrow_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn get_employer_jobs(&self, employer_id: Uuid) -> Result<Vec<Job>, Error> {
        sqlx::query_as!(
            Job,
            r#"
            SELECT id, employer_id, assigned_worker_id, category as "category: WorkerCategory",
            title, description, location_state, location_city, location_address, budget,
            estimated_duration_days, status as "status: JobStatus", 
            payment_status as "payment_status: PaymentStatus", escrow_amount, platform_fee,
            partial_payment_allowed, partial_payment_percentage, created_at, updated_at, deadline
            FROM jobs 
            WHERE employer_id = $1
            ORDER BY created_at DESC
            "#,
            employer_id
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn get_employer_active_contracts(&self, employer_id: Uuid) -> Result<Vec<JobContract>, Error> {
        sqlx::query_as!(
            JobContract,
            r#"
            SELECT jc.*
            FROM job_contracts jc
            JOIN jobs j ON jc.job_id = j.id
            WHERE jc.employer_id = $1 AND j.status IN ('in_progress', 'under_review')
            ORDER BY jc.contract_date DESC
            "#,
            employer_id
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn get_employer_pending_applications(&self, employer_id: Uuid) -> Result<Vec<JobApplication>, Error> {
        sqlx::query_as!(
            JobApplication,
            r#"
            SELECT ja.*
            FROM job_applications ja
            JOIN jobs j ON ja.job_id = j.id
            WHERE j.employer_id = $1 AND ja.status = 'pending'
            ORDER BY ja.created_at DESC
            "#,
            employer_id
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn get_job_application_by_id(&self, application_id: Uuid) -> Result<Option<JobApplication>, Error> {
        sqlx::query_as!(
            JobApplication,
            r#"
            SELECT id, job_id, worker_id, proposed_rate, estimated_completion, 
            cover_letter, status, created_at
            FROM job_applications 
            WHERE id = $1
            "#,
            application_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    async fn get_worker_active_jobs(&self, worker_id: Uuid) -> Result<Vec<Job>, Error> {
        sqlx::query_as!(
            Job,
            r#"
            SELECT id, employer_id, assigned_worker_id, category as "category: WorkerCategory",
            title, description, location_state, location_city, location_address, budget,
            estimated_duration_days, status as "status: JobStatus", 
            payment_status as "payment_status: PaymentStatus", escrow_amount, platform_fee,
            partial_payment_allowed, partial_payment_percentage, created_at, updated_at, deadline
            FROM jobs 
            WHERE assigned_worker_id = $1 AND status IN ('in_progress', 'under_review')
            ORDER BY created_at DESC
            "#,
            worker_id
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn get_worker_pending_applications(&self, worker_id: Uuid) -> Result<Vec<JobApplication>, Error> {
        sqlx::query_as!(
            JobApplication,
            r#"
            SELECT ja.*
            FROM job_applications ja
            JOIN worker_profiles wp ON ja.worker_id = wp.id
            WHERE wp.user_id = $1 AND ja.status = 'pending'
            ORDER BY ja.created_at DESC
            "#,
            worker_id
        )
        .fetch_all(&self.pool)
        .await
    }
}