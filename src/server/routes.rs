use super::engine_handlers;
use super::health_handlers;
use super::logging;
use crate::engine;

use std::convert::Infallible;
use std::sync::Arc;
use uuid::Uuid;
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

    let list_users_from_registry = warp::path!("users")
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::list_users_from_registry);

    let get_user_from_registry = warp::path!("users" / Uuid)
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::get_user_from_registry);

    let add_user_to_registry = warp::path!("users" / "new")
        .and(warp::post())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::add_user_to_registry);

    let add_card_to_user = warp::path!("users" / Uuid / "cards" / "add")
        .and(warp::post())
        .and(with_engine_api(Arc::clone(&api)))
        .and(with_json_from_body())
        .and_then(engine_handlers::add_card_to_user);

    let list_cards_from_compendium = warp::path!("compendium")
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::list_cards_from_compendium);

    let get_random_card_from_compendium = warp::path!("compendium" / "random")
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::get_random_card_from_compendium);

    let get_card_from_compendium = warp::path!("compendium" / Uuid)
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::get_card_from_compendium);

    let put_card_to_compendium = warp::path!("compendium" / Uuid)
        .and(warp::put())
        .and(with_engine_api(Arc::clone(&api)))
        .and(with_json_from_body())
        .and_then(engine_handlers::put_card_to_compendium);

    ping.or(version)
        .or(list_users_from_registry)
        .or(get_user_from_registry)
        .or(add_user_to_registry)
        .or(add_card_to_user)
        .or(list_cards_from_compendium)
        .or(get_random_card_from_compendium)
        .or(get_card_from_compendium)
        .or(put_card_to_compendium)
        .with(logging::log_incoming_request())
}

fn with_engine_api(
    api: Arc<engine::Api>,
) -> impl Filter<Extract = (Arc<engine::Api>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&api))
}

fn with_json_from_body<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
where
    T: Send + serde::de::DeserializeOwned,
{
    warp::body::json()
}
