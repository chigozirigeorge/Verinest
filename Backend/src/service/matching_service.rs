// services/matching_service.rs
use std::sync::Arc;
use uuid::Uuid;
use serde::Serialize;
use num_traits::ToPrimitive;


use crate::{
    db::{
        db::DBClient,
        labourdb::LaborExt,
    },
    models::labourmodel::*,
    service::error::ServiceError,

};

#[derive(Debug, Clone)]
pub struct MatchingService {
    db_client: Arc<DBClient>,
}

impl MatchingService {
    pub fn new(db_client: Arc<DBClient>) -> Self {
        Self { db_client }
    }

    pub async fn find_best_workers_for_job(
        &self,
        job: &Job,
        limit: usize,
    ) -> Result<Vec<WorkerMatch>, ServiceError> {
        // Get potential workers by location and category
        let potential_workers = self.db_client
            .get_workers_by_location_and_category(
                &job.location_state,
                job.category,
                (limit * 3) as i64, // Get more for filtering
                0,
            )
            .await?;

        // Score and rank workers
        let mut scored_workers: Vec<WorkerMatch> = potential_workers
            .into_iter()
            .filter_map(|worker| self.score_worker_for_job(&worker, job).ok())
            .collect();

        // Sort by match score (highest first)
        scored_workers.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Take top matches
        scored_workers.truncate(limit);

        Ok(scored_workers)
    }

    pub async fn find_relevant_jobs_for_worker(
        &self,
        worker_profile: &WorkerProfile,
        limit: usize,
    ) -> Result<Vec<JobMatch>, ServiceError> {
        // Get open jobs in worker's location and category
        let potential_jobs = self.db_client
            .get_jobs_by_location_and_category(
                &worker_profile.location_state,
                worker_profile.category,
                JobStatus::Open,
            )
            .await?;

        // Score and rank jobs
        let mut scored_jobs: Vec<JobMatch> = potential_jobs
            .into_iter()
            .filter_map(|job| self.score_job_for_worker(&job, worker_profile).ok())
            .collect();

        // Sort by match score (highest first)
        scored_jobs.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Take top matches
        scored_jobs.truncate(limit);

        Ok(scored_jobs)
    }

    fn score_worker_for_job(
        &self,
        worker: &WorkerProfile,
        job: &Job,
    ) -> Result<WorkerMatch, ServiceError> {
        let mut score = 0.0;
        let mut match_reasons = Vec::new();

        // Location proximity (40% of score)
        if worker.location_state == job.location_state {
            score += 30.0;
            match_reasons.push("Same state".to_string());
            
            if worker.location_city == job.location_city {
                score += 10.0;
                match_reasons.push("Same city".to_string());
            }
        }

        // Experience match (20% of score)
        let experience_score = (worker.experience_years as f32 / 10.0).min(10.0) * 2.0;
        score += experience_score;
        if experience_score > 0.0 {
            match_reasons.push(format!("{} years experience", worker.experience_years));
        }

        // Rating score (20% of score)
        if let Some(rating) = worker.rating {
            let rating_score = rating * 4.0; // 5-star rating = 20 points
            score += rating_score;
            if rating >= 4.0 {
                match_reasons.push("High rating".to_string());
            }
        }

        // Completed jobs (10% of score)
        if let Some(completed) = worker.completed_jobs {
            let completion_score = (completed as f32 / 50.0).min(10.0);
            score += completion_score;
            if completed > 10 {
                match_reasons.push("Experienced worker".to_string());
            }
        }

        // Availability bonus (10% of score)
        if worker.is_available.unwrap_or(false) {
            score += 10.0;
            match_reasons.push("Available now".to_string());
        }

        // Budget compatibility check
        if let (Some(worker_daily_rate), Some(job_budget)) = (worker.daily_rate.clone(), job.budget.to_f64()) {
            let worker_rate = worker_daily_rate.to_f64().unwrap_or(0.0);
            let job_budget_value = job_budget;
            
            if worker_rate <= job_budget_value * 1.2 {
                // Worker's rate is within 20% of job budget
                score += 5.0;
                match_reasons.push("Budget compatible".to_string());
            }
        }

        Ok(WorkerMatch {
            worker: worker.clone(),
            score: score.min(100.0), // Cap at 100
            match_reasons,
        })
    }

