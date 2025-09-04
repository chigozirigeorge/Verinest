//5
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::models::{
    referralmodel::{Referral, ReferralStats, ReferralUser}, 
    usermodel::{User, UserRole, VerificationType, VerificationStatus},
    walletmodels::{UserWallet, WalletUpdateRequest}
}; 

#[derive(Debug, Clone)]
pub struct DBClient {
    pool: Pool<Postgres>,
}

impl DBClient {
    pub fn new(pool: Pool<Postgres>) -> Self {
        DBClient { pool }
    }
}

#[async_trait]
pub trait UserExt {
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
}

#[async_trait]
impl UserExt for DBClient {
    async fn get_user(
        &self,
        user_id: Option<Uuid>,
        username: Option<&str>,
        email: Option<&str>,
        token: Option<&str>,
    ) -> Result<Option<User>, sqlx::Error> {
        let mut user: Option<User> = None;

        if let Some(user_id) = user_id {
            user = sqlx::query_as!(
                User,
               r#"
            SELECT 
                id, name, username, email, password,
                role as "role: UserRole", trust_score, verified,
                verification_type as "verification_type: VerificationType",
                referral_code, referral_count, google_id, avatar_url,
                wallet_address, verification_status as "verification_status: VerificationStatus",
                nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                verification_number, nationality,
                dob, lga, transaction_pin, next_of_kin,
                verification_token, token_expires_at,
                created_at as "created_at!: DateTime<Utc>", 
                updated_at as "updated_at!: DateTime<Utc>"
            FROM users
            WHERE id = $1"#,
                user_id
            ).fetch_optional(&self.pool).await?;
        } else if let Some(username) = username {
            user = sqlx::query_as!(
                User,
                r#"
            SELECT 
                id, name, username, email, password,
                role as "role: UserRole", trust_score, verified,
                verification_type as "verification_type: VerificationType",
                referral_code, referral_count, google_id, avatar_url,
                wallet_address, verification_status as "verification_status: VerificationStatus",
                nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                verification_number, nationality,
                dob, lga, transaction_pin, next_of_kin,
                verification_token, token_expires_at,
                created_at as "created_at!: DateTime<Utc>", 
                updated_at as "updated_at!: DateTime<Utc>"
            FROM users 
            WHERE username = $1"#,
                username
            ).fetch_optional(&self.pool).await?;
        } else if let Some(email) = email {
            user = sqlx::query_as!(
                User,
                r#"
            SELECT 
                id, name, username, email, password,
                role as "role: UserRole", trust_score, verified,
                verification_type as "verification_type: VerificationType",
                referral_code, referral_count, google_id, avatar_url,
                wallet_address, verification_status as "verification_status: VerificationStatus",
                nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                verification_number, nationality,
                dob, lga, transaction_pin, next_of_kin,
                verification_token, token_expires_at,
                created_at as "created_at!: DateTime<Utc>", 
                updated_at as "updated_at!: DateTime<Utc>"
            FROM users 
            WHERE email = $1"#,
                email
            ).fetch_optional(&self.pool).await?;
        } else if let Some(token) = token {
            user = sqlx::query_as!(
                User,
                r#"
            SELECT 
                id, name, username, email, password,
                role as "role: UserRole", trust_score, verified,
                verification_type as "verification_type: VerificationType",
                referral_code, referral_count, google_id, avatar_url,
                wallet_address, verification_status as "verification_status: VerificationStatus",
                nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                verification_number, nationality,
                dob, lga, transaction_pin, next_of_kin,
                verification_token, token_expires_at,
                created_at as "created_at!: DateTime<Utc>", 
                updated_at as "updated_at!: DateTime<Utc>"
            FROM users 
            WHERE verification_token = $1"#,
                token
            )
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

        let users = sqlx::query_as!(
            User,
            r#"
        SELECT 
            id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
        FROM users
        ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
            limit as i64,
            offset as i64,
        ).fetch_all(&self.pool)
        .await?;

        Ok(users)
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
        let user = sqlx::query_as!(
            User,
            r#"
        INSERT INTO users (name, username, email, password, verification_token, token_expires_at) 
        VALUES ($1, $2, $3, $4, $5, $6) 
        RETURNING id, name, username, email, password,
        role as "role: UserRole", trust_score, verified,
        verification_type as "verification_type: VerificationType",
        referral_code, referral_count, google_id, avatar_url,
        wallet_address, verification_status as "verification_status: VerificationStatus",
        nin_number, verification_document_id, facial_verification_id, nearest_landmark,
        verification_number, nationality,
        dob, lga, transaction_pin, next_of_kin,
        verification_token, token_expires_at,
        created_at as "created_at!: DateTime<Utc>", 
        updated_at as "updated_at!: DateTime<Utc>"
        "#,
            name.into(),
            username.into(),
            email.into(),
            password.into(),
            verification_token.into(),
            token_expires_at
        ).fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    async fn get_user_count(&self) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM users"#
        )
       .fetch_one(&self.pool)
       .await?;

        Ok(count.unwrap_or(0))
    }

