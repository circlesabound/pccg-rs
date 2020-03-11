use super::super::engine;

use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::Reply;

pub async fn get_random(api: Arc<Mutex<engine::Api>>) -> Result<impl Reply, Infallible> {
    info!("Handling:get_random");
    let api = api.lock().await;
    let random_card = api.get_random_card();
    let json = warp::reply::json(random_card);
    Ok(warp::reply::with_status(json, StatusCode::OK))
}
