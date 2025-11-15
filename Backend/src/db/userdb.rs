// db/userdb.rs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::Row;

use super::db::DBClient;

use crate::models::{
    referralmodel::{Referral, ReferralStats, ReferralUser}, usermodel::{User, UserRole, VerificationStatus, VerificationType}, verificationmodels::*, walletmodels::{UserWallet, WalletUpdateRequest}
};

use crate::db::verificationdb::VerificationExt;


#[async_trait]
pub trait UserExt {
    async fn update_user_verification_token<T: Into<String> + Send>(
        &self,
        user_id: Uuid,
        verification_token: T,
        token_expires_at: DateTime<Utc>,
    ) -> Result<User, sqlx::Error>;

    async fn get_user(
        &self,
        user_id: Option<Uuid>,
        username: Option<&str>,
        email: Option<&str>,
        token: Option<&str>,
    ) -> Result<Option<User>, sqlx::Error>;

    async fn get_users(
        &self,
        page: u32,
        limit: usize,
    ) -> Result<Vec<User>, sqlx::Error>;

    async fn save_user<T: Into<String> + Send>(
        &self,
        name: T,
        username: T,
        email: T,
        password: T,
        verification_token: T,
        token_expires_at: DateTime<Utc>,
    ) -> Result<User, sqlx::Error>;

    async fn get_user_count(&self) -> Result<i64, sqlx::Error>;

    async fn update_user_name<T: Into<String> + Send>(
        &self,
        user_id: Uuid,
        name: T,
    ) -> Result<User, sqlx::Error>;

    async fn update_user_role(
        &self,
        target_id: Uuid,
        role: UserRole,
    ) -> Result<User, sqlx::Error>;

    async fn update_user_password(
        &self,
        user_id: Uuid,
        password: String,
    ) -> Result<User, sqlx::Error>;

    async fn update_transaction_pin(
        &self,
        user_id: Uuid,
        transaction_pin: i32,
    ) -> Result<User, sqlx::Error>;

    async fn update_transaction_pin_hash(
        &self,
        user_id: Uuid,
        transaction_pin_hash: &str,
    ) -> Result<User, sqlx::Error>;

    async fn verifed_token(
        &self,
        token: &str,
    ) -> Result<(), sqlx::Error>;

