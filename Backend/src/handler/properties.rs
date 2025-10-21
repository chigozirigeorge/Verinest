use std::sync::Arc;
use axum::{
    extract::{Path, Query},
    middleware,
    response::IntoResponse,
    routing::{get, post, put},
    Extension, Json, Router,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db::{userdb::UserExt, propertydb::{PropertyExt, PropertySearchFilters}},
    dtos::{
        userdtos::{RequestQueryDto, Response},
        propertydtos::{CreatePropertyDto, PropertyFilterDto, AgentVerificationDto, LawyerVerificationDto}
    },
    error::HttpError,
    middleware::{role_check, JWTAuthMiddeware},
    models::{
        propertymodel::{
            PropertyStatus, ListingType, PropertyType, CurrencyType
        },
        usermodel::UserRole,
    },
    AppState,
};

pub fn property_handler() -> Router {
    Router::new()
        .route(
            "/create", 
            post(create_property).layer(middleware::from_fn(|state, req, next| {
                role_check(state, req, next, vec![UserRole::Landlord])
            })),
        )
        .route(
           "/my-properties",
           get(get_landlord_properties).layer(middleware::from_fn(|state, req, next| {
            role_check(state, req, next, vec![UserRole::Landlord])
           })),
        )
        .route(
            "/for-agent-verification",
            get(get_properties_for_agent).layer(middleware::from_fn(|state, req, next| {
                role_check(state, req, next, vec![UserRole::Agent])
            })),
        )
        .route(
            "/agent-verfy/:property_id", 
            post(agent_verify_property).layer(middleware::from_fn(|state, req, next| {
                role_check(state, req, next, vec![UserRole::Agent])
            })),
        )
        .route(
            "/assign-agent/:property_id",
            put(assign_agent_to_property).layer(middleware::from_fn(|state, req, next| {
                role_check(state, req, next, vec![UserRole::Admin, UserRole::Moderator])
            }))
        )
        .route(
            "/for-lawyer-verification",
            get(get_properties_for_lawyer).layer(middleware::from_fn(|state, req, next| {
                role_check(state, req, next, vec![UserRole::Lawyer])
            })),
        )
        .route(
            "/lawyer-verify/:property_id",
            post(lawyer_verify_property).layer(middleware::from_fn(|state, req, next| {
                role_check(state, req, next, vec![UserRole::Lawyer])
            }))
        )
        .route("/active", get(get_active_properties))
        .route("/:property_id", get(get_property_by_id))
        .route("/:property_id/verification-history", get(get_verification_history))
}

//Landlord creates property
pub async fn create_property(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<CreatePropertyDto>
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    //Verify the user is a landlord
    if user.user.role != UserRole::Landlord {
        return Err(HttpError::unauthorized("Only landlords can create properties"));
    }

    let property = app_state.db_client
        .create_property(user.user.id, body)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique_violation") || e.to_string().contains("Property already exists") {
                HttpError::bad_request("A similar property already exists at this location")
            } else {
                HttpError::server_error(e.to_string())
            }
        })?;

    //Get landlord info
    let landlord_username = user.user.username.clone();
    let filtered_property = PropertyFilterDto::from_property(&property, landlord_username);

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Property created successfully and sent for agent verification",
        "data": {
            "property": filtered_property
        }
    })))
}

pub async fn get_landlord_properties(
    Query(query_params): Query<RequestQueryDto>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    query_params.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let page = query_params.page.unwrap_or(1) as u32;
    let limit = query_params.limit.unwrap_or(10);

    let properties = app_state.db_client
        .get_properties_by_landlord(user.user.id, page, limit)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let landlord_username = user.user.username.clone();
    let filtered_properies:Vec<PropertyFilterDto> = properties
        .iter()
        .map(|p| PropertyFilterDto::from_property(p, landlord_username.clone()))
        .collect();

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "properties": filtered_properies,
            "pagination": {
                "page": page,
                "limit": limit,
                "total": filtered_properies.len()
            }
        }
    })))

}

