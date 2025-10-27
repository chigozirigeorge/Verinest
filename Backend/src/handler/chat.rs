use std::sync::Arc;
use axum::{extract::{Path, Query},
    response::IntoResponse,
    routing::{get, post, put},
    Extension, Json, Router
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db::{
        chatdb::ChatExt,
        labourdb::LaborExt,
        userdb::UserExt,
    },
    error::HttpError,
    middleware::JWTAuthMiddeware,
    models::chatnodels::*,
    AppState,
};

pub fn chat_handler() -> Router {
    Router::new()
        .route("/chats", get(get_user_chats).post(create_chat))
        .route("/chats/:chat_id", get(get_chat_details))
        .route("/chats/:chat_id/messages", get(get_messages).post(send_message))
        .route("/chats/:chat_id/read", put(mark_chat_as_read))
        .route("/chats/:chat_id/contract-proposal", post(propose_contract_from_chat))
        .route("/contract-proposals/:proposal_id/respond", put(respond_to_proposal))
        .route("/unread-count", get(get_unread_count))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateChatDto {
    pub other_user_id: Uuid,
    pub job_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChatWithDetails {
    pub chat: Chat,
    pub other_user: ChatParticipant,
    pub last_message: Option<Message>,
    pub unread_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChatParticipant {
    pub id: Uuid,
    pub name: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

pub async fn create_chat(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreateChatDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    //Verify the other user exist
    let other_user = app_state.db_client
        .get_user(Some(body.other_user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    //if a Job is provided verify it exist
    if let Some(job_id) = body.job_id {
        let _ = app_state.db_client
            .get_job_by_id(job_id)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(||HttpError::not_found("Job not found"))?;
    }

    let chat = app_state.db_client
        .create_or_get_chat(auth.user.id, body.other_user_id, body.job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response = ChatWithDetails {
        chat: chat.clone(),
        other_user : ChatParticipant { 
            id: other_user.id, 
            name: other_user.name, 
            username: other_user.username,
            avatar_url: other_user.avatar_url 
        },
        last_message: None,
        unread_count: 0,
    };

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": response
    })))
}

pub async fn get_user_chats (
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, HttpError> {
    let page = pagination.page.unwrap_or(1);
    let limit = pagination.limit.unwrap_or(20) as i64;
    let offset = ((page -1) * limit as u32) as i64;

    let chats = app_state.db_client
        .get_user_chats(auth.user.id, limit, offset)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let mut chat_details = Vec::new();

    for chat in chats {
        let other_user_id = if chat.participant_one_id == auth.user.id {
            chat.participant_two_id
        } else {
            chat.participant_one_id
        };

        let other_user = app_state.db_client
            .get_user(Some(other_user_id), None, None, None)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::not_found("User not found"))?;

        //get last messages
        let messages = app_state.db_client
            .get_chat_messages(chat.id, 1, 0)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        let last_message = messages.first().cloned();

        //get unread count for this chat
        let unread_count = sqlx::query_scalar::<_, i64>(
            r#"
                SELECT COUNT(*)
                FROM messages
                WHERE chat_id = $1
                    AND sender_id != $2
                    AND is_read = false
            "#
        )
        .bind(chat.id)
        .bind(auth.user.id)
        .fetch_one(&app_state.db_client.pool)
        .await
        .unwrap_or(0);

        chat_details.push(ChatWithDetails {
            chat: chat.clone(),
            other_user: ChatParticipant { 
                id: other_user.id, 
                name: other_user.name, 
                username: other_user.username , 
                avatar_url: other_user.avatar_url 
            },
            last_message,
            unread_count,
        });
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": chat_details
    })))
}

