// db/verificationdb.rs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::db::DBClient;

use crate::models::{
    usermodel::{VerificationStatus, VerificationType},
    verificationmodels::{
        DocumentVerificationRequest, FacialVerificationRequest, NinVerificationRequest, 
        OtpRecord, OtpPurpose, VerificationDocument, VerificationResponse
    },
};

#[async_trait]
pub trait VerificationExt {
    // OTP Methods
    async fn create_otp(
        &self,
        user_id: Uuid,
        email: String,
        otp_code: String,
        purpose: OtpPurpose,
        expires_at: DateTime<Utc>,
    ) -> Result<OtpRecord, sqlx::Error>;

    async fn get_valid_otp(
        &self,
        email: &str,
        otp_code: &str,
        purpose: OtpPurpose,
    ) -> Result<Option<OtpRecord>, sqlx::Error>;

    async fn mark_otp_used(
        &self,
        otp_id: Uuid,
    ) -> Result<(), sqlx::Error>;

    async fn cleanup_expired_otps(&self) -> Result<u64, sqlx::Error>;

    // Verification Document Methods
    async fn create_verification_document(
        &self,
        user_id: Uuid,
        document_type: VerificationType,
        document_id: String,
        document_url: String,
        selfie_url: String,
    ) -> Result<VerificationDocument, sqlx::Error>;

    async fn get_user_verification_documents(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<VerificationDocument>, sqlx::Error>;

    async fn get_pending_verifications(
        &self,
    ) -> Result<Vec<VerificationDocument>, sqlx::Error>;

    async fn update_verification_status(
        &self,
        verification_id: Uuid,
        status: VerificationStatus,
        reviewed_by: Option<Uuid>,
        review_notes: Option<String>,
    ) -> Result<VerificationDocument, sqlx::Error>;

    async fn get_verification_by_id(
        &self,
        verification_id: Uuid,
    ) -> Result<Option<VerificationDocument>, sqlx::Error>;

    // User verification status update
    async fn update_user_verification_status(
        &self,
        user_id: Uuid,
        status: VerificationStatus,
    ) -> Result<(), sqlx::Error>;
}

#[async_trait]
impl VerificationExt for DBClient {
    async fn create_otp(
        &self,
        user_id: Uuid,
        email: String,
        otp_code: String,
        purpose: OtpPurpose,
        expires_at: DateTime<Utc>,
    ) -> Result<OtpRecord, sqlx::Error> {
        sqlx::query_as!(
            OtpRecord,
            r#"
            INSERT INTO otp_codes (user_id, email, otp_code, purpose, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING 
                id, user_id, email, otp_code, 
                purpose as "purpose: OtpPurpose",
                expires_at, used, created_at
            "#,
            user_id,
            email,
            otp_code,
            purpose as OtpPurpose,
            expires_at,
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_valid_otp(
        &self,
        email: &str,
        otp_code: &str,
        purpose: OtpPurpose,
    ) -> Result<Option<OtpRecord>, sqlx::Error> {
        sqlx::query_as!(
            OtpRecord,
            r#"
            SELECT id, user_id, email, otp_code, 
                purpose as "purpose: OtpPurpose",
                expires_at, used, created_at
            FROM otp_codes
            WHERE email = $1 
                AND otp_code = $2 
                AND purpose = $3
                AND used = false
                AND expires_at > NOW()
            "#,
            email,
            otp_code,
            purpose as OtpPurpose,
        )
        .fetch_optional(&self.pool)
        .await
    }

    async fn mark_otp_used(
        &self,
        otp_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE otp_codes
            SET used = true
            WHERE id = $1
            "#,
            otp_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn cleanup_expired_otps(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            DELETE FROM otp_codes
            WHERE expires_at < NOW() OR used = true
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn create_verification_document(
        &self,
        user_id: Uuid,
        document_type: VerificationType,
        document_id: String,
        document_url: String,
        selfie_url: String,
    ) -> Result<VerificationDocument, sqlx::Error> {
        sqlx::query_as!(
            VerificationDocument,
            r#"
            INSERT INTO verification_documents 
                (user_id, document_type, document_id, document_url, selfie_url, status)
            VALUES ($1, $2, $3, $4, $5, 'pending')
            RETURNING 
                id, user_id, 
                document_type as "document_type: VerificationType",
                document_id, document_url, selfie_url,
                status as "status: VerificationStatus",
                reviewed_by, review_notes, created_at, updated_at
            "#,
            user_id,
            document_type as VerificationType,
            document_id,
            document_url,
            selfie_url,
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_user_verification_documents(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<VerificationDocument>, sqlx::Error> {
        sqlx::query_as!(
            VerificationDocument,
            r#"
            SELECT 
                id, user_id, 
                document_type as "document_type: VerificationType",
                document_id, document_url, selfie_url,
                status as "status: VerificationStatus",
                reviewed_by, review_notes, created_at, updated_at
            FROM verification_documents
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn get_pending_verifications(
        &self,
    ) -> Result<Vec<VerificationDocument>, sqlx::Error> {
        sqlx::query_as!(
            VerificationDocument,
            r#"
            SELECT 
                id, user_id, 
                document_type as "document_type: VerificationType",
                document_id, document_url, selfie_url,
                status as "status: VerificationStatus",
                reviewed_by, review_notes, created_at, updated_at
            FROM verification_documents
            WHERE status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn update_verification_status(
        &self,
        verification_id: Uuid,
        status: VerificationStatus,
        reviewed_by: Option<Uuid>,
        review_notes: Option<String>,
    ) -> Result<VerificationDocument, sqlx::Error> {
        sqlx::query_as!(
            VerificationDocument,
            r#"
            UPDATE verification_documents
            SET status = $1, 
                reviewed_by = $2, 
                review_notes = $3,
                updated_at = NOW()
            WHERE id = $4
            RETURNING 
                id, user_id, 
                document_type as "document_type: VerificationType",
                document_id, document_url, selfie_url,
                status as "status: VerificationStatus",
                reviewed_by, review_notes, created_at, updated_at
            "#,
            status as VerificationStatus,
            reviewed_by,
            review_notes,
            verification_id,
        )
        .fetch_one(&self.pool)
        .await
    }

    async fn get_verification_by_id(
        &self,
        verification_id: Uuid,
    ) -> Result<Option<VerificationDocument>, sqlx::Error> {
        sqlx::query_as!(
            VerificationDocument,
            r#"
            SELECT 
                id, user_id, 
                document_type as "document_type: VerificationType",
                document_id, document_url, selfie_url,
                status as "status: VerificationStatus",
                reviewed_by, review_notes, created_at, updated_at
            FROM verification_documents
            WHERE id = $1
            "#,
            verification_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    async fn update_user_verification_status(
        &self,
        user_id: Uuid,
        status: VerificationStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE users
            SET verification_status = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            status as VerificationStatus,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}