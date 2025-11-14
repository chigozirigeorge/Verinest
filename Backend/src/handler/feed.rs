use std::sync::Arc;
use axum::{Extension, Json};
use serde::Deserialize;
use uuid::Uuid;
use axum::http::HeaderMap;

use crate::{AppState, error::HttpError};
use crate::services::ranking::RankingService;
use crate::services::reco_db::RecoDB;
use crate::recommendation_models::{RankedItemMeta};

use crate::utils::token as token_utils;

#[derive(Deserialize)]
pub struct FeedQuery {
    pub user_id: Option<Uuid>,
    pub limit: Option<i64>,
}

/// GET /labour/feed
/// If Authorization: Bearer <token> is present and valid, the token's subject is used as the user_id.
/// Otherwise the `user_id` query parameter is required.
pub async fn get_feed(
    Extension(app_state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<FeedQuery>,
) -> Result<Json<serde_json::Value>, HttpError> {
    // Determine user_id: prefer Authorization bearer token
    let mut user_id_opt: Option<Uuid> = None;
    if let Some(auth_val) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = auth_val.to_str() {
            if s.to_lowercase().starts_with("bearer ") {
                let token = s[7..].trim();
                if !token.is_empty() {
                    match token_utils::decode_token_claims(token.to_string(), app_state.env.jwt_secret.as_bytes()) {
                        Ok(claims) => {
                            if let Ok(uid) = Uuid::parse_str(&claims.sub) {
                                user_id_opt = Some(uid);
                            }
                        }
                        Err(_) => {
                            // ignore invalid token here; we'll fall back to query param if provided
                        }
                    }
                }
            }
        }
    }

    let user_id = if let Some(uid) = user_id_opt {
        uid
    } else if let Some(qid) = query.user_id {
        qid
    } else {
        return Err(HttpError::bad_request("user_id query parameter is required or provide Authorization header"));
    };

    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let ranking = RankingService::new(app_state.db_client.clone(), 3600);

    // Try cached ranked first
    let ranked = match ranking.get_cached_ranked(user_id).await {
        Ok(Some(v)) => v,
        _ => match ranking.compute_ranked_items(user_id, limit).await {
            Ok(v) => v,
            Err(e) => return Err(HttpError::server_error(e.to_string())),
        },
    };

    // Extract item ids and fetch detailed jobs for those ids
    let ids: Vec<Uuid> = ranked.iter().map(|(id, _score)| *id).collect();
    let reco_db = RecoDB::new(app_state.db_client.clone());
    let jobs = reco_db.get_jobs_by_ids(&ids).await.map_err(|e| HttpError::server_error(e.to_string()))?;

    // Map job id -> job payload for quick lookup
    use serde_json::Value as JsonValue;
    use std::collections::HashMap;
    let mut job_map: HashMap<Uuid, JsonValue> = HashMap::new();
    for job in jobs.into_iter() {
        if let Ok(v) = serde_json::to_value(&job) {
            job_map.insert(job.id, v);
        }
    }

    // Build enriched items in original order
    let mut items: Vec<serde_json::Value> = Vec::new();
    for (id, score) in ranked.into_iter() {
        if let Some(payload) = job_map.get(&id) {
            items.push(serde_json::json!({
                "id": id,
                "score": score,
                "payload": payload
            }));
        }
    }

    let resp = serde_json::json!({
        "user_id": user_id,
        "items": items,
        "generated_at": chrono::Utc::now()
    });

    Ok(Json(resp))
}
