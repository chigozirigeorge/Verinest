// use std::{path::Path, sync::Arc};

// use chrono::{DateTime, Utc};
// use serde::{Deserialize, Serialize};
// use tract_core::{model::{Graph, TypedFact}, ops::TypedOp, plan::SimplePlan};
// use uuid::Uuid;
// use validator::Validate;

// use crate::models::usermodel::{VerificationStatus, VerificationType};

// #[derive(Debug, Deserialize, Serialize, Validate)]
// pub struct NinVerificationRequest {
//     #[validate(length(min = 11, max = 11, message = "NIN must be 11 digits"))]
//     pub nin_number: String,
//     pub verification_type: VerificationType,

//     #[validate(length(min = 3, message = "Nationality is required"))]
//     pub nationality: String,

//     pub dob: Option<DateTime<Utc>>,
//     pub lga: Option<String>,
//     pub nearest_landmark: Option<String>,
// }

// #[derive(Debug,Deserialize, Serialize)]
// pub struct FacialVericationRequest {
//     pub facial_data: String,  //Base64 encoded image or facial features
//     pub verification_document_id: String, //References to the uploaded document
// }


// #[derive(Debug, Deserialize, Serialize)]
// pub struct VerificationDocument {
//     pub id: Uuid,
//     pub user_id: Uuid,
//     pub document_type: VerificationType,
//     pub document_url: String,
//     pub status: VerificationStatus,
//     pub reviewed_by: Option<Uuid>,
//     pub review_notes: Option<String>,
//     pub created_at: DateTime<Utc>,
//     pub updated_at: DateTime<Utc>,
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub struct VerificationResponse {
//     pub status: VerificationStatus,
//     pub message: String,
//     pub next_steps: Option<Vec<String>>,
//     pub estimated_completion_time: Option<i32>, //hours
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub struct VerificationResult {
//     pub is_match: bool,
//     pub confidence: f32,
//     pub message: Option<String>
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub struct UploadForm {
//     pub document_type: VerificationType,
//     pub file: Vec<u8>
// }

// #[derive(Debug, Deserialize)]
// pub struct ApprovalRequest {
//     pub user_id: Uuid,
//     pub notes: Option<String>
// }

// #[derive(Debug, Clone)]
// pub struct FaceRecognitionService {
//     model: Arc<RunnableModel>,
// }

// type RunnableModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

// impl FaceRecognitionService {
//     pub fn new() -> Result<Self, String> {
//         let model_path = "../../models/face_recognition.onnx";

//         //load the ONNX models
//         let model = tract_onnx::onnx()
//             .model_for_path(Path::new(model_path))
//             .map_err(|e| format!("Failed to load model: {}", e))?
//             .into_optimized()
//             .map_err(|e| format!("Failed to optimize model: {}", e))?
//             .into_runnable()
//             .map_err(|e| format!("Failed to create runnable model: {}", e))?;

//         Ok(Self {
//             model: Arc::new(model),
//         })
//     }
// }