use crate::engine;
use crate::models;
use super::util;

use std::convert::Infallible;
use std::sync::Arc;
use warp::http::StatusCode;
use warp::Reply;

pub async fn get_random(api: Arc<engine::Api>) -> Result<impl Reply, Infallible> {
    info!("Handling: get_random");

    match api.get_random_card().await {
        Some(card) => {
            Ok(util::reply_with_value(&card, StatusCode::OK))
        }
        None => {
            Ok(util::reply_with_error(&"No cards in compendium", StatusCode::NO_CONTENT))
        }
    }
}

pub async fn put_card(
    id: uuid::Uuid,
    api: Arc<engine::Api>,
    card: models::Card,
) -> Result<impl Reply, Infallible> {
    info!("Handling: put_card");

    // Validate explicit ID paramter matches ID in card
    if id != card.id {
        return Ok(util::reply_with_error(&"id mismatch", StatusCode::BAD_REQUEST));
    }

    match api.add_or_update_card_in_compendium(card).await {
        Ok(engine::api::AddOrUpdateOperation::Add) => {
            Ok(util::reply_empty(StatusCode::CREATED))
        }
        Ok(engine::api::AddOrUpdateOperation::Update) => {
            Ok(util::reply_empty(StatusCode::OK))
        }
        Err(e) => {
            Ok(util::reply_with_error(&e, StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}