    fn score_job_for_worker(
        &self,
        job: &Job,
        worker: &WorkerProfile,
    ) -> Result<JobMatch, ServiceError> {
        let mut score: f32 = 0.0;
        let mut match_reasons = Vec::new();

        // Location match (30% of score)
        if worker.location_state == job.location_state {
            score += 20.0;
            match_reasons.push("Same state".to_string());
            
            if worker.location_city == job.location_city {
                score += 10.0;
                match_reasons.push("Same city".to_string());
            }
        }

        // Category match (20% of score) - already filtered, but still important
        score += 20.0;
        match_reasons.push("Category match".to_string());

        // Budget compatibility (25% of score)
        if let (Some(worker_daily_rate), Some(job_budget)) = (worker.daily_rate.clone(), job.budget.to_f64()) {
            let worker_rate = worker_daily_rate.to_f64().unwrap_or(0.0);
            let job_budget_value = job_budget;
            
            if worker_rate <= job_budget_value {
                score += 20.0;
                match_reasons.push("Good budget".to_string());
            } else if worker_rate <= job_budget_value * 1.2 {
                score += 10.0;
                match_reasons.push("Reasonable budget".to_string());
            }
            
            // Bonus for high-budget jobs
            if job_budget_value > 10000.0 {
                score += 5.0;
                match_reasons.push("High budget job".to_string());
            }
        }

        // Job duration compatibility (15% of score)
        if job.estimated_duration_days <= 30 {
            score += 15.0;
            match_reasons.push("Short duration".to_string());
        } else if job.estimated_duration_days <= 90 {
            score += 10.0;
            match_reasons.push("Medium duration".to_string());
        } else {
            score += 5.0;
            match_reasons.push("Long term project".to_string());
        }

        // Employer trust score (10% of score)
        // This would require fetching employer data - simplified for now
        score += 10.0;

        Ok(JobMatch {
            job: job.clone(),
            score: score.min(100.0),  // Type inference now works since score is explicitly f32
            match_reasons,
        })
    }

    pub async fn get_smart_recommendations(
        &self,
        user_id: Uuid,
        user_type: UserType,
        limit: usize,
    ) -> Result<Recommendations, ServiceError> {
        match user_type {
            UserType::Worker => {
                let worker_profile = self.db_client.get_worker_profile(user_id).await?;
                let job_matches = self.find_relevant_jobs_for_worker(&worker_profile, limit).await?;
                
                Ok(Recommendations::Worker {
                    profile: worker_profile,
                    recommended_jobs: job_matches,
                })
            }
            UserType::Employer => {
                // For employers, recommend workers for their open jobs
                let open_jobs = self.db_client
                    .get_jobs_by_employer_and_status(user_id, JobStatus::Open)
                    .await?;

                let mut worker_recommendations = Vec::new();
                
                for job in open_jobs {
                    let workers = self.find_best_workers_for_job(&job, 3).await?;
                    worker_recommendations.push(JobWorkerRecommendation {
                        job,
                        recommended_workers: workers,
                    });
                }

                Ok(Recommendations::Employer {
                    worker_recommendations,
                })
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WorkerMatch {
    pub worker: WorkerProfile,
    pub score: f32,
    pub match_reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct JobMatch {
    pub job: Job,
    pub score: f32,
    pub match_reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct JobWorkerRecommendation {
    pub job: Job,
    pub recommended_workers: Vec<WorkerMatch>,
}

#[derive(Debug, Serialize)]
pub enum Recommendations {
    Worker {
        profile: WorkerProfile,
        recommended_jobs: Vec<JobMatch>,
    },
    Employer {
        worker_recommendations: Vec<JobWorkerRecommendation>,
    },
}

#[derive(Debug)]
pub enum UserType {
    Worker,
    Employer,
}
