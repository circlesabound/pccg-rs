use super::util;
use crate::engine;
use crate::models;

use engine::api::AddOrUpdateOperation;
use std::convert::Infallible;
use std::sync::Arc;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::Reply;

pub async fn get_random(api: Arc<engine::Api>) -> Result<impl Reply, Infallible> {
    info!("Handling: get_random");

    match api.get_random_card().await {
        Ok(Some(card)) => Ok(util::reply_with_value(&card, StatusCode::OK)),
        Ok(None) => Ok(util::reply_empty(StatusCode::NO_CONTENT)),
        Err(e) => Ok(util::reply_with_error(
            &e,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn add_user(api: Arc<engine::Api>) -> Result<impl Reply, Infallible> {
    info!("Handling: add_user");

    let id = uuid::Uuid::new_v4();
    match api.add_new_user(id).await {
        Ok(_) => Ok(util::reply_with_value(&id, StatusCode::CREATED)),
        Err(e) => Ok(util::reply_with_error(
            &e,
            StatusCode::INTERNAL_SERVER_ERROR
        )),
    }
}

pub async fn get_card(id: Uuid, api: Arc<engine::Api>) -> Result<impl Reply, Infallible> {
    info!("Handling: get_card");

    match api.get_card_by_id(id).await {
        Ok(Some(card)) => Ok(util::reply_with_value(&card, StatusCode::OK)),
        Ok(None) => Ok(util::reply_empty(StatusCode::NOT_FOUND)),
        Err(e) => Ok(util::reply_with_error(
            &e,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn put_card(
    id: Uuid,
    api: Arc<engine::Api>,
    card: models::Card,
) -> Result<impl Reply, Infallible> {
    info!("Handling: put_card");

    // Validate explicit ID paramter matches ID in card
    if id != card.id {
        return Ok(util::reply_with_error(
            &"id mismatch",
            StatusCode::BAD_REQUEST,
        ));
    }

    match api.add_or_update_card_in_compendium(card).await {
        Ok(AddOrUpdateOperation::Add) => Ok(util::reply_empty(StatusCode::CREATED)),
        Ok(AddOrUpdateOperation::Update) => Ok(util::reply_empty(StatusCode::OK)),
        Err(e) => Ok(util::reply_with_error(
            &e,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}