pub async fn get_properties_for_agent(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let properties = app_state.db_client
        .get_properties_for_agent_verification(user.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let mut property_data = Vec::new();

    for property in properties {
        let landlord = app_state.db_client
            .get_user(Some(property.landlord_id), None, None, None)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::server_error("Landlord not Found"))?;

        let mut filtered_property = PropertyFilterDto::from_property(&property, landlord.name);

        //Adding additional verification info for agents
        let property_info = serde_json::json!({
            "property": filtered_property,
            "verification_info": {
                "address": property.address,
                "landmark": property.landmark,
                "coordinates": {
                    "latitude": property.latitude,
                    "longitude": property.longitude
                },
                "landlord_contact": {
                    "name": landlord.name.clone(),
                    
                },
                "property_photos": property.property_photos.0,
            }
        });

        property_data.push(property_info);
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "properties": property_data,
            "total": property_data.len()
        }
    })))
}

pub async fn agent_verify_property(
    Path(property_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<AgentVerificationDto>,
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    //Set property_id from path
   let property_id = body.property_id;

    //Verifying the property is assigned to this agent
    let property = app_state.db_client
        .get_property_by_id(property_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::server_error("Property not found"))?;

    if property.agent_id != Some(user.user.id) {
        return Err(HttpError::unauthorized("You are not assigned to verify this property"))
    }

    if property.status != PropertyStatus::AwaitingAgent {
        return Err(HttpError::bad_request("Property is not awaiting agent verification"));
    }

    let updated_property = app_state.db_client
        .agent_verify_property(user.user.id, body)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    //Get landlord info
    let landlord = app_state.db_client
        .get_user(Some(updated_property.landlord_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::server_error("Landlord not found"))?;

    let filtered_property = PropertyFilterDto::from_property(&updated_property, landlord.name);

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": if updated_property.status == PropertyStatus::AwaitingLawyer {
            "Property verified successfully and sent to lawyer for document verification"
        } else {
            "Property verification completed"
        },
        "data": {
            "property": filtered_property
        }
    })))
}

pub async fn assign_agent_to_property(
    Path(property_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let agent_id = body["agent_id"]
        .as_str()
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| HttpError::bad_request("Valid agent_id is required"))?;

    //verify agent exist and has correct role
    let agent = app_state.db_client
        .get_user(Some(agent_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(||HttpError::bad_request("Agent not found"))?;

    if agent.role != UserRole::Agent {
        return Err(HttpError::bad_request("User is not an agent"));
    }

    let updated_property = app_state.db_client
        .assign_agent_to_property(property_id, agent_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Agent assigned successfully",
        "data": {
            "property_id": updated_property.id,
            "agent": {
                "id": agent.id,
                "name": agent.name,
                "email": agent.email
            }
        }
    })))
}

//Lawyer endpoints
pub async fn get_properties_for_lawyer(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
) -> Result<impl IntoResponse, HttpError> {
    let properties = app_state.db_client
        .get_properties_for_lawyer_verification(user.user.id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let mut property_data = Vec::new();
    for property in properties {
        //Get landlord and agent info
        let landlord = app_state.db_client
            .get_user(Some(property.landlord_id), None, None, None)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::server_error("Landlord not found"))?;

        let agent = if let Some(agent_id) = property.agent_id {
            app_state.db_client
        .get_user(Some(agent_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
    } else {
        None
    };

    let filtered_property = PropertyFilterDto::from_property(&property, landlord.name.clone());

    let property_info = serde_json::json!({
        "property": filtered_property,
        "document_verification_info": {
            "documents": {
                "certificate_of_ownership": property.certificate_of_occupancy,
                "deed_of_agreement": property.deed_of_agreement,
                "survey_plan": property.survey_plan,
                "building_plan_approval": property.building_plan_approval
            },
            "landlord_info": {
                "name": landlord.name.clone(),
                "email": landlord.email,
                "verification_status": landlord.verification_status,
                "nin_number": landlord.nin_number
            },
            "agent_verification": {
                "agent_name": agent.as_ref().map(|a| a.name.clone()),
                "verification_notes": property.agent_verification_notes,
                "verification_photos": property.agent_verification_photos,
                "verified_at": property.agent_verified_at
            }
        }
    });
        property_data.push(property_info);
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "properties": property_data,
            "total": property_data.len()
        }
    })))
}

