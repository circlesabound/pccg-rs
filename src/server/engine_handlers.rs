use crate::engine;

use std::convert::Infallible;
use std::sync::Arc;
use warp::http::StatusCode;
use warp::Reply;

pub async fn get_random(api: Arc<engine::Api>) -> Result<impl Reply, Infallible> {
    info!("Handling:get_random");
    let random_card = api.get_random_card().await;
    let json = warp::reply::json(&random_card);
    Ok(warp::reply::with_status(json, StatusCode::OK))
}
