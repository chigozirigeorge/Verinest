use std::sync::Arc;
use axum::{Extension, Json};
use serde_json::json;
use crate::AppState;
use crate::recommendation_models::Interaction;
use crate::services::reco_db::RecoDB;

// POST /api/debug/reco/push
// Accepts a JSON Interaction and pushes it into the reco pipeline (Redis stream + list).
pub async fn push_reco_event(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(interaction): Json<Interaction>,
) -> Json<serde_json::Value> {
    let reco = RecoDB::new(app_state.db_client.clone());
    let res = reco.push_event_stream(&interaction).await;
    match res {
        Ok(_) => Json(json!({"status":"ok","message":"pushed"})),
        Err(e) => Json(json!({"status":"error","message":e.to_string()})),
    }
}
