
-- migrations/labor_platform_schema.sql

-- Create enums for labor platform
CREATE TYPE worker_category AS ENUM (
    'painter', 'plumber', 'electrician', 'carpenter', 'mason', 
    'tiler', 'roofer', 'interior_decorator', 'landscaper', 
    'cleaner', 'security_guard', 'other'
);

CREATE TYPE job_status AS ENUM (
    'open', 'in_progress', 'under_review', 'completed', 'disputed', 'cancelled'
);

CREATE TYPE payment_status AS ENUM (
    'pending', 'escrowed', 'partially_paid', 'completed', 'refunded'
);

CREATE TYPE dispute_status AS ENUM (
    'open', 'under_review', 'resolved', 'escalated'
);

-- Worker profiles table
CREATE TABLE worker_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category worker_category NOT NULL,
    experience_years INTEGER NOT NULL DEFAULT 0,
    description TEXT NOT NULL,
    hourly_rate DECIMAL(10,2),
    daily_rate DECIMAL(10,2),
    location_state VARCHAR(100) NOT NULL,
    location_city VARCHAR(100) NOT NULL,
    is_available BOOLEAN DEFAULT TRUE,
    rating REAL DEFAULT 0.0,
    completed_jobs INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Worker portfolio table
CREATE TABLE worker_portfolios (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id UUID NOT NULL REFERENCES worker_profiles(id) ON DELETE CASCADE,
    title VARCHAR(100) NOT NULL,
    description TEXT NOT NULL,
    image_url TEXT NOT NULL,
    project_date TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Jobs table
CREATE TABLE jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    employer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    assigned_worker_id UUID REFERENCES worker_profiles(id) ON DELETE SET NULL,
    category worker_category NOT NULL,
    title VARCHAR(200) NOT NULL,
    description TEXT NOT NULL,
    location_state VARCHAR(100) NOT NULL,
    location_city VARCHAR(100) NOT NULL,
    location_address TEXT NOT NULL,
    budget DECIMAL(12,2) NOT NULL,
    estimated_duration_days INTEGER NOT NULL,
    status job_status DEFAULT 'open',
    payment_status payment_status DEFAULT 'pending',
    escrow_amount DECIMAL(12,2) NOT NULL,
    platform_fee DECIMAL(12,2) NOT NULL,
    partial_payment_allowed BOOLEAN DEFAULT FALSE,
    partial_payment_percentage INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    deadline TIMESTAMPTZ,
    CONSTRAINT valid_partial_payment_percentage 
        CHECK (partial_payment_percentage IS NULL OR (partial_payment_percentage >= 10 AND partial_payment_percentage <= 90))
);

-- Job applications table
CREATE TABLE job_applications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    worker_id UUID NOT NULL REFERENCES worker_profiles(id) ON DELETE CASCADE,
    proposed_rate DECIMAL(12,2) NOT NULL,
    estimated_completion INTEGER NOT NULL,
    cover_letter TEXT NOT NULL,
    status VARCHAR(20) DEFAULT 'applied', -- applied, shortlisted, rejected, accepted
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(job_id, worker_id)
);

-- Job contracts table
CREATE TABLE job_contracts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    employer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    worker_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    agreed_rate DECIMAL(12,2) NOT NULL,
    agreed_timeline INTEGER NOT NULL,
    terms TEXT NOT NULL,
    signed_by_employer BOOLEAN DEFAULT FALSE,
    signed_by_worker BOOLEAN DEFAULT FALSE,
    contract_date TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(job_id)
);

-- Escrow transactions table
CREATE TABLE escrow_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    employer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    worker_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount DECIMAL(12,2) NOT NULL,
    platform_fee DECIMAL(12,2) NOT NULL,
    status payment_status DEFAULT 'pending',
    transaction_hash VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    released_at TIMESTAMPTZ,
    UNIQUE(job_id)
);

-- Job progress tracking table
CREATE TABLE job_progress (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    worker_id UUID NOT NULL REFERENCES worker_profiles(id) ON DELETE CASCADE,
    progress_percentage INTEGER NOT NULL CHECK (progress_percentage >= 0 AND progress_percentage <= 100),
    description TEXT NOT NULL,
    image_urls TEXT[] NOT NULL DEFAULT '{}',
    submitted_at TIMESTAMPTZ DEFAULT NOW()
);

-- Job reviews table
CREATE TABLE job_reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    reviewer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reviewee_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    comment TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(job_id, reviewer_id)
);

-- Disputes table
CREATE TABLE disputes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    raised_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    against UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason VARCHAR(200) NOT NULL,
    description TEXT NOT NULL,
    evidence_urls TEXT[] DEFAULT '{}',
    status dispute_status DEFAULT 'open',
    assigned_verifier UUID REFERENCES users(id) ON DELETE SET NULL,
    resolution TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

-- Verification tasks table (for dispute resolution)
CREATE TABLE verification_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dispute_id UUID NOT NULL REFERENCES disputes(id) ON DELETE CASCADE,
    verifier_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(20) DEFAULT 'pending', -- pending, in_review, completed
    notes TEXT,
    decision VARCHAR(50), -- favor_employer, favor_worker, partial_payment
    created_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    UNIQUE(dispute_id)
);