pub async fn get_chat_details(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(chat_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let chat = app_state.db_client
        .get_chat_by_id(chat_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Chat not found"))?;

    //verify user is participant
    if chat.participant_one_id != auth.user.id && chat.participant_two_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to view this chat"));
    }

    //Get other user
    let other_user_id = if chat.participant_one_id == auth.user.id {
        chat.participant_two_id
    } else {
        chat.participant_one_id
    };

    let other_user = app_state.db_client
        .get_user(Some(other_user_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "chat": chat,
            "other_user": ChatParticipant {
                id: other_user.id,
                name: other_user.name,
                username: other_user.username,
                avatar_url: other_user.avatar_url
            }
        }
    })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct SendMessaageDto {
    #[validate(length(min = 1, max = 5000))]
    pub content: String,
    pub message_type: Option<MessageType>,
    pub metadata: Option<serde_json::Value>,
}

pub async fn send_message(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(chat_id): Path<Uuid>,
    Json(body): Json<SendMessaageDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    //verify that the chat exist and the user is a participant of that chat first
    let chat = app_state.db_client
        .get_chat_by_id(chat_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Chat not found"))?;

    if chat.participant_one_id != auth.user.id && chat.participant_two_id != auth.user.id {
        return  Err(HttpError::unauthorized("Not authorized to send message in this chat"));
    }

    let message_type = body.message_type.unwrap_or(MessageType::Text);

    let message = app_state.db_client
        .send_message(
            chat_id, 
            auth.user.id, 
            message_type, 
            body.content, 
            body.metadata
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let other_user_id = if chat.participant_one_id == auth.user.id {
        chat.participant_two_id
    } else {
        chat.participant_one_id
    };

    let _ = app_state.notification_service
        .notify_new_message(other_user_id, &auth.user.name, &message)
        .await;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": message
    })))
}


pub async fn get_messages(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(chat_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, HttpError> {
    // Verify chat exists and user is participant
    let chat = app_state.db_client
        .get_chat_by_id(chat_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Chat not found"))?;
    
    if chat.participant_one_id != auth.user.id && chat.participant_two_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to view messages in this chat"));
    }
    
    let page = pagination.page.unwrap_or(1);
    let limit = pagination.limit.unwrap_or(50) as i64;
    let offset = ((page - 1) * limit as u32) as i64;
    
    let messages = app_state.db_client
        .get_chat_messages(chat_id, limit, offset)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": messages
    })))
}

pub async fn mark_chat_as_read(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(chat_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    // Verify chat exists and user is participant
    let chat = app_state.db_client
        .get_chat_by_id(chat_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Chat not found"))?;
    
    if chat.participant_one_id != auth.user.id && chat.participant_two_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized"));
    }
    
    app_state.db_client
        .mark_messages_as_read(chat_id, auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Messages marked as read"
    })))
}

pub async fn get_unread_count(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let count = app_state.db_client
        .get_unread_count(auth.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "unread_count": count
        }
    })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct ProposeContractDto {
    pub job_id: Uuid,
    #[validate(range(min = 0.0))]
    pub agreed_rate: f64,
    #[validate(range(min = 1))]
    pub agreed_timeline: i32,
    #[validate(length(min = 10, max = 5000))]
    pub terms: String,
}