pub async fn lawyer_verify_property(
    Path(property_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(user): Extension<JWTAuthMiddeware>,
    Json(body): Json<LawyerVerificationDto>
) -> Result<impl IntoResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    //Set property_id from path
    let property_id= body.property_id;

    //Verify the property is awaiting lawyer verification

    let property = app_state.db_client
        .get_property_by_id(property_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::bad_request("Property not Found"))?;

    if property.status != PropertyStatus::AwaitingLawyer {
        return Err(HttpError::unauthorized("Property is not awaiting lawyer verification"));
    }

    let updated_property = app_state.db_client
        .lawyer_verify_property(user.user.id, body)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    //Get landlord info
    let landlord = app_state.db_client
        .get_user(Some(updated_property.landlord_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::server_error("Landlord not found"))?;

    let filtered_property = PropertyFilterDto::from_property(&updated_property, landlord.name);

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": if updated_property.status == PropertyStatus::Active {
            "Property documents verified successfully and property is now live"
        } else {
            "Property document verification completed"
        },
        "data": {
            "property": filtered_property
        }
    })))
}

pub async fn get_active_properties(
    Query(query_params): Query<RequestQueryDto>,
    Extension(app_state): Extension<Arc<AppState>>,
    Query(filters): Query<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    query_params.validate()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let page = query_params.page.unwrap_or(1) as u32;
    let limit = query_params.limit.unwrap_or(10);

    //Parse search FIlters
    let search_filters = PropertySearchFilters {
        property_type: filters["property_type"].as_str().and_then(|t| serde_json::from_str(&format!("\"{}\"", t)).ok()),
        listing_type: filters["listing_type"].as_str().and_then(|t| serde_json::from_str(&format!("\"{}\"", t)).ok()),
        min_price: filters["min_price"].as_i64(),
        max_price: filters["max_price"].as_i64(),
        city: filters["city"].as_str().map(String::from),
        state: filters["state"].as_str().map(String::from),
        country: filters["country"].as_str().map(String::from),
        bedrooms: filters["bedrooms"].as_i64().map(|b| b as i32),
        bathrooms: filters["bathrooms"].as_i64().map(|b| b as i32),
    };

    let properties = app_state.db_client
        .get_active_properties(search_filters, page, limit)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    let mut property_data = Vec::new();

    for property in properties {
        let landlord = app_state.db_client
            .get_user(Some(property.landlord_id), None, None, None)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?
            .ok_or_else(|| HttpError::server_error("Landlord not found"))?;

        let filtered_property = PropertyFilterDto::from_property(&property, landlord.name);
        property_data.push(filtered_property);
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "properties": property_data,
            "pagination": {
                "page": page,
                "limit": limit,
                "total": property_data.len()
            }
        }
    })))
}

pub async fn get_property_by_id(
    Path(property_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpError> {
    let property = app_state.db_client
        .get_property_by_id(property_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::bad_request("Property not found"))?;
    
    let landlord = app_state.db_client
        .get_user(Some(property.landlord_id), None, None, None)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?
        .ok_or_else(|| HttpError::server_error("Landlord not found"))?;

    let filtered_property = PropertyFilterDto::from_property(&property, landlord.username);

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "property": filtered_property
        }
    })))
}

pub async fn get_verification_history(
    Path(property_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, HttpError> {
    let verification = app_state.db_client
        .get_property_verification_history(property_id)
        .await
        .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "verification": verification
        }
    })))
}