use crate::engine;
use crate::models;

use std::convert::Infallible;
use std::sync::Arc;
use warp::http::StatusCode;
use warp::Reply;

pub async fn get_random(api: Arc<engine::Api>) -> Result<impl Reply, Infallible> {
    info!("Handling:get_random");
    match api.get_random_card().await {
        Some(card) => {
            let json = warp::reply::json(&card);
            Ok(warp::reply::with_status(json, StatusCode::OK))
        }
        None => {
            let json = warp::reply::json(&"No cards in compendium");
            Ok(warp::reply::with_status(json, StatusCode::NO_CONTENT))
        }
    }
}

pub async fn add_card(api: Arc<engine::Api>, card: models::Card) -> Result<impl Reply, Infallible> {
    info!("Handling:add_card");
    info!("Card to add: {:?}", card);
    match api.add_card_to_compendium(card).await {
        Ok(c) => {
            let json = warp::reply::json(&c);
            return Ok(warp::reply::with_status(json, StatusCode::CREATED));
        }
        Err(e) => {
            let json = warp::reply::json(&format!("Error: {:?}", e));
            return Ok(warp::reply::with_status(
                json,
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    }
}