-- Trust points tracking for labor activities
CREATE TABLE trust_point_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    job_id UUID REFERENCES jobs(id) ON DELETE SET NULL,
    points INTEGER NOT NULL,
    transaction_type VARCHAR(50) NOT NULL, -- job_completion, quality_bonus, timely_completion, dispute_resolution
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_worker_profiles_user_id ON worker_profiles(user_id);
CREATE INDEX idx_worker_profiles_category_location ON worker_profiles(category, location_state, is_available);
CREATE INDEX idx_worker_profiles_rating ON worker_profiles(rating DESC, completed_jobs DESC);

CREATE INDEX idx_worker_portfolios_worker_id ON worker_portfolios(worker_id);

CREATE INDEX idx_jobs_employer_id ON jobs(employer_id);
CREATE INDEX idx_jobs_assigned_worker_id ON jobs(assigned_worker_id);
CREATE INDEX idx_jobs_category_location ON jobs(category, location_state, status);
CREATE INDEX idx_jobs_status ON jobs(status, created_at DESC);

CREATE INDEX idx_job_applications_job_id ON job_applications(job_id);
CREATE INDEX idx_job_applications_worker_id ON job_applications(worker_id);

CREATE INDEX idx_job_contracts_job_id ON job_contracts(job_id);

CREATE INDEX idx_escrow_transactions_job_id ON escrow_transactions(job_id);
CREATE INDEX idx_escrow_transactions_status ON escrow_transactions(status);

CREATE INDEX idx_job_progress_job_id ON job_progress(job_id, submitted_at DESC);

CREATE INDEX idx_job_reviews_reviewee_id ON job_reviews(reviewee_id);

CREATE INDEX idx_disputes_job_id ON disputes(job_id);
CREATE INDEX idx_disputes_status ON disputes(status, created_at DESC);
CREATE INDEX idx_disputes_verifier ON disputes(assigned_verifier, status);

CREATE INDEX idx_verification_tasks_verifier ON verification_tasks(verifier_id, status);

CREATE INDEX idx_trust_transactions_user_id ON trust_point_transactions(user_id, created_at DESC);

-- Create triggers for updating timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_worker_profiles_updated_at 
    BEFORE UPDATE ON worker_profiles 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_jobs_updated_at 
    BEFORE UPDATE ON jobs 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to update worker rating based on reviews
CREATE OR REPLACE FUNCTION update_worker_rating(worker_user_id UUID)
RETURNS VOID AS $$
DECLARE
    avg_rating REAL;
    job_count INTEGER;
BEGIN
    -- Calculate average rating from reviews where user is the reviewee
    SELECT 
        COALESCE(AVG(rating), 0.0),
        COUNT(*)
    INTO avg_rating, job_count
    FROM job_reviews 
    WHERE reviewee_id = worker_user_id;
    
    -- Update worker profile
    UPDATE worker_profiles 
    SET 
        rating = avg_rating,
        completed_jobs = job_count,
        updated_at = NOW()
    WHERE user_id = worker_user_id;
END;
$$ LANGUAGE plpgsql;

-- Function to calculate platform fee (2% of job budget)
CREATE OR REPLACE FUNCTION calculate_platform_fee(budget DECIMAL)
RETURNS DECIMAL AS $$
BEGIN
    RETURN budget * 0.02;
END;
$$ LANGUAGE plpgsql;

-- Function to award trust points for job completion
CREATE OR REPLACE FUNCTION award_job_completion_points(
    worker_user_id UUID,
    employer_user_id UUID,
    job_rating INTEGER DEFAULT 3,
    completed_on_time BOOLEAN DEFAULT TRUE,
    job_ref UUID DEFAULT NULL
)
RETURNS VOID AS $$
DECLARE
    worker_points INTEGER := 20;
    employer_points INTEGER := 30;
BEGIN
    -- Bonus points for high ratings
    IF job_rating >= 4 THEN
        worker_points := worker_points + 10;
    END IF;
    
    -- Bonus for timely completion
    IF completed_on_time THEN
        worker_points := worker_points + 5;
        employer_points := employer_points + 5;
    END IF;
    
    -- Award points to worker
    UPDATE users SET trust_score = trust_score + worker_points WHERE id = worker_user_id;
    INSERT INTO trust_point_transactions (user_id, job_id, points, transaction_type, description)
    VALUES (worker_user_id, job_ref, worker_points, 'job_completion', 'Points awarded for job completion');
    
    -- Award points to employer
    UPDATE users SET trust_score = trust_score + employer_points WHERE id = employer_user_id;
    INSERT INTO trust_point_transactions (user_id, job_id, points, transaction_type, description)
    VALUES (employer_user_id, job_ref, employer_points, 'job_completion', 'Points awarded for job posting and completion');
END;
$$ LANGUAGE plpgsql;

-- Add labor-specific constraints and checks
ALTER TABLE users ADD CONSTRAINT valid_user_role_for_labor 
    CHECK (role IN ('super_admin', 'admin', 'moderator', 'verifier', 'lawyer', 'agent', 'landlord', 'whistleblower', 'customer_care', 'dev', 'user'));

-- Add wallet verification nonces table if not exists (from your existing code)
CREATE TABLE IF NOT EXISTS wallet_verification_nonces (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    nonce BIGINT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create user_wallets table if not exists (from your existing code)
CREATE TABLE IF NOT EXISTS user_wallets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    wallet_address VARCHAR(255) NOT NULL,
    wallet_type VARCHAR(50) DEFAULT 'primary',
    blockchain VARCHAR(50) DEFAULT 'ethereum',
    is_verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, wallet_type)
);