    async fn update_user_name<T: Into<String> + Send>(
        &self,
        user_id: Uuid,
        new_name: T
    ) -> Result<User, sqlx::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET name = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
            "#,
            new_name.into(),
            user_id
        ).fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    async fn update_user_role(
        &self,
        target_id: Uuid,
        new_role: UserRole
    ) -> Result<User, sqlx::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET role = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
            "#,
            new_role as UserRole,
            target_id
        ).fetch_one(&self.pool)
       .await?;

        Ok(user)
    }

    async fn update_user_password(
        &self,
        user_id: Uuid,
        new_password: String
    ) -> Result<User, sqlx::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET password = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
            "#,
            new_password,
            user_id
        ).fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    async fn verifed_token(
        &self,
        token: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE users
            SET verified = true, 
                updated_at = NOW(),
                verification_token = NULL,
                token_expires_at = NULL
            WHERE verification_token = $1
            "#,
            token
        ).execute(&self.pool)
       .await?;

        Ok(())
    }

    async fn add_verifed_token(
        &self,
        user_id: Uuid,
        token: &str,
        token_expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE users
            SET verification_token = $1, token_expires_at = $2, updated_at = NOW()
            WHERE id = $3
            "#,
            token,
            token_expires_at,
            user_id,
        ).execute(&self.pool)
       .await?;

        Ok(())
    }

    async fn update_trust_point(
        &self,
        user_id: Uuid,
        score_to_add: i32,
    ) -> Result<User, sqlx::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET trust_score = trust_score + $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
            "#,
            score_to_add,
            user_id
        ).fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    async fn get_users_by_trustscore(
        &self,
        limit: i64
    ) -> Result<Vec<User>, sqlx::Error> {
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
            FROM users 
            ORDER BY trust_score DESC 
            LIMIT $1
            "#,
            limit
        ).fetch_all(&self.pool)
        .await?;

        Ok(users)
    }


    async fn get_user_by_referral_code(
        &self,
        referral_code: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT 
                id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
            FROM users 
            WHERE referral_code = $1"#,
            referral_code
        )
        .fetch_optional(&self.pool)
        .await
    }
    
    async fn update_user_referral_code(
        &self,
        user_id: Uuid,
        referral_code: String,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET referral_code = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
            "#,
            referral_code,
            user_id
        )
        .fetch_one(&self.pool)
        .await
    }
    
    async fn add_referral_points(
        &self,
        referrer_id: Uuid,
        points: i32,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET trust_score = trust_score + $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
            "#,
            points,
            referrer_id
        )
        .fetch_one(&self.pool)
        .await
    }

   async fn create_referral_record(
    &self,
    referrer_id: Uuid,  
    referee_id: Uuid,
    points: i32
) -> Result<Referral, sqlx::Error> {
    sqlx::query_as!(
        Referral,
        r#"
        INSERT INTO referrals (referrer_id, referee_id, points_awarded)
        VALUES ($1, $2, $3)
        RETURNING id, referrer_id, referee_id, points_awarded, created_at
        "#,
        referrer_id,  
        referee_id,
        points
    )
    .fetch_one(&self.pool)
    .await
}

    async fn increment_referral_count(
        &self,
        user_id: Uuid,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET referral_count = referral_count + 1, updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_referral_stats(
        &self,
        user_id: Uuid,
    ) -> Result<ReferralStats, sqlx::Error> {
        // Get total referrals and points
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_referrals,
                COALESCE(SUM(points_awarded), 0) as total_points_earned
            FROM referrals 
            WHERE referrer_id = $1
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Get successful referrals with user details
        let successful_referrals = sqlx::query_as!(
                ReferralUser,
                r#"
                SELECT 
                    u.id as "id!",
                    u.name as "name!",
                    u.username as "username!",
                    u.email as "email!",
                    r.created_at as "joined_at!"
                FROM referrals r
                JOIN users u ON r.referee_id = u.id
                WHERE r.referrer_id = $1
                ORDER BY r.created_at DESC
                "#,
                user_id
            )
            .fetch_all(&self.pool)
            .await?;

        Ok(ReferralStats {
            total_referrals: stats.total_referrals.unwrap_or(0),
            total_points_earned: stats.total_points_earned.unwrap_or(0),
            successful_referrals,
        })
    }
    
    async fn get_referral_by_referee(
        &self,
        referee_id: Uuid,
    ) -> Result<Option<Referral>, sqlx::Error> {
        sqlx::query_as!(
            Referral,
            r#"
            SELECT id, referrer_id, referee_id, points_awarded, created_at
            FROM referrals
            WHERE referee_id = $1
            "#,
            referee_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    async fn get_user_by_google_id(
        &self,
        google_id: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT
                id, name, username, email, password,
            role as "role: UserRole", trust_score, verified,
            verification_type as "verification_type: VerificationType",
            referral_code, referral_count, google_id, avatar_url,
            wallet_address, verification_status as "verification_status: VerificationStatus",
            nin_number, verification_document_id, facial_verification_id, nearest_landmark,
            verification_number, nationality,
            dob, lga, transaction_pin, next_of_kin,
            verification_token, token_expires_at,
            created_at as "created_at!: DateTime<Utc>", 
            updated_at as "updated_at!: DateTime<Utc>"
            FROM users
            WHERE google_id = $1
            "#,
            google_id
        )
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

        sqlx::query_as!(
            User,
            r#"
                INSERT INTO users (name, username, email, google_id, avatar_url, trust_score, verified)
                VALUES ($1, $2, $3, $4, $5, $6, true)
                RETURNING id, name, username, email, password,
                role as "role: UserRole", trust_score, verified,
                verification_type as "verification_type: VerificationType",
                referral_code, referral_count, google_id, avatar_url,
                wallet_address, verification_status as "verification_status: VerificationStatus",
                nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                verification_number, nationality, dob, lga, transaction_pin, next_of_kin,
                verification_token, token_expires_at,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            "#,
            name,
            username,
            email,
            google_id,
            avatar_url,
            trust_points,
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn update_user_wallet(
        &self,
        user_id: Uuid,
        wallet_address: String,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
                UPDATE users
                SET wallet_address = $1, updated_at = NOW()
                WHERE id = $2
                RETURNING id, name, username, email, password, role as "role: UserRole",
                trust_score, verified, verification_type as "verification_type: VerificationType",
                referral_code, referral_count, google_id, avatar_url, wallet_address, verification_status as "verification_status: VerificationStatus",
                nin_number, verification_document_id, facial_verification_id, nearest_landmark, verification_number, nationality,
                dob, lga, transaction_pin, next_of_kin, verification_token, token_expires_at, 
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>" 
            "#,
            wallet_address,
            user_id
        )
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

        sqlx::query_as!(
            UserWallet,
            r#"
                INSERT INTO user_wallets (user_id, wallet_address, wallet_type, blockchain)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (user_id, wallet_type) DO UPDATE
                SET wallet_address = $2, updated_at = NOW()
                RETURNING id, user_id, wallet_address, wallet_type, blockchain, is_verified, created_at, updated_at
            "#,
            user_id,
            wallet_request.wallet_address,
            wallet_type,
            blockchain
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_wallets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserWallet>, sqlx::Error> {
        sqlx::query_as!(
            UserWallet,
            r#"
                SELECT id, user_id, wallet_address, wallet_type, blockchain, is_verified, created_at, updated_at
                FROM user_wallets
                WHERE user_id = $1
                ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn get_primary_wallet(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserWallet>, sqlx::Error> {
        sqlx::query_as!(
            UserWallet,
            r#"
                SELECT id, user_id, wallet_address, wallet_type, blockchain, is_verified, created_at, updated_at
                FROM user_wallets
                WHERE user_id = $1 AND wallet_type = 'primary'
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    async fn verify_wallet(
        &self,
        user_id: Uuid,
        wallet_address: String,
    ) -> Result<UserWallet, sqlx::Error> {
        sqlx::query_as!(
            UserWallet,
            r#"
                UPDATE user_wallets
                SET is_verified = true, updated_at = NOW()
                WHERE user_id = $1 AND wallet_address = $2
                RETURNING id, user_id, wallet_address, wallet_type, blockchain, is_verified, created_at, updated_at
            "#,
            user_id,
            wallet_address
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_by_wallet_address(
        &self,
        wallet_address: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT 
                id, name, username, email, password,
                role as "role: UserRole", trust_score, verified,
                verification_type as "verification_type: VerificationType",
                referral_code, referral_count, google_id, avatar_url,
                wallet_address, verification_status as "verification_status: VerificationStatus",
                nin_number, verification_document_id, facial_verification_id, nearest_landmark,
                verification_number, nationality,
                dob, lga, transaction_pin, next_of_kin,
                verification_token, token_expires_at,
                created_at as "created_at!: DateTime<Utc>", 
                updated_at as "updated_at!: DateTime<Utc>"
            FROM users 
            WHERE wallet_address = $1"#,
            wallet_address
        )
        .fetch_optional(&self.pool)
        .await
    }
}
