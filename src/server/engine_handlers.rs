use super::schemas;
use crate::engine;

use engine::api::AddOrUpdateOperation;
use engine::{ErrorCategory, ErrorCode};
use std::sync::Arc;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::{reject, reply, Rejection, Reply};

pub async fn draw_card_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: draw_card_for_user");

    match api.add_random_card_to_user(&user_id).await {
        Ok(card) => Ok(reply::with_status(reply::json(&card), StatusCode::OK)),
        Err(e) => Err(reject::custom(EngineError::new(e))),
    }
}

pub async fn delete_user_from_registry(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: delete_user_from_registry");

    match api.delete_user(&user_id).await {
        Ok(_) => Ok(reply::with_status(reply::reply(), StatusCode::OK)),
        Err(e) => {
            if let ErrorCode::UserNotFound = e.code {
                Err(reject::custom(EngineError {
                    error: e,
                    status_code: StatusCode::NOT_FOUND,
                }))
            } else {
                Err(reject::custom(EngineError::new(e)))
            }
        }
    }
}

pub async fn get_user_from_registry(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: get_user_from_registry");

    match api.get_user_by_id(&user_id).await {
        Ok(Some(user)) => Ok(reply::with_status(reply::json(&user), StatusCode::OK)),
        Ok(None) => Err(reject::not_found()),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn add_user_to_registry(api: Arc<engine::Api>) -> Result<impl Reply, Rejection> {
    info!("Handling: add_user_to_registry");

    let user_id = uuid::Uuid::new_v4();
    match api.add_new_user(&user_id).await {
        Ok(_) => Ok(reply::with_status(
            reply::json(&user_id),
            StatusCode::CREATED,
        )),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn add_card_to_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
    body: schemas::AddCardToUserRequest,
) -> Result<impl Reply, Rejection> {
    info!("Handling: add_card_to_user");

    // Validate explicit user ID parameter matches ID in body
    if user_id != body.user_id {
        return Err(reject::custom(MessageError {
            error_message: "id mismatch".to_owned(),
            status_code: StatusCode::BAD_REQUEST,
        }));
    }

    match api.add_card_to_user(&user_id, &body.card_id).await {
        Ok(_) => Ok(reply::with_status(reply::reply(), StatusCode::OK)),
        Err(e) => Err(reject::custom(EngineError::new(e))),
    }
}

pub async fn claim_daily_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: claim_daily_for_user");

    match api.claim_daily_for_user(&user_id).await {
        Ok(currency) => Ok(reply::with_status(
            reply::json(&schemas::ClaimDailyForUserResponse { user_id, currency }),
            StatusCode::OK,
        )),
        Err(e) => {
            let status_code;
            if let ErrorCode::DailyAlreadyClaimed = e.code {
                status_code = StatusCode::CONFLICT;
            } else {
                status_code = get_http_code(&e);
            }

            Err(reject::custom(EngineError {
                error: e,
                status_code,
            }))
        }
    }
}

pub async fn get_card_from_compendium(
    card_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: get_card_from_compendium");

    match api.get_card_by_id(&card_id).await {
        Ok(Some(card)) => Ok(reply::with_status(reply::json(&card), StatusCode::OK)),
        Ok(None) => Err(reject::not_found()),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn put_card_to_compendium(
    card_id: Uuid,
    api: Arc<engine::Api>,
    body: schemas::PutCardToCompendiumRequest,
) -> Result<impl Reply, Rejection> {
    info!("Handling: put_card_to_compendium");

    // Validate explicit ID parameter matches ID in body
    if card_id != body.card.id {
        return Err(reject::custom(MessageError {
            error_message: "id mismatch".to_owned(),
            status_code: StatusCode::BAD_REQUEST,
        }));
    }

    match api.add_or_update_card_in_compendium(body.card).await {
        Ok(AddOrUpdateOperation::Add) => {
            Ok(reply::with_status(reply::reply(), StatusCode::CREATED))
        }
        Ok(AddOrUpdateOperation::Update) => Ok(reply::with_status(reply::reply(), StatusCode::OK)),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn list_users_from_registry(api: Arc<engine::Api>) -> Result<impl Reply, Rejection> {
    info!("Handling: list_users_from_registry");

    match api.get_user_ids().await {
        Ok(user_ids) => Ok(reply::with_status(
            reply::json(&schemas::ListUsersFromRegistryResponse::from(user_ids)),
            StatusCode::OK,
        )),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn list_cards_from_compendium(api: Arc<engine::Api>) -> Result<impl Reply, Rejection> {
    info!("Handling: list_cards_from_compendium");

    match api.get_card_ids().await {
        Ok(card_ids) => Ok(reply::with_status(
            reply::json(&schemas::ListCardsFromCompendiumResponse::from(card_ids)),
            StatusCode::OK,
        )),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn list_cards_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: list_cards_for_user");

    match api.get_owned_card_ids(&user_id).await {
        Ok(card_ids) => Ok(reply::with_status(
            reply::json(&schemas::ListCardsForUserResponse::from(card_ids)),
            StatusCode::OK,
        )),
        Err(e) => Err(reject::custom(EngineError::new(e))),
    }
}

fn get_http_code(error: &engine::Error) -> http::StatusCode {
    match error.classify() {
        ErrorCategory::BadRequest => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn handle_engine_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(e) = err.find::<EngineError>() {
        let json = warp::reply::json(&e.error);
        Ok(warp::reply::with_status(json, e.status_code))
    } else if let Some(e) = err.find::<MessageError>() {
        let json = warp::reply::json(&ErrorResponse {
            error_message: e.error_message.clone(),
        });
        Ok(warp::reply::with_status(json, e.status_code))
    } else {
        Err(err)
    }
}

#[derive(Debug)]
struct EngineError {
    pub error: engine::Error,
    pub status_code: StatusCode,
}

impl EngineError {
    fn new(error: engine::Error) -> EngineError {
        let status_code = get_http_code(&error);
        EngineError { error, status_code }
    }
}

impl reject::Reject for EngineError {}

#[derive(Debug)]
struct MessageError {
    pub error_message: String,
    pub status_code: StatusCode,
}

impl reject::Reject for MessageError {}

#[derive(serde::Serialize)]
struct ErrorResponse {
    error_message: String,
}
