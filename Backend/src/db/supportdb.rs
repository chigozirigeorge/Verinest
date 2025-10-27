// src/db/supportdb.rs
use sqlx::{Pool, Postgres, Error};
use uuid::Uuid;
use async_trait::async_trait;

use super::db::DBClient;
use crate::models::supportmodel::*;

#[async_trait]
pub trait SupportExt {
    async fn create_support_ticket(
        &self,
        user_id: Uuid,
        title: String,
        description: String,
        category: TicketCategory,
        priority: TicketPriority,
    ) -> Result<SupportTicket, Error>;
    
    async fn get_support_tickets(
        &self,
        limit: i64,
        offset: i64,
        status: Option<TicketStatus>,
    ) -> Result<Vec<SupportTicketWithUser>, Error>;
    
    async fn get_user_support_tickets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<SupportTicket>, Error>;
    
    async fn get_support_ticket(
        &self,
        ticket_id: Uuid,
    ) -> Result<Option<SupportTicket>, Error>;
    
    async fn get_support_ticket_with_messages(
        &self,
        ticket_id: Uuid,
    ) -> Result<Option<SupportTicketWithMessages>, Error>;
    
    async fn add_ticket_message(
        &self,
        ticket_id: Uuid,
        user_id: Uuid,
        message: String,
        is_internal: bool,
    ) -> Result<SupportMessage, Error>;
    
    async fn get_ticket_messages(
        &self,
        ticket_id: Uuid,
    ) -> Result<Vec<SupportMessageWithUser>, Error>;
    
    async fn update_ticket_status(
        &self,
        ticket_id: Uuid,
        status: TicketStatus,
    ) -> Result<SupportTicket, Error>;
    
    async fn assign_ticket(
        &self,
        ticket_id: Uuid,
        assigned_to: Uuid,
    ) -> Result<SupportTicket, Error>;
}


#[async_trait]
impl SupportExt for DBClient {
    async fn create_support_ticket(
        &self,
        user_id: Uuid,
        title: String,
        description: String,
        category: TicketCategory,
        priority: TicketPriority,
    ) -> Result<SupportTicket, sqlx::Error> {
        let ticket = sqlx::query_as::<_, SupportTicket>(
            r#"
            INSERT INTO support_tickets (user_id, title, description, category, priority, status)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(title)
        .bind(description)
        .bind(category)
        .bind(priority)
        .bind(TicketStatus::Open)
        .fetch_one(&self.pool)
        .await?;

        Ok(ticket)
    }

    async fn get_support_tickets(
        &self,
        limit: i64,
        offset: i64,
        status: Option<TicketStatus>,
    ) -> Result<Vec<SupportTicketWithUser>, sqlx::Error> {
        let query = match status {
            Some(status) => {
                sqlx::query_as::<_, SupportTicketWithUser>(
                    r#"
                    SELECT 
                        st.*,
                        u.name as user_name,
                        u.email as user_email,
                        u.username as user_username
                    FROM support_tickets st
                    JOIN users u ON st.user_id = u.id
                    WHERE st.status = $1
                    ORDER BY st.created_at DESC
                    LIMIT $2 OFFSET $3
                    "#
                )
                .bind(status)
                .bind(limit)
                .bind(offset)
            }
            None => {
                sqlx::query_as::<_, SupportTicketWithUser>(
                    r#"
                    SELECT 
                        st.*,
                        u.name as user_name,
                        u.email as user_email,
                        u.username as user_username
                    FROM support_tickets st
                    JOIN users u ON st.user_id = u.id
                    ORDER BY st.created_at DESC
                    LIMIT $1 OFFSET $2
                    "#
                )
                .bind(limit)
                .bind(offset)
            }
        };

        let tickets = query.fetch_all(&self.pool).await?;
        Ok(tickets)
    }

    async fn get_user_support_tickets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<SupportTicket>, sqlx::Error> {
        let tickets = sqlx::query_as::<_, SupportTicket>(
            r#"
            SELECT * FROM support_tickets
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(tickets)
    }

    async fn get_support_ticket(
        &self,
        ticket_id: Uuid,
    ) -> Result<Option<SupportTicket>, sqlx::Error> {
        let ticket = sqlx::query_as::<_, SupportTicket>(
            r#"
            SELECT * FROM support_tickets
            WHERE id = $1
            "#
        )
        .bind(ticket_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(ticket)
    }

    async fn get_support_ticket_with_messages(
        &self,
        ticket_id: Uuid,
    ) -> Result<Option<SupportTicketWithMessages>, sqlx::Error> {
        let ticket = sqlx::query_as::<_, SupportTicket>(
            r#"
            SELECT * FROM support_tickets
            WHERE id = $1
            "#
        )
        .bind(ticket_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(ticket) = ticket {
            let messages = Self::get_ticket_messages(&self, ticket_id).await?;
            Ok(Some(SupportTicketWithMessages {
                ticket,
                messages,
            }))
        } else {
            Ok(None)
        }
    }

    async fn add_ticket_message(
        &self,
        ticket_id: Uuid,
        user_id: Uuid,
        message: String,
        is_internal: bool,
    ) -> Result<SupportMessage, sqlx::Error> {
        let msg = sqlx::query_as::<_, SupportMessage>(
            r#"
            INSERT INTO support_messages (ticket_id, user_id, message, is_internal)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#
        )
        .bind(ticket_id)
        .bind(user_id)
        .bind(message)
        .bind(is_internal)
        .fetch_one(&self.pool)
        .await?;

        Ok(msg)
    }

    async fn get_ticket_messages(
        &self,
        ticket_id: Uuid,
    ) -> Result<Vec<SupportMessageWithUser>, sqlx::Error> {
        let messages = sqlx::query_as::<_, SupportMessageWithUser>(
            r#"
            SELECT 
                sm.*,
                u.name as user_name,
                u.role as user_role
            FROM support_messages sm
            JOIN users u ON sm.user_id = u.id
            WHERE sm.ticket_id = $1
            ORDER BY sm.created_at ASC
            "#
        )
        .bind(ticket_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    async fn update_ticket_status(
        &self,
        ticket_id: Uuid,
        status: TicketStatus,
    ) -> Result<SupportTicket, sqlx::Error> {
        let ticket = sqlx::query_as::<_, SupportTicket>(
            r#"
            UPDATE support_tickets
            SET status = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            RETURNING *
            "#
        )
        .bind(status)
        .bind(ticket_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(ticket)
    }

    async fn assign_ticket(
        &self,
        ticket_id: Uuid,
        assigned_to: Uuid,
    ) -> Result<SupportTicket, sqlx::Error> {
        let ticket = sqlx::query_as::<_, SupportTicket>(
            r#"
            UPDATE support_tickets
            SET assigned_to = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            RETURNING *
            "#
        )
        .bind(assigned_to)
        .bind(ticket_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(ticket)
    }
}