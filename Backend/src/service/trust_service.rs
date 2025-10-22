// services/trust_service.rs
use std::sync::Arc;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::Serialize;

use crate::{
    db::{
        db::DBClient,
        labourdb::LaborExt,
    },
    service::error::ServiceError,
    models::labourmodel::*,

};

#[derive(Debug, Clone)]
pub struct TrustService {
    db_client: Arc<DBClient>,
}

impl TrustService {
    pub fn new(db_client: Arc<DBClient>) -> Self {
        Self { db_client }
    }

    pub async fn award_job_completion_points(
        &self,
        worker_id: Uuid,
        employer_id: Uuid,
        job_id: Uuid,
        job_rating: i32,
        completed_on_time: bool,
    ) -> Result<(), ServiceError> {
        let mut tx = self.db_client.pool.begin().await?;

        let mut worker_points = 20; // Base points for worker
        let mut employer_points = 30; // Base points for employer
        
        // Bonus points for high ratings
        if job_rating >= 4 {
            worker_points += 10;
            employer_points += 5; // Employer gets points for giving good rating
        }
        
        // Bonus for timely completion
        if completed_on_time {
            worker_points += 5;
            employer_points += 5;
        }

        // // Award points to worker
        // sqlx::query!(
        //     "UPDATE users SET trust_score = trust_score + $1 WHERE id = $2",
        //     worker_points,
        //     worker_id
        // )
        // .execute(&mut *tx)
        // .await?;

        // // Award points to employer
        // sqlx::query!(
        //     "UPDATE users SET trust_score = trust_score + $1 WHERE id = $2",
        //     employer_points,
        //     employer_id
        // )
        // .execute(&mut *tx)
        // .await?;

        // Award points to worker
        sqlx::query(
            "UPDATE users SET trust_score = trust_score + $1 WHERE id = $2"
        )
        .bind(worker_points)
        .bind(worker_id)
        .execute(&mut *tx)
        .await?;

        // Award points to employer
        sqlx::query(
            "UPDATE users SET trust_score = trust_score + $1 WHERE id = $2"
        )
        .bind(employer_points)
        .bind(employer_id)
        .execute(&mut *tx)
        .await?;

        // Log trust events
        self.log_trust_event(
            worker_id,
            "job_completion".to_string(),
            worker_points,
            format!("Completed job {}", job_id),
            &mut tx,
        ).await?;

        self.log_trust_event(
            employer_id,
            "job_completion".to_string(),
            employer_points,
            format!("Employer for completed job {}", job_id),
            &mut tx,
        ).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn deduct_trust_points(
        &self,
        user_id: Uuid,
        points: i32,
        reason: String,
    ) -> Result<(), ServiceError> {
        let mut tx = self.db_client.pool.begin().await?;

        // // Ensure trust score doesn't go below 0
        // sqlx::query!(
        //     "UPDATE users SET trust_score = GREATEST(0, trust_score - $1) WHERE id = $2",
        //     points,
        //     user_id
        // )
        // .execute(&mut *tx)
        // .await?;

        // Ensure trust score doesn't go below 0
        sqlx::query(
            "UPDATE users SET trust_score = GREATEST(0, trust_score - $1) WHERE id = $2"
        )
        .bind(points)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Log trust event (negative points)
        self.log_trust_event(
            user_id,
            "penalty".to_string(),
            -points,
            reason,
            &mut tx,
        ).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn calculate_trust_score(
        &self,
        user_id: Uuid,
    ) -> Result<TrustScore, ServiceError> {
        // Get user's jobs and reviews to calculate multi-dimensional trust score
        let user_jobs = self.get_user_jobs(user_id).await?;
        let user_reviews = self.get_user_reviews(user_id).await?;
        let user_disputes = self.get_user_disputes(user_id).await?;

        let dimensions = TrustDimensions {
            completion_rate: self.calculate_completion_rate(&user_jobs),
            on_time_delivery: self.calculate_on_time_rate(&user_jobs),
            communication: self.calculate_communication_score(&user_reviews),
            quality_of_work: self.calculate_quality_score(&user_reviews),
            dispute_resolution: self.calculate_dispute_score(&user_disputes),
        };

        let overall_score = dimensions.calculate_overall();

        Ok(TrustScore {
            user_id,
            overall_score,
            dimensions,
            history: vec![], // Would be populated from trust_events table
        })
    }

    // async fn log_trust_event(
    //     &self,
    //     user_id: Uuid,
    //     category: String,
    //     points: i32,
    //     reason: String,
    //     tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    // ) -> Result<(), ServiceError> {
    //     sqlx::query!(
    //         r#"
    //         INSERT INTO trust_events (user_id, category, points, reason, created_at)
    //         VALUES ($1, $2, $3, $4, NOW())
    //         "#,
    //         user_id,
    //         category,
    //         points,
    //         reason
    //     )
    //     .execute(&mut **tx)
    //     .await?;

    //     Ok(())
    // }

    // async fn get_user_jobs(&self, user_id: Uuid) -> Result<Vec<UserJob>, ServiceError> {
    //     // Get jobs where user is either employer or worker
    //     let jobs = sqlx::query_as!(
    //         UserJob,
    //         r#"
    //         SELECT 
    //             j.id,
    //             j.employer_id,
    //             j.assigned_worker_id,
    //             j.status as "status: JobStatus",
    //             j.created_at,
    //             j.updated_at,
    //             j.deadline,
    //             jp.progress_percentage,
    //             jp.submitted_at as last_progress_date
    //         FROM jobs j
    //         LEFT JOIN (
    //             SELECT job_id, progress_percentage, submitted_at,
    //             ROW_NUMBER() OVER (PARTITION BY job_id ORDER BY submitted_at DESC) as rn
    //             FROM job_progress
    //         ) jp ON j.id = jp.job_id AND jp.rn = 1
    //         WHERE j.employer_id = $1 OR j.assigned_worker_id = $1
    //         "#,
    //         user_id
    //     )
    //     .fetch_all(&self.db_client.pool)
    //     .await?;

    //     Ok(jobs)
    // }


    async fn log_trust_event(
        &self,
        user_id: Uuid,
        category: String,
        points: i32,
        reason: String,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), ServiceError> {
        sqlx::query(
            r#"
            INSERT INTO trust_events (user_id, category, points, reason, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            "#
        )
        .bind(user_id)
        .bind(category)
        .bind(points)
        .bind(reason)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn get_user_jobs(&self, user_id: Uuid) -> Result<Vec<UserJob>, ServiceError> {
        // Get jobs where user is either employer or worker
        let jobs = sqlx::query_as::<_, UserJob>(
            r#"
            SELECT 
                j.id,
                j.employer_id,
                j.assigned_worker_id,
                j.status,
                j.created_at,
                j.updated_at,
                j.deadline,
                jp.progress_percentage,
                jp.submitted_at as last_progress_date
            FROM jobs j
            LEFT JOIN (
                SELECT job_id, progress_percentage, submitted_at,
                ROW_NUMBER() OVER (PARTITION BY job_id ORDER BY submitted_at DESC) as rn
                FROM job_progress
            ) jp ON j.id = jp.job_id AND jp.rn = 1
            WHERE j.employer_id = $1 OR j.assigned_worker_id = $1
            "#
        )
        .bind(user_id)
        .fetch_all(&self.db_client.pool)
        .await?;

        Ok(jobs)
    }

    async fn get_user_reviews(&self, user_id: Uuid) -> Result<Vec<JobReview>, ServiceError> {
        self.db_client.get_worker_reviews(user_id).await
            .map_err(ServiceError::from)
    }

    // async fn get_user_disputes(&self, user_id: Uuid) -> Result<Vec<UserDispute>, ServiceError> {
    //     let disputes = sqlx::query_as!(
    //         UserDispute,
    //         r#"
    //         SELECT 
    //             d.id,
    //             d.raised_by,
    //             d.against,
    //             d.status as "status: DisputeStatus",
    //             d.resolution,
    //             d.created_at,
    //             d.resolved_at
    //         FROM disputes d
    //         WHERE d.raised_by = $1 OR d.against = $1
    //         "#,
    //         user_id
    //     )
    //     .fetch_all(&self.db_client.pool)
    //     .await?;

    async fn get_user_disputes(&self, user_id: Uuid) -> Result<Vec<UserDispute>, ServiceError> {
        let disputes = sqlx::query_as::<_, UserDispute>(
        r#"
        SELECT 
            d.id,
            d.raised_by,
            d.against,
            d.status,
            d.resolution,
            d.created_at,
            d.resolved_at
        FROM disputes d
        WHERE d.raised_by = $1 OR d.against = $1
        "#
    )
    .bind(user_id)
    .fetch_all(&self.db_client.pool)
    .await?;

    Ok(disputes)
    }
      

    fn calculate_completion_rate(&self, jobs: &[UserJob]) -> f32 {
        if jobs.is_empty() {
            return 0.0;
        }

        let completed = jobs.iter()
            .filter(|job| job.status == Some(JobStatus::Completed))
            .count();

        (completed as f32 / jobs.len() as f32) * 100.0
    }

    fn calculate_on_time_rate(&self, jobs: &[UserJob]) -> f32 {
        let completed_jobs: Vec<&UserJob> = jobs.iter()
            .filter(|job| job.status == Some(JobStatus::Completed))
            .collect();

        if completed_jobs.is_empty() {
            return 0.0;
        }

        let on_time = completed_jobs.iter()
            .filter(|job| {
                // Simple check: if last progress was before deadline
                if let (Some(deadline), Some(last_progress)) = (job.deadline, job.last_progress_date) {
                    last_progress <= deadline
                } else {
                    false
                }
            })
            .count();

        (on_time as f32 / completed_jobs.len() as f32) * 100.0
    }

    fn calculate_communication_score(&self, reviews: &[JobReview]) -> f32 {
        if reviews.is_empty() {
            return 0.0;
        }

        // Simple average for now - in reality, you'd analyze review comments
        let total_rating: i32 = reviews.iter().map(|r| r.rating).sum();
        (total_rating as f32 / reviews.len() as f32) * 20.0 // Convert to percentage
    }

    fn calculate_quality_score(&self, reviews: &[JobReview]) -> f32 {
        // For now, same as communication score - could be enhanced with sentiment analysis
        self.calculate_communication_score(reviews)
    }

    fn calculate_dispute_score(&self, disputes: &[UserDispute]) -> f32 {
        if disputes.is_empty() {
            return 100.0; // No disputes = perfect score
        }

        let resolved_favorably = disputes.iter()
            .filter(|d| {
                d.status == Some(DisputeStatus::Resolved) &&
                d.resolution.as_ref().map_or(false, |r| r.contains("favor"))
            })
            .count();

        (resolved_favorably as f32 / disputes.len() as f32) * 100.0
    }
}

#[derive(Debug, Serialize)]
pub struct TrustScore {
    pub user_id: Uuid,
    pub overall_score: f32,
    pub dimensions: TrustDimensions,
    pub history: Vec<TrustEvent>,
}

#[derive(Debug, Serialize)]
pub struct TrustDimensions {
    pub completion_rate: f32,
    pub on_time_delivery: f32,
    pub communication: f32,
    pub quality_of_work: f32,
    pub dispute_resolution: f32,
}

impl TrustDimensions {
    pub fn calculate_overall(&self) -> f32 {
        // Weighted average of all dimensions
        (self.completion_rate * 0.3 +
         self.on_time_delivery * 0.25 +
         self.quality_of_work * 0.25 +
         self.communication * 0.1 +
         self.dispute_resolution * 0.1)
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TrustEvent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category: String,
    pub points: i32,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct UserJob {
    pub id: Uuid,
    pub employer_id: Uuid,
    pub assigned_worker_id: Option<Uuid>,
    pub status: Option<JobStatus>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deadline: Option<DateTime<Utc>>,
    pub progress_percentage: Option<i32>,
    pub last_progress_date: Option<DateTime<Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct UserDispute {
    pub id: Uuid,
    pub raised_by: Uuid,
    pub against: Uuid,
    pub status: Option<DisputeStatus>,
    pub resolution: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}