pub async fn propose_contract_from_chat(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(chat_id): Path<Uuid>,
    Json(body): Json<ProposeContractDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;
    
    // Verify chat exists and user is participant
    let chat = app_state.db_client
        .get_chat_by_id(chat_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Chat not found"))?;
    
    if chat.participant_one_id != auth.user.id && chat.participant_two_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized"));
    }
    
    // Verify job exists
    let job = app_state.db_client
        .get_job_by_id(body.job_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::not_found("Job not found"))?;
    
    // Determine worker and employer
    let other_user_id = if chat.participant_one_id == auth.user.id {
        chat.participant_two_id
    } else {
        chat.participant_one_id
    };
    
    let (worker_id, employer_id) = if job.employer_id == auth.user.id {
        (other_user_id, auth.user.id)
    } else {
        (auth.user.id, other_user_id)
    };
    
    // Create message with contract proposal
    let proposal_metadata = serde_json::json!({
        "job_id": body.job_id,
        "agreed_rate": body.agreed_rate,
        "agreed_timeline": body.agreed_timeline,
    });
    
    let message_content = format!(
        "Contract Proposal for Job: {}\nRate: ₦{}\nTimeline: {} days\n\nTerms:\n{}",
        job.title,
        body.agreed_rate,
        body.agreed_timeline,
        body.terms
    );
    
    let message = app_state.db_client
        .send_message(
            chat_id,
            auth.user.id,
            MessageType::ContractProposal,
            message_content,
            Some(proposal_metadata),
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Create contract proposal record
    let proposal = app_state.db_client
        .create_contract_proposal(
            chat_id,
            message.id,
            body.job_id,
            auth.user.id,
            worker_id,
            employer_id,
            body.agreed_rate,
            body.agreed_timeline,
            body.terms,
        )
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;
    
    // Send notification
    let _ = app_state.notification_service
        .notify_contract_proposal(other_user_id, &auth.user.name, &job)
        .await;
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "message": message,
            "proposal": proposal
        }
    })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct RespondToProposalDto {
    #[validate(length(min = 1))]
    pub response: String, // "accepted" or "rejected"
}

pub async fn respond_to_proposal(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(auth): Extension<JWTAuthMiddeware>,
    Path(proposal_id): Path<Uuid>,
    Json(body): Json<RespondToProposalDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;
    
    if body.response != "accepted" && body.response != "rejected" {
        return Err(HttpError::bad_request("Response must be 'accepted' or 'rejected'"));
    }
    
    // Get proposal
    let proposal = sqlx::query_as::<_, ContractProposal>(
        r#"
        SELECT id, chat_id, message_id, job_id, proposed_by, worker_id,
               employer_id, agreed_rate, agreed_timeline, terms, status,
               created_at, responded_at
        FROM contract_proposals
        WHERE id = $1
        "#
    )
    .bind(proposal_id)
    .fetch_optional(&app_state.db_client.pool)
    .await
    .map_err(|e| HttpError::server_error(e.to_string()))?
    .ok_or_else(|| HttpError::not_found("Proposal not found"))?;
    
    // Verify user is the recipient (not the proposer)
    if proposal.proposed_by == auth.user.id {
        return Err(HttpError::bad_request("Cannot respond to your own proposal"));
    }
    
    // Verify user is either worker or employer in the proposal
    if proposal.worker_id != auth.user.id && proposal.employer_id != auth.user.id {
        return Err(HttpError::unauthorized("Not authorized to respond to this proposal"));
    }
    
    if body.response == "accepted" {
        // Create contract and assign worker
        let contract = app_state.db_client
            .create_job_contract(
                proposal.job_id,
                proposal.employer_id,
                proposal.worker_id,
                proposal.agreed_rate,
                proposal.agreed_timeline,
                proposal.terms,
            )
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Assign worker to job and create escrow
        let (job, escrow) = app_state.db_client
            .assign_worker_to_job(proposal.job_id, proposal.worker_id)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Update proposal status
        let updated_proposal = app_state.db_client
            .respond_to_contract_proposal(proposal_id, "accepted".to_string())
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Send notifications
        let _ = app_state.notification_service
            .notify_contract_accepted(proposal.proposed_by, &job)
            .await;
        
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Contract proposal accepted",
            "data": {
                "proposal": updated_proposal,
                "contract": contract,
                "job": job,
                "escrow": escrow
            }
        })))
    } else {
        // Update proposal status to rejected
        let updated_proposal = app_state.db_client
            .respond_to_contract_proposal(proposal_id, "rejected".to_string())
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;
        
        // Send notification
        let _ = app_state.notification_service
            .notify_contract_rejected(proposal.proposed_by)
            .await;
        
        Ok(Json(serde_json::json!({
            "status": "success",
            "message": "Contract proposal rejected",
            "data": {
                "proposal": updated_proposal
            }
        })))
    }
}