// db/naira_walletdb.rs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::{Error, Row};
use serde_json::Value as JsonValue;
use num_traits::ToPrimitive;
use bigdecimal::BigDecimal;

use super::db::DBClient;
use crate::models::walletmodels::*;
use crate::utils::decimal::BigDecimalHelpers;

#[async_trait]
pub trait NairaWalletExt {
    // Wallet Management
    async fn create_naira_wallet(&self, user_id: Uuid) -> Result<NairaWallet, Error>;
    async fn get_naira_wallet(&self, user_id: Uuid) -> Result<Option<NairaWallet>, Error>;
    async fn update_wallet_status(
        &self, 
        wallet_id: Uuid, 
        status: WalletStatus
    ) -> Result<NairaWallet, Error>;
    
    // Balance Operations
    async fn get_wallet_balance(&self, user_id: Uuid) -> Result<i64, Error>;
    async fn credit_wallet(
        &self, 
        user_id: Uuid, 
        amount: i64, 
        transaction_type: TransactionType, 
        description: String,
        reference: String,
        external_reference: Option<String>,
        metadata: Option<JsonValue>
    ) -> Result<WalletTransaction, Error>;
    
    async fn debit_wallet(
        &self, 
        user_id: Uuid, 
        amount: i64, 
        transaction_type: TransactionType, 
        description: String,
        reference: String,
        external_reference: Option<String>,
        metadata: Option<JsonValue>
    ) -> Result<WalletTransaction, Error>;

    async fn refund_transaction(
        &self,
        transaction_id: Uuid,
    ) -> Result<WalletTransaction, Error>;

    // Transfer Operations
    async fn transfer_funds(
        &self,
        sender_id: Uuid,
        recipient_id: Uuid,
        amount: i64,
        description: String,
        reference: String
    ) -> Result<(WalletTransaction, WalletTransaction), Error>;

    // Hold Operations (for escrow)
    async fn create_wallet_hold(
        &self,
        wallet_id: Uuid,
        job_id: Option<Uuid>,
        amount: i64,
        reason: String,
        expires_at: Option<DateTime<Utc>>
    ) -> Result<WalletHold, Error>;

    async fn release_wallet_hold(
        &self,
        hold_id: Uuid,
        release_to_available: bool
    ) -> Result<(), Error>;

    async fn get_wallet_holds(
        &self,
        wallet_id: Uuid,
        status: Option<String>
    ) -> Result<Vec<WalletHold>, Error>;

    // Transaction History
    async fn get_wallet_transactions(
        &self,
        user_id: Uuid,
        transaction_type: Option<TransactionType>,
        status: Option<TransactionStatus>,
        limit: i64,
        offset: i64
    ) -> Result<Vec<WalletTransaction>, Error>;

    async fn get_transaction_by_reference(
        &self,
        reference: &str
    ) -> Result<Option<WalletTransaction>, Error>;

    async fn update_transaction_status(
        &self,
        transaction_id: Uuid,
        status: TransactionStatus,
        external_reference: Option<String>
    ) -> Result<WalletTransaction, Error>;

    // Bank Account Management
    async fn add_bank_account(
        &self,
        user_id: Uuid,
        account_name: String,
        account_number: String,
        bank_code: String,
        bank_name: String
    ) -> Result<BankAccount, Error>;

    async fn verify_bank_account(
        &self,
        account_id: Uuid
    ) -> Result<BankAccount, Error>;

    async fn set_primary_bank_account(
        &self,
        user_id: Uuid,
        account_id: Uuid
    ) -> Result<BankAccount, Error>;

    async fn get_user_bank_accounts(
        &self,
        user_id: Uuid
    ) -> Result<Vec<BankAccount>, Error>;

    async fn get_primary_bank_account(
        &self,
        user_id: Uuid
    ) -> Result<Option<BankAccount>, Error>;

    // Fee Calculation
    async fn calculate_transaction_fee(
        &self,
        transaction_type: TransactionType,
        amount: i64
    ) -> Result<i64, Error>;

    // Limit Checking
    async fn check_transaction_limits(
        &self,
        user_id: Uuid,
        transaction_type: TransactionType,
        amount: i64
    ) -> Result<bool, Error>;

