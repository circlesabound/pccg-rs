use super::engine_handlers;
use super::health_handlers;
use super::logging;
use crate::engine;
use crate::models;

use std::convert::Infallible;
use std::sync::Arc;
use warp::{Filter, Rejection, Reply};

pub fn build_routes(
    api: Arc<engine::Api>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let ping = warp::path!("ping")
        .and(warp::get())
        .and_then(health_handlers::ping);
    let version = warp::path!("version")
        .and(warp::get())
        .and_then(health_handlers::version);
    let rnd = warp::path!("rnd")
        .and(warp::get())
        .and(with_engine_api(api.clone()))
        .and_then(engine_handlers::get_random);
    let compendium_put = warp::path!("compendium")
        .and(warp::put())
        .and(with_engine_api(api.clone()))
        .and(with_card_from_body())
        .and_then(engine_handlers::add_card);

    ping.or(version)
        .or(rnd)
        .or(compendium_put)
        .with(logging::log_incoming_request())
}

fn with_engine_api(
    api: Arc<engine::Api>,
) -> impl Filter<Extract = (Arc<engine::Api>,), Error = Infallible> + Clone {
    warp::any().map(move || api.clone())
}

fn with_card_from_body() -> impl Filter<Extract = (models::Card,), Error = warp::Rejection> + Clone
{
    warp::body::json()
}
