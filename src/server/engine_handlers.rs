use super::schemas;
use super::util;
use crate::engine;

use engine::api::AddOrUpdateOperation;
use engine::{ErrorCategory, ErrorCode};
use std::convert::Infallible;
use std::sync::Arc;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::Reply;

pub async fn draw_card_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Infallible> {
    info!("Handling: draw_card_for_user");

    match api.add_random_card_to_user(&user_id).await {
        Ok(card) => Ok(util::reply_with_value(&card, StatusCode::OK)),
        Err(e) => Ok(util::reply_with_engine_error(&e, get_http_code(&e))),
    }
}

pub async fn get_user_from_registry(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Infallible> {
    info!("Handling: get_user_from_registry");

    match api.get_user_by_id(&user_id).await {
        Ok(Some(user)) => Ok(util::reply_with_value(&user, StatusCode::OK)),
        Ok(None) => Ok(util::reply_empty(StatusCode::NOT_FOUND)),
        Err(e) => Ok(util::reply_with_engine_error(
            &e,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn add_user_to_registry(api: Arc<engine::Api>) -> Result<impl Reply, Infallible> {
    info!("Handling: add_user_to_registry");

    let user_id = uuid::Uuid::new_v4();
    match api.add_new_user(&user_id).await {
        Ok(_) => Ok(util::reply_with_value(&user_id, StatusCode::CREATED)),
        Err(e) => Ok(util::reply_with_engine_error(
            &e,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn add_card_to_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
    body: schemas::AddCardToUserRequest,
) -> Result<impl Reply, Infallible> {
    info!("Handling: add_card_to_user");

    // Validate explicit user ID parameter matches ID in body
    if user_id != body.user_id {
        return Ok(util::reply_with_error_message(
            "id mismatch".to_owned(),
            StatusCode::BAD_REQUEST,
        ));
    }

    match api.add_card_to_user(&user_id, &body.card_id).await {
        Ok(_) => Ok(util::reply_empty(StatusCode::OK)),
        Err(e) => Ok(util::reply_with_engine_error(&e, get_http_code(&e))),
    }
}

pub async fn claim_daily_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Infallible> {
    info!("Handling: claim_daily_for_user");

    match api.claim_daily_for_user(&user_id).await {
        Ok(new_currency) => Ok(util::reply_with_value(
            &schemas::ClaimDailyForUserResponse {
                user_id: user_id,
                currency: new_currency,
            },
            StatusCode::OK,
        )),
        Err(e) => {
            if let ErrorCode::DailyAlreadyClaimed = e.code {
                Ok(util::reply_with_engine_error(&e, StatusCode::CONFLICT))
            } else {
                Ok(util::reply_with_engine_error(&e, get_http_code(&e)))
            }
        }
    }
}

pub async fn get_card_from_compendium(
    card_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Infallible> {
    info!("Handling: get_card_from_compendium");

    match api.get_card_by_id(&card_id).await {
        Ok(Some(card)) => Ok(util::reply_with_value(&card, StatusCode::OK)),
        Ok(None) => Ok(util::reply_empty(StatusCode::NOT_FOUND)),
        Err(e) => Ok(util::reply_with_engine_error(
            &e,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn put_card_to_compendium(
    card_id: Uuid,
    api: Arc<engine::Api>,
    body: schemas::PutCardToCompendiumRequest,
) -> Result<impl Reply, Infallible> {
    info!("Handling: put_card_to_compendium");

    // Validate explicit ID parameter matches ID in body
    if card_id != body.card.id {
        return Ok(util::reply_with_error_message(
            "id mismatch".to_owned(),
            StatusCode::BAD_REQUEST,
        ));
    }

    match api.add_or_update_card_in_compendium(body.card).await {
        Ok(AddOrUpdateOperation::Add) => Ok(util::reply_empty(StatusCode::CREATED)),
        Ok(AddOrUpdateOperation::Update) => Ok(util::reply_empty(StatusCode::OK)),
        Err(e) => Ok(util::reply_with_engine_error(
            &e,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn list_users_from_registry(api: Arc<engine::Api>) -> Result<impl Reply, Infallible> {
    info!("Handling: list_users_from_registry");

    match api.get_user_ids().await {
        Ok(user_ids) => Ok(util::reply_with_value(
            &schemas::ListUsersFromRegistryResponse::from(user_ids),
            StatusCode::OK,
        )),
        Err(e) => Ok(util::reply_with_engine_error(
            &e,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn list_cards_from_compendium(api: Arc<engine::Api>) -> Result<impl Reply, Infallible> {
    info!("Handling: list_cards_from_compendium");

    match api.get_card_ids().await {
        Ok(card_ids) => Ok(util::reply_with_value(
            &schemas::ListCardsFromCompendiumResponse::from(card_ids),
            StatusCode::OK,
        )),
        Err(e) => Ok(util::reply_with_engine_error(
            &e,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn list_cards_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Infallible> {
    info!("Handling: list_cards_for_user");

    match api.get_owned_card_ids(&user_id).await {
        Ok(card_ids) => Ok(util::reply_with_value(
            &schemas::ListCardsForUserResponse::from(card_ids),
            StatusCode::OK,
        )),
        Err(e) => Ok(util::reply_with_engine_error(&e, get_http_code(&e))),
    }
}

fn get_http_code(error: &engine::Error) -> http::StatusCode {
    match error.classify() {
        ErrorCategory::BadRequest => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
