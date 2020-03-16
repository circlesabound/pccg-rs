use super::engine_handlers;
use super::health_handlers;
use super::logging;
use crate::engine;

use std::convert::Infallible;
use std::sync::Arc;
use warp::{Filter, Rejection, Reply};

pub fn build_routes(
    api: Arc<engine::Api>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let ping = warp::path!("ping")
        .and(warp::get())
        .and_then(health_handlers::ping);
    let rnd = warp::path!("rnd")
        .and(warp::get())
        .and(with_engine_api(api))
        .and_then(engine_handlers::get_random);

    ping.or(rnd).with(logging::log_incoming_request())
}

fn with_engine_api(
    api: Arc<engine::Api>,
) -> impl Filter<Extract = (Arc<engine::Api>,), Error = Infallible> + Clone {
    warp::any().map(move || api.clone())
}