    async fn add_verifed_token(
        &self,
        user_id: Uuid,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error>;

    async fn update_trust_point(
        &self,
        user_id: Uuid,
        score_to_add: i32
    ) -> Result<User, sqlx::Error>;

    async fn get_user_by_identifier(
        &self, 
        identifier: &str
    ) -> Result<Option<User>, sqlx::Error>;

    async fn get_users_by_trustscore(
        &self,
        limit: i64,
    ) -> Result<Vec<User>, sqlx::Error>;

    async fn get_user_by_referral_code(   
        &self,
        referral_code: &str,
    ) -> Result<Option<User>, sqlx::Error>;
    
    async fn update_user_referral_code(
        &self,
        user_id: Uuid,
        referral_code: String,
    ) -> Result<User, sqlx::Error>;
    
    async fn add_referral_points(
        &self,
        referrer_id: Uuid,
        points: i32,
    ) -> Result<User, sqlx::Error>;

    async fn create_referral_record(
        &self,
        referrer_id: Uuid,  
        referee_id: Uuid,
        points: i32
    ) -> Result<Referral, sqlx::Error>;

    async fn increment_referral_count(
        &self,
        user_id: Uuid,
    ) -> Result<User, sqlx::Error>;
    
    async fn get_user_referral_stats(
        &self,
        user_id: Uuid,
    ) -> Result<ReferralStats, sqlx::Error>;
    
    async fn get_referral_by_referee(
        &self,
        referee_id: Uuid,
    ) -> Result<Option<Referral>, sqlx::Error>;

    //oauth methods
    async fn get_user_by_google_id(
        &self,
        google_id: &str,
    ) -> Result<Option<User>, sqlx::Error>;

    async fn create_oauth_user(
        &self,
        name: String,
        username: String,
        google_id: String,
        avatar_url: Option<String>,
        trust_points: i32,
    ) -> Result<User, sqlx::Error>;

    async fn link_google_account(
        &self,
        user_id: Uuid,
        google_id: &str,
        avatar_url: Option<&str>,
    ) -> Result<User, sqlx::Error>;

    //wallets methods
    async fn update_user_wallet(
        &self,
        user_id: Uuid,
        wallet_address: String,
    ) -> Result<User, sqlx::Error>;

    async fn add_user_wallet(
        &self,
        user_id: Uuid,
        wallet_request: WalletUpdateRequest,
    ) -> Result<UserWallet, sqlx::Error>;

    async fn get_user_wallets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserWallet>, sqlx::Error>;

    async fn get_primary_wallet(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserWallet>, sqlx::Error>;

    async fn verify_wallet(
        &self,
        user_id: Uuid,
        wallet_address: String,
    ) -> Result<UserWallet, sqlx::Error>;

    async fn get_user_by_wallet_address(
        &self,
        wallet_address: &str,
    ) -> Result<Option<User>, sqlx::Error>;

    async fn store_wallet_verification_nonce(
        &self,
        user_id: Uuid,
        nonce: i64,
    ) -> Result<(), sqlx::Error>;

    async fn get_wallet_verification_nonce(
        &self,
        user_id: Uuid
    ) -> Result<i64, sqlx::Error>;

    async fn clear_wallet_verification_nonce(
        &self,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error>;

    async fn get_available_verifiers(&self) -> Result<Vec<User>, sqlx::Error>;

    async fn get_admin_user(&self) -> Result<Option<User>, sqlx::Error>;
 
    async fn assign_verifier_to_dispute(
        &self, 
        dispute_id: Uuid, 
        verifier_id: Uuid
    ) -> Result<(), sqlx::Error>;

   async fn update_user_verification_data(
        &self,
        user_id: Uuid,
        verification_status: VerificationStatus,
        verification_number: Option<String>,
        verification_type: VerificationType,
        verification_document_id: Option<String>,
        facial_verification_id: Option<String>,
        nationality: Option<String>,
        dob: Option<DateTime<Utc>>,
        lga: Option<String>,
        nearest_landmark: Option<String>,
    ) -> Result<User, sqlx::Error>;

    /// Get user by verification document ID (to check for duplicates)
    async fn get_user_by_verification_number(
        &self,
        verification_number: &str,
    ) -> Result<Option<User>, sqlx::Error>; 

    async fn update_user_avatar(
        &self,
        user_id: Uuid,
        avatar_url: String,
    ) -> Result<User, sqlx::Error>;

    async fn get_user_with_verification_status(
        &self,
        user_id: Uuid,
    ) -> Result<Option<(User, Vec<VerificationDocument>)>, sqlx::Error>;
    
    async fn get_users_with_verification_status(
        &self,
        page: u32,
        limit: usize,
    ) -> Result<Vec<(User, Vec<VerificationDocument>)>, sqlx::Error>;
    
    async fn get_role_change_stats(
        &self,
        user_id: Uuid,
    ) -> Result<RoleChangeStats, sqlx::Error>;
    
    async fn increment_role_change_count(
        &self,
        user_id: Uuid,
    ) -> Result<User, sqlx::Error>;
    
    async fn update_user_role_change_count(
        &self,
        user_id: Uuid,
        new_count: i32,
        reset_at: DateTime<Utc>,
    ) -> Result<User, sqlx::Error>;

}

#[derive(Debug)]
pub struct RoleChangeStats {
    pub current_count: i32,
    pub monthly_limit: i32,
    pub reset_at: DateTime<Utc>,
    pub has_premium: bool,
    pub remaining_changes: i32
}

#[async_trait]
impl UserExt for DBClient {
    async fn update_user_verification_token<T: Into<String> + Send>(
        &self,
        user_id: Uuid,
        verification_token: T,
        token_expires_at: DateTime<Utc>,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users 
            SET verification_token = $2,
                token_expires_at = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, name, username, email, password,
                role, trust_score, verified,
                verification_type,
                referral_code, referral_count, google_id, avatar_url,
                wallet_address, verification_status,
                nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                verification_token, token_expires_at,
                subscription_tier, role_change_count, role_change_reset_at,
                created_at,
                updated_at
            "#
        )
        .bind(user_id)
        .bind(verification_token.into())
        .bind(token_expires_at)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_available_verifiers(&self) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT 
                id, name, username, email, password,
                role, trust_score, verified,
                verification_type,
                referral_code, referral_count, google_id, avatar_url,
                wallet_address, verification_status,
                nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                verification_token, token_expires_at,
                subscription_tier, role_change_count, role_change_reset_at,
                created_at,
                updated_at
            FROM users
            WHERE base_role = 'verifier'::user_role
            AND id NOT IN (
                SELECT assigned_verifier 
                FROM disputes 
                WHERE status = 'open' AND assigned_verifier IS NOT NULL
            )
            "#
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn get_admin_user(&self) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT 
                id, name, username, email, password,
                role, trust_score, verified,
                verification_type,
                referral_code, referral_count, google_id, avatar_url,
                wallet_address, verification_status,
                nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                verification_token, token_expires_at,
                subscription_tier, role_change_count, role_change_reset_at,
                created_at,
                updated_at
            FROM users
            WHERE base_role = 'admin'::user_role
            LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await
    }

    async fn assign_verifier_to_dispute(&self, dispute_id: Uuid, verifier_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE disputes
            SET assigned_verifier = $1,
                status = 'under_review'::dispute_status
            WHERE id = $2
            "#
        )
        .bind(verifier_id)
        .bind(dispute_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_user(
        &self,
        user_id: Option<Uuid>,
        username: Option<&str>,
        email: Option<&str>,
        token: Option<&str>,
    ) -> Result<Option<User>, sqlx::Error> {
        let mut user: Option<User> = None;

        if let Some(user_id) = user_id {
            user = sqlx::query_as::<_, User>(
                r#"
                SELECT 
                    id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
                FROM users
                WHERE id = $1
                "#
            )
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        } else if let Some(username) = username {
            user = sqlx::query_as::<_, User>(
                r#"
                SELECT 
                    id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
                FROM users 
                WHERE username = $1
                "#
            )
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;
        } else if let Some(email) = email {
            user = sqlx::query_as::<_, User>(
                r#"
                SELECT 
                    id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
                FROM users 
                WHERE email = $1
                "#
            )
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        } else if let Some(token) = token {
            user = sqlx::query_as::<_, User>(
                r#"
                SELECT 
                    id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
                FROM users 
                WHERE verification_token = $1
                "#
            )
            .bind(token)
            .fetch_optional(&self.pool)
            .await?;
        }

        Ok(user)
    }

    async fn get_users(
        &self,
        page: u32,
        limit: usize,
    ) -> Result<Vec<User>, sqlx::Error> {
        let offset = (page - 1) * limit as u32;

        sqlx::query_as::<_, User>(
            r#"
            SELECT 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            FROM users
            ORDER BY created_at DESC LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
    }

    async fn save_user<T: Into<String> + Send>(
        &self,
        name: T,
        username: T,
        email: T,
        password: T,
        verification_token: T,
        token_expires_at: DateTime<Utc>,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (name, username, email, password, verification_token, token_expires_at, verification_status) 
            VALUES ($1, $2, $3, $4, $5, $6, 'unverified'::verification_status) 
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(name.into())
        .bind(username.into())
        .bind(email.into())
        .bind(password.into())
        .bind(verification_token.into())
        .bind(token_expires_at)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_count(&self) -> Result<i64, sqlx::Error> {
        let count :i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM users"#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    async fn update_user_name<T: Into<String> + Send>(
        &self,
        user_id: Uuid,
        new_name: T
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET name = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(new_name.into())
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn update_user_role(
        &self,
        target_id: Uuid,
        new_role: UserRole
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET base_role = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(new_role)
        .bind(target_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn update_user_password(
        &self,
        user_id: Uuid,
        new_password: String
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET password = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(new_password)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn update_transaction_pin(
        &self,
        user_id: Uuid,
        transaction_pin: i32,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET transaction_pin = $1, updated_at = NOW()
            WHERE id = $2
        RETURNING id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#,
        )
        .bind(transaction_pin)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn update_transaction_pin_hash(
        &self,
        user_id: Uuid,
        transaction_pin_hash: &str,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET transaction_pin_hash = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#,
        )
        .bind(transaction_pin_hash)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn verifed_token(
        &self,
        token: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET verified = true, 
                updated_at = NOW(),
                verification_token = NULL,
                token_expires_at = NULL
            WHERE verification_token = $1
            "#
        )
        .bind(token)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn add_verifed_token(
        &self,
        user_id: Uuid,
        token: &str,
        token_expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET verification_token = $1, token_expires_at = $2, updated_at = NOW()
            WHERE id = $3
            "#
        )
        .bind(token)
        .bind(token_expires_at)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_trust_point(
        &self,
        user_id: Uuid,
        score_to_add: i32,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET trust_score = trust_score + $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(score_to_add)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_by_identifier(&self, identifier: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = $1 OR email = $1"
        )
        .bind(identifier)
        .fetch_optional(&self.pool)
        .await
    }

    async fn get_users_by_trustscore(
        &self,
        limit: i64
    ) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            FROM users 
            ORDER BY trust_score DESC 
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_user_by_referral_code(
        &self,
        referral_code: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            FROM users 
            WHERE referral_code = $1
            "#
        )
        .bind(referral_code)
        .fetch_optional(&self.pool)
        .await
    }
    
    async fn update_user_referral_code(
        &self,
        user_id: Uuid,
        referral_code: String,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET referral_code = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(referral_code)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn add_referral_points(
        &self,
        referrer_id: Uuid,
        points: i32,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET trust_score = trust_score + $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
               id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(points)
        .bind(referrer_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn create_referral_record(
        &self,
        referrer_id: Uuid,  
        referee_id: Uuid,
        points: i32
    ) -> Result<Referral, sqlx::Error> {
        sqlx::query_as::<_, Referral>(
            r#"
            INSERT INTO referrals (referrer_id, referee_id, points_awarded)
            VALUES ($1, $2, $3)
            RETURNING id, referrer_id, referee_id, points_awarded, created_at
            "#
        )
        .bind(referrer_id)
        .bind(referee_id)
        .bind(points)
        .fetch_one(&self.pool)
        .await
    }

    async fn increment_referral_count(
        &self,
        user_id: Uuid,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET referral_count = referral_count + 1, updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_referral_stats(
        &self,
        user_id: Uuid,
    ) -> Result<ReferralStats, sqlx::Error> {
        // Get total referrals and points
        let stats = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_referrals,
                COALESCE(SUM(points_awarded), 0) as total_points_earned
            FROM referrals 
            WHERE referrer_id = $1
            "#
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Get successful referrals with user details
        let successful_referrals = sqlx::query_as::<_, ReferralUser>(
            r#"
            SELECT 
                u.id,
                u.name,
                u.username,
                u.email,
                r.created_at as joined_at
            FROM referrals r
            JOIN users u ON r.referee_id = u.id
            WHERE r.referrer_id = $1
            ORDER BY r.created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(ReferralStats {
            total_referrals: stats.get::<i64, _>("total_referrals"),
            total_points_earned: stats.get::<i64, _>("total_points_earned"),
            successful_referrals,
        })
    }
    
    async fn get_referral_by_referee(
        &self,
        referee_id: Uuid,
    ) -> Result<Option<Referral>, sqlx::Error> {
        sqlx::query_as::<_, Referral>(
            r#"
            SELECT id, referrer_id, referee_id, points_awarded, created_at
            FROM referrals
            WHERE referee_id = $1
            "#
        )
        .bind(referee_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn get_user_by_google_id(
        &self,
        google_id: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            FROM users
            WHERE google_id = $1
            "#
        )
        .bind(google_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn create_oauth_user(
        &self,
        name: String,
        email: String,
        google_id: String,
        avatar_url: Option<String>,
        trust_points: i32,
    ) -> Result<User, sqlx::Error> {
        //generate username from email
        let username = email.split("@").next().unwrap_or("user");
        let username = format!("{}_{}", username, chrono::Utc::now().timestamp());

        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (name, username, email, google_id, avatar_url, trust_score, verified, verification_status)
            VALUES ($1, $2, $3, $4, $5, $6, true, 'unverified'::verification_status)
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(name)
        .bind(username)
        .bind(email)
        .bind(google_id)
        .bind(avatar_url)
        .bind(trust_points)
        .fetch_one(&self.pool)
        .await
    }

    async fn link_google_account(
        &self,
        user_id: Uuid,
        google_id: &str,
        avatar_url: Option<&str>,
    ) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users 
        SET google_id = $1, avatar_url = $2, verified = true, updated_at = NOW()
        WHERE id = $3
        RETURNING *
        "#
    )
    .bind(google_id)
    .bind(avatar_url)
    .bind(user_id)
    .fetch_one(&self.pool)
    .await?;

    Ok(user)
}

    async fn update_user_wallet(
        &self,
        user_id: Uuid,
        wallet_address: String,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET wallet_address = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(wallet_address)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn add_user_wallet(
        &self,
        user_id: Uuid,
        wallet_request: WalletUpdateRequest,
    ) -> Result<UserWallet, sqlx::Error> {
        let wallet_type = wallet_request.wallet_type.unwrap_or_else(|| "primary".to_string());
        let blockchain = wallet_request.blockchain.unwrap_or_else(|| "ethereum".to_string());

        sqlx::query_as::<_, UserWallet>(
            r#"
            INSERT INTO user_wallets (user_id, wallet_address, wallet_type, blockchain)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, wallet_type) DO UPDATE
            SET wallet_address = $2, updated_at = NOW()
            RETURNING id, user_id, wallet_address, wallet_type, blockchain, is_verified, created_at, updated_at
            "#
        )
        .bind(user_id)
        .bind(wallet_request.wallet_address)
        .bind(wallet_type)
        .bind(blockchain)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_wallets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserWallet>, sqlx::Error> {
        sqlx::query_as::<_, UserWallet>(
            r#"
            SELECT id, user_id, wallet_address, wallet_type, blockchain, is_verified, created_at, updated_at
            FROM user_wallets
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_primary_wallet(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserWallet>, sqlx::Error> {
        sqlx::query_as::<_, UserWallet>(
            r#"
            SELECT id, user_id, wallet_address, wallet_type, blockchain, is_verified, created_at, updated_at
            FROM user_wallets
            WHERE user_id = $1 AND wallet_type = 'primary'
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn verify_wallet(
        &self,
        user_id: Uuid,
        wallet_address: String,
    ) -> Result<UserWallet, sqlx::Error> {
        sqlx::query_as::<_, UserWallet>(
            r#"
            UPDATE user_wallets
            SET is_verified = true, updated_at = NOW()
            WHERE user_id = $1 AND wallet_address = $2
            RETURNING id, user_id, wallet_address, wallet_type, blockchain, is_verified, created_at, updated_at
            "#
        )
        .bind(user_id)
        .bind(wallet_address)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_by_wallet_address(
        &self,
        wallet_address: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT 
                id, name, username, email, password,
                base_role, trust_score, verified,
                verification_type,
                referral_code, referral_count, google_id, avatar_url,
                wallet_address, verification_status,
                nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                verification_number, nationality,
                dob, lga, transaction_pin, transaction_pin_hash, next_of_kin,
                verification_token, token_expires_at,
                created_at,
                updated_at
            FROM users 
            WHERE wallet_address = $1
            "#
        )
        .bind(wallet_address)
        .fetch_optional(&self.pool)
        .await
    }

    async fn store_wallet_verification_nonce(
        &self,
        user_id: Uuid,
        nonce: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO wallet_verification_nonces (user_id, nonce, expires_at)
            VALUES ($1, $2, NOW() + INTERVAL '5 minutes')
            ON CONFLICT (user_id)
            DO UPDATE SET
                nonce = $2,
                expires_at = NOW() + INTERVAL '5 minutes',
                updated_at = NOW()
            "#
        )
        .bind(user_id)
        .bind(nonce)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_wallet_verification_nonce(
        &self,
        user_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let record = sqlx::query(
            r#"
            SELECT nonce 
            FROM wallet_verification_nonces
            WHERE user_id = $1
            AND expires_at > NOW()
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match record {
            Some(record) => Ok(record.get::<i64, _>("nonce")),
            None => Err(sqlx::Error::RowNotFound),
        }
    }

    async fn clear_wallet_verification_nonce(
        &self,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM wallet_verification_nonces
            WHERE user_id = $1
            "#
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_user_verification_data(
        &self,
        user_id: Uuid,
        verification_status: VerificationStatus,
        verification_number: Option<String>,
        verification_type: VerificationType,
        verification_document_id: Option<String>,
        facial_verification_id: Option<String>,
        nationality: Option<String>,
        dob: Option<DateTime<Utc>>,
        lga: Option<String>,
        nearest_landmark: Option<String>,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users 
            SET 
                verification_status = $2,
                verification_number = COALESCE($3, verification_number),
                verification_type = COALESCE($4, verification_type),
                verification_document_id = COALESCE($5, verification_document_id),
                facial_verification_id = COALESCE($6, facial_verification_id),
                nationality = COALESCE($7, nationality),
                dob = COALESCE($8, dob),
                lga = COALESCE($9, lga),
                nearest_landmark = COALESCE($10, nearest_landmark),
                updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(user_id)
        .bind(verification_status)
        .bind(verification_number)
        .bind(verification_type)
        .bind(verification_document_id)
        .bind(facial_verification_id)
        .bind(nationality)
        .bind(dob)
        .bind(lga)
        .bind(nearest_landmark)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_by_verification_number(
        &self,
        verification_number: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            FROM users 
            WHERE verification_number = $1 
                OR nin_number = $1
            "#
        )
        .bind(verification_number)
        .fetch_optional(&self.pool)
        .await
    }

    async fn update_user_avatar(
        &self,
        user_id: Uuid,
        avatar_url: String,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET avatar_url = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING 
                id, name, username, email, password,
                    role, trust_score, verified,
                    verification_type,
                    referral_code, referral_count, google_id, avatar_url,
                    wallet_address, verification_status,
                    nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                    verification_number, nationality, dob, lga, transaction_pin, next_of_kin,
                    verification_token, token_expires_at,
                    subscription_tier, role_change_count, role_change_reset_at,
                    created_at,
                    updated_at
            "#
        )
        .bind(avatar_url)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_with_verification_status(
        &self,
        user_id: Uuid,
    ) -> Result<Option<(User, Vec<VerificationDocument>)>, sqlx::Error> {
        let user = self.get_user(Some(user_id), None, None, None).await?;
        
        if let Some(user) = user {
            let documents = self.get_user_verification_documents(user_id).await?;
            Ok(Some((user, documents)))
        } else {
            Ok(None)
        }
    }
    
    async fn get_users_with_verification_status(
        &self,
        page: u32,
        limit: usize,
    ) -> Result<Vec<(User, Vec<VerificationDocument>)>, sqlx::Error> {
        let users = self.get_users(page, limit).await?;
        let mut result = Vec::new();
        
        for user in users {
            let documents = self.get_user_verification_documents(user.id).await?;
            result.push((user, documents));
        }
        
        Ok(result)
    }
    
    async fn get_role_change_stats(
        &self,
        user_id: Uuid,
    ) -> Result<RoleChangeStats, sqlx::Error> {
        let user = self.get_user(Some(user_id), None, None, None).await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;
            
        let monthly_limit = user.clone().get_monthly_role_changes();
        let current_count = user.role_change_count.unwrap_or(0);
        let reset_at = user.role_change_reset_at.unwrap_or_else(|| Utc::now() + chrono::Duration::days(30));
        
        Ok(RoleChangeStats {
            current_count,
            monthly_limit,
            reset_at,
            has_premium: user.has_premium_subscription(),
            remaining_changes: monthly_limit.saturating_sub(current_count),
        })
    }
    
    async fn increment_role_change_count(
        &self,
        user_id: Uuid,
    ) -> Result<User, sqlx::Error> {
        let user = self.get_user(Some(user_id), None, None, None).await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;
            
        let now = Utc::now();
        let (new_count, reset_at) = if let Some(reset_at) = user.role_change_reset_at {
            if now > reset_at {
                // Reset period passed, start fresh
                (1, now + chrono::Duration::days(30))
            } else {
                // Increment within current period
                (user.role_change_count.unwrap_or(0) + 1, reset_at)
            }
        } else {
            // First time setting
            (1, now + chrono::Duration::days(30))
        };
        
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET role_change_count = $1, role_change_reset_at = $2, updated_at = NOW()
            WHERE id = $3
            RETURNING *
            "#
        )
        .bind(new_count)
        .bind(reset_at)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn update_user_role_change_count(
        &self,
        user_id: Uuid,
        new_count: i32,
        reset_at: DateTime<Utc>,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET role_change_count = $1, role_change_reset_at = $2, updated_at = NOW()
            WHERE id = $3
            RETURNING *
            "#
        )
        .bind(new_count)
        .bind(reset_at)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }
}