    // Analytics
    async fn get_wallet_summary(
        &self,
        user_id: Uuid
    ) -> Result<WalletSummary, Error>;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WalletSummary {
    pub balance: i64,
    pub available_balance: i64,
    pub total_deposits: i64,
    pub total_withdrawals: i64,
    pub pending_transactions: i64,
    pub active_holds: i64,
}

#[async_trait]
impl NairaWalletExt for DBClient {
    async fn refund_transaction(
        &self,
        transaction_id: Uuid,
    ) -> Result<WalletTransaction, Error> {
        let mut tx = self.pool.begin().await?;

        // Get the original transaction
        let original = sqlx::query_as::<_, WalletTransaction>(
            r#"
            SELECT 
                id, 
                wallet_id, 
                user_id, 
                transaction_type,
                amount, 
                balance_before, 
                balance_after, 
                status,
                reference, 
                external_reference, 
                payment_method,
                description, 
                metadata, 
                job_id, 
                recipient_wallet_id, 
                fee_amount,
                created_at, 
                updated_at, 
                completed_at
            FROM wallet_transactions
            WHERE id = $1
            FOR UPDATE
            "#
        )
        .bind(transaction_id)
        .fetch_one(&mut *tx)
        .await?;

        // Get current wallet balance
        let wallet = sqlx::query(
            "SELECT id, balance, available_balance FROM naira_wallets WHERE id = $1 FOR UPDATE"
        )
        .bind(original.wallet_id)
        .fetch_one(&mut *tx)
        .await?;

        let balance_before = wallet.get::<i64, _>("balance");
        let balance_after = balance_before + original.amount;
        let available_after = wallet.get::<i64, _>("available_balance") + original.amount;

        // Update wallet balance
        sqlx::query(
            r#"
            UPDATE naira_wallets 
            SET balance = $2, 
                available_balance = $3,
                updated_at = NOW(),
                last_activity_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(wallet.get::<Uuid, _>("id"))
        .bind(balance_after)
        .bind(available_after)
        .execute(&mut *tx)
        .await?;

        // Create refund transaction record
        let refund = sqlx::query_as::<_, WalletTransaction>(
            r#"
            INSERT INTO wallet_transactions 
            (wallet_id, user_id, transaction_type, amount, balance_before, balance_after, 
             reference, external_reference, description, metadata, status)
            VALUES ($1, $2, 'job_refund'::transaction_type, $3, $4, $5, $6, $7, $8, $9, 'completed'::transaction_status)
            RETURNING 
                id, 
                wallet_id, 
                user_id, 
                transaction_type,
                amount, 
                balance_before, 
                balance_after, 
                status,
                reference, 
                external_reference, 
                payment_method,
                description, 
                metadata, 
                job_id, 
                recipient_wallet_id, 
                fee_amount,
                created_at, 
                updated_at, 
                completed_at
            "#
        )
        .bind(original.wallet_id)
        .bind(original.user_id)
        .bind(original.amount)
        .bind(balance_before)
        .bind(balance_after)
        .bind(format!("REFUND-{}", &original.reference))
        .bind(original.external_reference)
        .bind(format!("Refund for transaction {}", original.reference))
        .bind(original.metadata)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(refund)
    }

    async fn create_naira_wallet(&self, user_id: Uuid) -> Result<NairaWallet, Error> {
        sqlx::query_as::<_, NairaWallet>(
            r#"
            INSERT INTO naira_wallets (user_id)
            VALUES ($1)
            RETURNING 
                id, 
                user_id, 
                balance, 
                available_balance, 
                total_deposits, 
                total_withdrawals, 
                status, 
                daily_limit, 
                monthly_limit, 
                is_verified, 
                bvn_verified, 
                created_at, 
                updated_at, 
                last_activity_at
            "#
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_naira_wallet(&self, user_id: Uuid) -> Result<Option<NairaWallet>, Error> {
        sqlx::query_as::<_, NairaWallet>(
            r#"
            SELECT 
                id, 
                user_id, 
                balance, 
                available_balance, 
                total_deposits, 
                total_withdrawals, 
                status, 
                daily_limit, 
                monthly_limit, 
                is_verified, 
                bvn_verified, 
                created_at, 
                updated_at, 
                last_activity_at
            FROM naira_wallets 
            WHERE user_id = $1
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn update_wallet_status(
        &self, 
        wallet_id: Uuid, 
        status: WalletStatus
    ) -> Result<NairaWallet, Error> {
        sqlx::query_as::<_, NairaWallet>(
            r#"
            UPDATE naira_wallets 
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, 
                user_id, 
                balance, 
                available_balance, 
                total_deposits, 
                total_withdrawals, 
                status, 
                daily_limit, 
                monthly_limit, 
                is_verified, 
                bvn_verified, 
                created_at, 
                updated_at, 
                last_activity_at
            "#
        )
        .bind(wallet_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_wallet_balance(&self, user_id: Uuid) -> Result<i64, Error> {
        let result = sqlx::query(
            "SELECT balance FROM naira_wallets WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(result.get::<i64, _>("balance"))
    }

    async fn credit_wallet(
        &self, 
        user_id: Uuid, 
        amount: i64, 
        transaction_type: TransactionType, 
        description: String,
        reference: String,
        external_reference: Option<String>,
        metadata: Option<JsonValue>
    ) -> Result<WalletTransaction, Error> {
        let mut tx = self.pool.begin().await?;
        
        // Get current wallet balance
        let wallet = sqlx::query(
            "SELECT id, balance, available_balance FROM naira_wallets WHERE user_id = $1 FOR UPDATE"
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        let balance_before = wallet.get::<i64, _>("balance");
        let balance_after = balance_before + amount;
        let available_after = wallet.get::<i64, _>("available_balance") + amount;

        // Update wallet balance
        sqlx::query(
            r#"
            UPDATE naira_wallets 
            SET balance = $2, 
                available_balance = $3,
                total_deposits = CASE WHEN $4 = 'deposit'::transaction_type THEN total_deposits + $5 ELSE total_deposits END,
                updated_at = NOW(),
                last_activity_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(wallet.get::<Uuid, _>("id"))
        .bind(balance_after)
        .bind(available_after)
        .bind(transaction_type)
        .bind(amount)
        .execute(&mut *tx)
        .await?;

        // Create transaction record
        let transaction = sqlx::query_as::<_, WalletTransaction>(
            r#"
            INSERT INTO wallet_transactions 
            (wallet_id, user_id, transaction_type, amount, balance_before, balance_after, 
             reference, external_reference, description, metadata, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'completed'::transaction_status)
            RETURNING 
                id, 
                wallet_id, 
                user_id, 
                transaction_type,
                amount, 
                balance_before, 
                balance_after, 
                status,
                reference, 
                external_reference, 
                payment_method,
                description, 
                metadata, 
                job_id, 
                recipient_wallet_id, 
                fee_amount,
                created_at, 
                updated_at, 
                completed_at
            "#
        )
        .bind(wallet.get::<Uuid, _>("id"))
        .bind(user_id)
        .bind(transaction_type)
        .bind(amount)
        .bind(balance_before)
        .bind(balance_after)
        .bind(reference)
        .bind(external_reference)
        .bind(description)
        .bind(metadata)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(transaction)
    }

    async fn debit_wallet(
        &self, 
        user_id: Uuid, 
        amount: i64, 
        transaction_type: TransactionType, 
        description: String,
        reference: String,
        external_reference: Option<String>,
        metadata: Option<JsonValue>
    ) -> Result<WalletTransaction, Error> {
        let mut tx = self.pool.begin().await?;
        
        // Get current wallet balance
        let wallet = sqlx::query(
            "SELECT id, balance, available_balance FROM naira_wallets WHERE user_id = $1 FOR UPDATE"
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        // Check sufficient balance
        if wallet.get::<i64, _>("available_balance") < amount {
            return Err(Error::RowNotFound); // Should be custom error for insufficient funds
        }

        let balance_before = wallet.get::<i64, _>("balance");
        let balance_after = balance_before - amount;
        let available_after = wallet.get::<i64, _>("available_balance") - amount;

        // Update wallet balance
        sqlx::query(
            r#"
            UPDATE naira_wallets 
            SET balance = $2, 
                available_balance = $3,
                total_withdrawals = CASE WHEN $4 = 'withdrawal' THEN total_withdrawals + $5 ELSE total_withdrawals END,
                updated_at = NOW(),
                last_activity_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(wallet.get::<Uuid, _>("id"))
        .bind(balance_after)
        .bind(available_after)
        .bind(transaction_type)
        .bind(amount)
        .execute(&mut *tx)
        .await?;

        // Create transaction record
        let transaction = sqlx::query_as::<_, WalletTransaction>(
            r#"
            INSERT INTO wallet_transactions 
            (wallet_id, user_id, transaction_type, amount, balance_before, balance_after, 
             reference, external_reference, description, metadata, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'completed')
            RETURNING 
                id, 
                wallet_id, 
                user_id, 
                transaction_type,
                amount, 
                balance_before, 
                balance_after, 
                status,
                reference, 
                external_reference, 
                payment_method,
                description, 
                metadata, 
                job_id, 
                recipient_wallet_id, 
                fee_amount,
                created_at, 
                updated_at, 
                completed_at
            "#
        )
        .bind(wallet.get::<Uuid, _>("id"))
        .bind(user_id)
        .bind(transaction_type)
        .bind(amount)
        .bind(balance_before)
        .bind(balance_after)
        .bind(reference)
        .bind(external_reference)
        .bind(description)
        .bind(metadata)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(transaction)
    }

    async fn transfer_funds(
        &self,
        sender_id: Uuid,
        recipient_id: Uuid,
        amount: i64,
        description: String,
        reference: String
    ) -> Result<(WalletTransaction, WalletTransaction), Error> {
        let mut tx = self.pool.begin().await?;

        // Get sender and recipient wallets
        let sender_wallet = sqlx::query(
            "SELECT id, balance, available_balance FROM naira_wallets WHERE user_id = $1 FOR UPDATE"
        )
        .bind(sender_id)
        .fetch_one(&mut *tx)
        .await?;

        let recipient_wallet = sqlx::query(
            "SELECT id, balance, available_balance FROM naira_wallets WHERE user_id = $1 FOR UPDATE"
        )
        .bind(recipient_id)
        .fetch_one(&mut *tx)
        .await?;

        // Check sufficient balance
        if sender_wallet.get::<i64, _>("available_balance") < amount {
            return Err(Error::RowNotFound); // Should be custom error
        }

        // Calculate fee
        let fee = self.calculate_transaction_fee_internal(TransactionType::Transfer, amount).await?;
        let total_deduction = amount + fee;

        if sender_wallet.get::<i64, _>("available_balance") < total_deduction {
            return Err(Error::RowNotFound); // Insufficient funds including fee
        }

        // Update sender wallet
        let sender_balance_after = sender_wallet.get::<i64, _>("balance") - total_deduction;
        let sender_available_after = sender_wallet.get::<i64, _>("available_balance") - total_deduction;

        sqlx::query(
            r#"
            UPDATE naira_wallets 
            SET balance = $2, available_balance = $3, updated_at = NOW(), last_activity_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(sender_wallet.get::<Uuid, _>("id"))
        .bind(sender_balance_after)
        .bind(sender_available_after)
        .execute(&mut *tx)
        .await?;

        // Update recipient wallet
        let recipient_balance_after = recipient_wallet.get::<i64, _>("balance") + amount;
        let recipient_available_after = recipient_wallet.get::<i64, _>("available_balance") + amount;

        sqlx::query(
            r#"
            UPDATE naira_wallets 
            SET balance = $2, available_balance = $3, updated_at = NOW(), last_activity_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(recipient_wallet.get::<Uuid, _>("id"))
        .bind(recipient_balance_after)
        .bind(recipient_available_after)
        .execute(&mut *tx)
        .await?;

        // Create sender transaction (debit)
        let sender_tx = sqlx::query_as::<_, WalletTransaction>(
            r#"
            INSERT INTO wallet_transactions 
            (wallet_id, user_id, transaction_type, amount, balance_before, balance_after, 
             reference, description, recipient_wallet_id, fee_amount, status)
            VALUES ($1, $2, 'transfer', $3, $4, $5, $6, $7, $8, $9, 'completed')
            RETURNING 
                id, 
                wallet_id, 
                user_id, 
                transaction_type,
                amount, 
                balance_before, 
                balance_after, 
                status,
                reference, 
                external_reference, 
                payment_method,
                description, 
                metadata, 
                job_id, 
                recipient_wallet_id, 
                fee_amount,
                created_at, 
                updated_at, 
                completed_at
            "#
        )
        .bind(sender_wallet.get::<Uuid, _>("id"))
        .bind(sender_id)
        .bind(total_deduction)
        .bind(sender_wallet.get::<i64, _>("balance"))
        .bind(sender_balance_after)
        .bind(reference.clone())
        .bind(format!("Transfer to user: {}", description))
        .bind(recipient_wallet.get::<Uuid, _>("id"))
        .bind(fee)
        .fetch_one(&mut *tx)
        .await?;

        // Create recipient transaction (credit)
        let recipient_tx = sqlx::query_as::<_, WalletTransaction>(
            r#"
            INSERT INTO wallet_transactions 
            (wallet_id, user_id, transaction_type, amount, balance_before, balance_after, 
             reference, description, recipient_wallet_id, status)
            VALUES ($1, $2, 'transfer', $3, $4, $5, $6, $7, $8, 'completed')
            RETURNING 
                id, 
                wallet_id, 
                user_id, 
                transaction_type,
                amount, 
                balance_before, 
                balance_after, 
                status,
                reference, 
                external_reference, 
                payment_method,
                description, 
                metadata, 
                job_id, 
                recipient_wallet_id, 
                fee_amount,
                created_at, 
                updated_at, 
                completed_at
            "#
        )
        .bind(recipient_wallet.get::<Uuid, _>("id"))
        .bind(recipient_id)
        .bind(amount)
        .bind(recipient_wallet.get::<i64, _>("balance"))
        .bind(recipient_balance_after)
        .bind(reference)
        .bind(format!("Transfer from user: {}", description))
        .bind(sender_wallet.get::<Uuid, _>("id"))
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((sender_tx, recipient_tx))
    }

    async fn create_wallet_hold(
        &self,
        wallet_id: Uuid,
        job_id: Option<Uuid>,
        amount: i64,
        reason: String,
        expires_at: Option<DateTime<Utc>>
    ) -> Result<WalletHold, Error> {
        let mut tx = self.pool.begin().await?;

        // Get wallet and check available balance
        let wallet = sqlx::query(
            "SELECT available_balance FROM naira_wallets WHERE id = $1 FOR UPDATE"
        )
        .bind(wallet_id)
        .fetch_one(&mut *tx)
        .await?;

        if wallet.get::<i64, _>("available_balance") < amount {
            return Err(Error::RowNotFound); // Insufficient available balance
        }

        // Reduce available balance
        let new_available_balance = wallet.get::<i64, _>("available_balance") - amount;
        sqlx::query(
            "UPDATE naira_wallets SET available_balance = $2 WHERE id = $1"
        )
        .bind(wallet_id)
        .bind(new_available_balance)
        .execute(&mut *tx)
        .await?;

        // Create hold record
        let hold = sqlx::query_as::<_, WalletHold>(
            r#"
            INSERT INTO wallet_holds (wallet_id, job_id, amount, reason, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, wallet_id, job_id, amount, reason, status, created_at, expires_at, released_at
            "#
        )
        .bind(wallet_id)
        .bind(job_id)
        .bind(amount)
        .bind(reason)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(hold)
    }

    async fn release_wallet_hold(
        &self,
        hold_id: Uuid,
        release_to_available: bool
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        // Get hold details
        let hold = sqlx::query(
            "SELECT wallet_id, amount FROM wallet_holds WHERE id = $1 AND status = 'active'"
        )
        .bind(hold_id)
        .fetch_one(&mut *tx)
        .await?;

        if release_to_available {
            // Return amount to available balance
            sqlx::query(
                "UPDATE naira_wallets SET available_balance = available_balance + $2 WHERE id = $1"
            )
            .bind(hold.get::<Uuid, _>("wallet_id"))
            .bind(hold.get::<i64, _>("amount"))
            .execute(&mut *tx)
            .await?;
        } else {
            // Remove from total balance (funds used)
            sqlx::query(
                "UPDATE naira_wallets SET balance = balance - $2 WHERE id = $1"
            )
            .bind(hold.get::<Uuid, _>("wallet_id"))
            .bind(hold.get::<i64, _>("amount"))
            .execute(&mut *tx)
            .await?;
        }

        // Mark hold as released
        sqlx::query(
            "UPDATE wallet_holds SET status = 'released', released_at = NOW() WHERE id = $1"
        )
        .bind(hold_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_wallet_holds(
        &self,
        wallet_id: Uuid,
        status: Option<String>
    ) -> Result<Vec<WalletHold>, Error> {
        match status {
            Some(hold_status) => {
                sqlx::query_as::<_, WalletHold>(
                    r#"
                    SELECT id, wallet_id, job_id, amount, reason, status, created_at, expires_at, released_at
                    FROM wallet_holds 
                    WHERE wallet_id = $1 AND status = $2
                    ORDER BY created_at DESC
                    "#
                )
                .bind(wallet_id)
                .bind(hold_status)
                .fetch_all(&self.pool)
                .await
            },
            None => {
                sqlx::query_as::<_, WalletHold>(
                    r#"
                    SELECT id, wallet_id, job_id, amount, reason, status, created_at, expires_at, released_at
                    FROM wallet_holds 
                    WHERE wallet_id = $1
                    ORDER BY created_at DESC
                    "#
                )
                .bind(wallet_id)
                .fetch_all(&self.pool)
                .await
            }
        }
    }

    async fn get_wallet_transactions(
        &self,
        user_id: Uuid,
        transaction_type: Option<TransactionType>,
        status: Option<TransactionStatus>,
        limit: i64,
        offset: i64
    ) -> Result<Vec<WalletTransaction>, Error> {
        let mut query = r#"
            SELECT 
                id, wallet_id, user_id, transaction_type,
                amount, balance_before, balance_after, status,
                reference, external_reference, payment_method,
                description, metadata, job_id, recipient_wallet_id, fee_amount,
                created_at, updated_at, completed_at
            FROM wallet_transactions 
            WHERE user_id = $1
        "#.to_string();

        let mut param_count = 1;
        
        if transaction_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND transaction_type = ${}", param_count));
        }
        
        if status.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND status = ${}", param_count));
        }

        query.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", param_count + 1, param_count + 2));

        match (transaction_type, status) {
            (Some(tx_type), Some(tx_status)) => {
                sqlx::query_as::<_, WalletTransaction>(&query)
                    .bind(user_id)
                    .bind(tx_type)
                    .bind(tx_status)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await
            },
            (Some(tx_type), None) => {
                sqlx::query_as::<_, WalletTransaction>(&query)
                    .bind(user_id)
                    .bind(tx_type)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await
            },
            (None, Some(tx_status)) => {
                sqlx::query_as::<_, WalletTransaction>(&query)
                    .bind(user_id)
                    .bind(tx_status)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await
            },
            (None, None) => {
                sqlx::query_as::<_, WalletTransaction>(&query)
                    .bind(user_id)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await
            }
        }
    }

    async fn get_transaction_by_reference(
        &self,
        reference: &str
    ) -> Result<Option<WalletTransaction>, Error> {
        sqlx::query_as::<_, WalletTransaction>(
            r#"
            SELECT 
                id, wallet_id, user_id, transaction_type,
                amount, balance_before, balance_after, status,
                reference, external_reference, payment_method,
                description, metadata, job_id, recipient_wallet_id, fee_amount,
                created_at, updated_at, completed_at
            FROM wallet_transactions 
            WHERE reference = $1
            "#
        )
        .bind(reference)
        .fetch_optional(&self.pool)
        .await
    }

    async fn update_transaction_status(
        &self,
        transaction_id: Uuid,
        status: TransactionStatus,
        external_reference: Option<String>
    ) -> Result<WalletTransaction, Error> {
        sqlx::query_as::<_, WalletTransaction>(
            r#"
            UPDATE wallet_transactions 
            SET status = $2, external_reference = $3, 
                completed_at = CASE WHEN $2 = 'completed'::transaction_status THEN NOW() ELSE completed_at END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, wallet_id, user_id, transaction_type,
                amount, balance_before, balance_after, status,
                reference, external_reference, payment_method,
                description, metadata, job_id, recipient_wallet_id, fee_amount,
                created_at, updated_at, completed_at
            "#
        )
        .bind(transaction_id)
        .bind(status)
        .bind(external_reference)
        .fetch_one(&self.pool)
        .await
    }

    async fn add_bank_account(
        &self,
        user_id: Uuid,
        account_name: String,
        account_number: String,
        bank_code: String,
        bank_name: String
    ) -> Result<BankAccount, Error> {
        sqlx::query_as::<_, BankAccount>(
            r#"
            INSERT INTO bank_accounts (user_id, account_name, account_number, bank_code, bank_name)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING 
                id, 
                user_id, 
                account_name, 
                account_number, 
                bank_code, 
                bank_name,
                is_verified, 
                is_primary, 
                created_at, 
                updated_at
            "#
        )
        .bind(user_id)
        .bind(account_name)
        .bind(account_number)
        .bind(bank_code)
        .bind(bank_name)
        .fetch_one(&self.pool)
        .await
    }

    async fn verify_bank_account(
        &self,
        account_id: Uuid
    ) -> Result<BankAccount, Error> {
        sqlx::query_as::<_, BankAccount>(
            r#"
            UPDATE bank_accounts 
            SET is_verified = true, updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, 
                user_id, 
                account_name, 
                account_number, 
                bank_code, 
                bank_name,
                is_verified, 
                is_primary, 
                created_at, 
                updated_at
            "#
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn set_primary_bank_account(
        &self,
        user_id: Uuid,
        account_id: Uuid
    ) -> Result<BankAccount, Error> {
        let mut tx = self.pool.begin().await?;

        // Remove primary status from all user's accounts
        sqlx::query(
            "UPDATE bank_accounts SET is_primary = false WHERE user_id = $1"
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Set new primary account
        let account = sqlx::query_as::<_, BankAccount>(
            r#"
            UPDATE bank_accounts 
            SET is_primary = true, updated_at = NOW()
            WHERE id = $1 AND user_id = $2
            RETURNING 
                id, user_id, account_name, account_number, bank_code, bank_name,
                is_verified, is_primary, created_at, updated_at
            "#
        )
        .bind(account_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(account)
    }

    async fn get_user_bank_accounts(
        &self,
        user_id: Uuid
    ) -> Result<Vec<BankAccount>, Error> {
        sqlx::query_as::<_, BankAccount>(
            r#"
            SELECT 
                id, 
                user_id, 
                account_name, 
                account_number, 
                bank_code, 
                bank_name,
                is_verified, 
                is_primary, 
                created_at, 
                updated_at
            FROM bank_accounts 
            WHERE user_id = $1
            ORDER BY is_primary DESC, created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_primary_bank_account(
        &self,
        user_id: Uuid
    ) -> Result<Option<BankAccount>, Error> {
        sqlx::query_as::<_, BankAccount>(
            r#"
            SELECT 
                id, user_id, account_name, account_number, bank_code, bank_name,
                is_verified, is_primary, created_at, updated_at
            FROM bank_accounts 
            WHERE user_id = $1 AND is_primary = true
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn calculate_transaction_fee(
        &self,
        transaction_type: TransactionType,
        amount: i64
    ) -> Result<i64, Error> {
        self.calculate_transaction_fee_internal(transaction_type, amount).await
    }

    async fn check_transaction_limits(
        &self,
        user_id: Uuid,
        transaction_type: TransactionType,
        amount: i64
    ) -> Result<bool, Error> {
        // Get user tier (simplified - you'd determine this based on verification status)
        let user_tier = "basic"; // This should be determined from user verification status

        // Get limits for user tier and transaction type
        let limits = sqlx::query(
            r#"
            SELECT daily_limit, monthly_limit, per_transaction_limit
            FROM wallet_limits 
            WHERE user_tier = $1 AND transaction_type = $2 AND is_active = true
            "#
        )
        .bind(user_tier)
        .bind(transaction_type)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(limit) = limits {
            // Check per transaction limit
            if amount > limit.get::<i64, _>("per_transaction_limit") {
                return Ok(false);
            }

            // Check daily limit
            let today_total = sqlx::query(
                r#"
                SELECT COALESCE(SUM(amount), 0) as total
                FROM wallet_transactions 
                WHERE user_id = $1 AND transaction_type = $2 
                AND DATE(created_at) = CURRENT_DATE
                AND status = 'completed'
                "#
            )
            .bind(user_id)
            .bind(transaction_type)
            .fetch_one(&self.pool)
            .await?;

            let today_total_amount = today_total.get::<Option<BigDecimal>, _>("total")
                .and_then(|bd| bd.to_i64())
                .unwrap_or(0);

            if today_total_amount + amount > limit.get::<i64, _>("daily_limit") {
                return Ok(false);
            }

            // Check monthly limit
            let month_total = sqlx::query(
                r#"
                SELECT COALESCE(SUM(amount), 0) as total
                FROM wallet_transactions 
                WHERE user_id = $1 AND transaction_type = $2 
                AND DATE_TRUNC('month', created_at) = DATE_TRUNC('month', CURRENT_DATE)
                AND status = 'completed'
                "#
            )
            .bind(user_id)
            .bind(transaction_type)
            .fetch_one(&self.pool)
            .await?;

            let month_total_amount = month_total.get::<Option<BigDecimal>, _>("total")
                .and_then(|bd| bd.to_i64())
                .unwrap_or(0);

            if month_total_amount + amount > limit.get::<i64, _>("monthly_limit") {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn get_wallet_summary(
        &self,
        user_id: Uuid
    ) -> Result<WalletSummary, Error> {
        let wallet = sqlx::query(
            "SELECT balance, available_balance, total_deposits, total_withdrawals FROM naira_wallets WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let pending_count = sqlx::query(
            "SELECT COUNT(*) as count FROM wallet_transactions WHERE user_id = $1 AND status = 'pending'"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let active_holds = sqlx::query(
            r#"
            SELECT COALESCE(SUM(wh.amount), 0) as total
            FROM wallet_holds wh 
            JOIN naira_wallets nw ON wh.wallet_id = nw.id
            WHERE nw.user_id = $1 AND wh.status = 'active'
            "#
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(WalletSummary {
            balance: wallet.get::<i64, _>("balance"),
            available_balance: wallet.get::<i64, _>("available_balance"),
            total_deposits: wallet.get::<i64, _>("total_deposits"),
            total_withdrawals: wallet.get::<i64, _>("total_withdrawals"),
            pending_transactions: pending_count.get::<Option<i64>, _>("count").unwrap_or(0),
            active_holds: active_holds.get::<Option<BigDecimal>, _>("total")
                .and_then(|bd| bd.to_i64())
                .unwrap_or(0),
        })
    }
}

// Internal helper methods
impl DBClient {
    async fn calculate_transaction_fee_internal(
        &self,
        transaction_type: TransactionType,
        amount: i64
    ) -> Result<i64, Error> {
        let fee_config = sqlx::query(
            r#"
            SELECT fee_type, fee_value
            FROM transaction_fees 
            WHERE transaction_type = $1 AND is_active = true
            AND min_amount <= $2 AND max_amount >= $2
            ORDER BY min_amount DESC
            LIMIT 1
            "#
        )
        .bind(transaction_type)
        .bind(amount)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(config) = fee_config {
            match config.get::<String, _>("fee_type").as_str() {
                "fixed" => Ok(config.get::<i64, _>("fee_value")),
                "percentage" => {
                    // fee_value is in basis points (1/100th of a percent)
                    let fee = (amount * config.get::<i64, _>("fee_value")) / 10000;
                    Ok(fee)
                },
                _ => Ok(0)
            }
        } else {
            Ok(0) // No fee configuration found
        }
    }
}