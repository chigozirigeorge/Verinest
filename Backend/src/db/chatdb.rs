// db/chatdb.rs
use async_trait::async_trait;
use uuid::Uuid;
use sqlx::Error;
use super::db::DBClient;
use crate::models::chatnodels::*;

#[async_trait]
pub trait ChatExt {
    // Chat management
    async fn create_or_get_chat(
        &self,
        user_one_id: Uuid,
        user_two_id: Uuid,
        job_id: Option<Uuid>,
    ) -> Result<Chat, Error>;
    
    async fn get_user_chats(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Chat>, Error>;
    
    async fn get_chat_by_id(
        &self,
        chat_id: Uuid,
    ) -> Result<Option<Chat>, Error>;
    
    async fn update_chat_status(
        &self,
        chat_id: Uuid,
        status: ChatStatus,
    ) -> Result<Chat, Error>;
    
    // Message management
    async fn send_message(
        &self,
        chat_id: Uuid,
        sender_id: Uuid,
        message_type: MessageType,
        content: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<Message, Error>;
    
    async fn get_chat_messages(
        &self,
        chat_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, Error>;
    
    async fn mark_messages_as_read(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), Error>;
    
    async fn get_unread_count(
        &self,
        user_id: Uuid,
    ) -> Result<i64, Error>;
    
    // Contract proposal from chat
    async fn create_contract_proposal(
        &self,
        chat_id: Uuid,
        message_id: Uuid,
        job_id: Uuid,
        proposed_by: Uuid,
        worker_id: Uuid,
        employer_id: Uuid,
        agreed_rate: f64,
        agreed_timeline: i32,
        terms: String,
    ) -> Result<ContractProposal, Error>;
    
    async fn respond_to_contract_proposal(
        &self,
        proposal_id: Uuid,
        status: String,
    ) -> Result<ContractProposal, Error>;
    
    async fn get_contract_proposal_by_message(
        &self,
        message_id: Uuid,
    ) -> Result<Option<ContractProposal>, Error>;
}

#[async_trait]
impl ChatExt for DBClient {
    async fn create_or_get_chat(
        &self,
        user_one_id: Uuid,
        user_two_id: Uuid,
        job_id: Option<Uuid>,
    ) -> Result<Chat, Error> {
        // Try to find existing chat
        let existing = sqlx::query_as::<_, Chat>(
            r#"
            SELECT id, participant_one_id, participant_two_id, job_id, status, 
                   last_message_at, created_at
            FROM chats
            WHERE (participant_one_id = $1 AND participant_two_id = $2)
               OR (participant_one_id = $2 AND participant_two_id = $1)
            AND ($3::uuid IS NULL OR job_id = $3)
            "#
        )
        .bind(user_one_id)
        .bind(user_two_id)
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(chat) = existing {
            return Ok(chat);
        }
        
        // Create new chat
        sqlx::query_as::<_, Chat>(
            r#"
            INSERT INTO chats (participant_one_id, participant_two_id, job_id)
            VALUES ($1, $2, $3)
            RETURNING id, participant_one_id, participant_two_id, job_id, status,
                      last_message_at, created_at
            "#
        )
        .bind(user_one_id)
        .bind(user_two_id)
        .bind(job_id)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn get_user_chats(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Chat>, Error> {
        sqlx::query_as::<_, Chat>(
            r#"
            SELECT id, participant_one_id, participant_two_id, job_id, status,
                   last_message_at, created_at
            FROM chats
            WHERE (participant_one_id = $1 OR participant_two_id = $1)
              AND status = 'active'::chat_status
            ORDER BY last_message_at DESC NULLS LAST, created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }
    
    async fn get_chat_by_id(
        &self,
        chat_id: Uuid,
    ) -> Result<Option<Chat>, Error> {
        sqlx::query_as::<_, Chat>(
            r#"
            SELECT id, participant_one_id, participant_two_id, job_id, status,
                   last_message_at, created_at
            FROM chats
            WHERE id = $1
            "#
        )
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await
    }
    
    async fn update_chat_status(
        &self,
        chat_id: Uuid,
        status: ChatStatus,
    ) -> Result<Chat, Error> {
        sqlx::query_as::<_, Chat>(
            r#"
            UPDATE chats
            SET status = $2
            WHERE id = $1
            RETURNING id, participant_one_id, participant_two_id, job_id, status,
                      last_message_at, created_at
            "#
        )
        .bind(chat_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn send_message(
        &self,
        chat_id: Uuid,
        sender_id: Uuid,
        message_type: MessageType,
        content: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<Message, Error> {
        let mut tx = self.pool.begin().await?;
        
        // Insert message
        let message = sqlx::query_as::<_, Message>(
            r#"
            INSERT INTO messages (chat_id, sender_id, message_type, content, metadata)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, chat_id, sender_id, message_type, content, metadata,
                      is_read, read_at, created_at
            "#
        )
        .bind(chat_id)
        .bind(sender_id)
        .bind(message_type)
        .bind(content)
        .bind(metadata)
        .fetch_one(&mut *tx)
        .await?;
        
        // Update chat's last_message_at
        sqlx::query(
            r#"
            UPDATE chats
            SET last_message_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(chat_id)
        .execute(&mut *tx)
        .await?;
        
        tx.commit().await?;
        Ok(message)
    }
    
    async fn get_chat_messages(
        &self,
        chat_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, Error> {
        sqlx::query_as::<_, Message>(
            r#"
            SELECT id, chat_id, sender_id, message_type, content, metadata,
                   is_read, read_at, created_at
            FROM messages
            WHERE chat_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(chat_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }
    
    async fn mark_messages_as_read(
        &self,
        chat_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
            UPDATE messages
            SET is_read = true, read_at = NOW()
            WHERE chat_id = $1
              AND sender_id != $2
              AND is_read = false
            "#
        )
        .bind(chat_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn get_unread_count(
        &self,
        user_id: Uuid,
    ) -> Result<i64, Error> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM messages m
            INNER JOIN chats c ON m.chat_id = c.id
            WHERE (c.participant_one_id = $1 OR c.participant_two_id = $1)
              AND m.sender_id != $1
              AND m.is_read = false
            "#
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(result)
    }
    
    async fn create_contract_proposal(
        &self,
        chat_id: Uuid,
        message_id: Uuid,
        job_id: Uuid,
        proposed_by: Uuid,
        worker_id: Uuid,
        employer_id: Uuid,
        agreed_rate: f64,
        agreed_timeline: i32,
        terms: String,
    ) -> Result<ContractProposal, Error> {
        sqlx::query_as::<_, ContractProposal>(
            r#"
            INSERT INTO contract_proposals 
            (chat_id, message_id, job_id, proposed_by, worker_id, employer_id, 
             agreed_rate, agreed_timeline, terms)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, chat_id, message_id, job_id, proposed_by, worker_id,
                      employer_id, agreed_rate, agreed_timeline, terms, status,
                      created_at, responded_at
            "#
        )
        .bind(chat_id)
        .bind(message_id)
        .bind(job_id)
        .bind(proposed_by)
        .bind(worker_id)
        .bind(employer_id)
        .bind(agreed_rate)
        .bind(agreed_timeline)
        .bind(terms)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn respond_to_contract_proposal(
        &self,
        proposal_id: Uuid,
        status: String,
    ) -> Result<ContractProposal, Error> {
        sqlx::query_as::<_, ContractProposal>(
            r#"
            UPDATE contract_proposals
            SET status = $2, responded_at = NOW()
            WHERE id = $1
            RETURNING id, chat_id, message_id, job_id, proposed_by, worker_id,
                      employer_id, agreed_rate, agreed_timeline, terms, status,
                      created_at, responded_at
            "#
        )
        .bind(proposal_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }
    
    async fn get_contract_proposal_by_message(
        &self,
        message_id: Uuid,
    ) -> Result<Option<ContractProposal>, Error> {
        sqlx::query_as::<_, ContractProposal>(
            r#"
            SELECT id, chat_id, message_id, job_id, proposed_by, worker_id,
                   employer_id, agreed_rate, agreed_timeline, terms, status,
                   created_at, responded_at
            FROM contract_proposals
            WHERE message_id = $1
            "#
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
    }
}