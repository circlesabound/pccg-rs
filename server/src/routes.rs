use super::engine_handlers;
use super::health_handlers;
use super::logging;
use crate::engine;

use http::StatusCode;
use std::convert::Infallible;
use std::sync::Arc;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

pub fn build_routes(
    api: Arc<engine::Api>,
) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    let ping = warp::path!("api" / "v0.1" / "ping")
        .and(warp::get())
        .and_then(health_handlers::ping);

    let version = warp::path!("api" / "v0.1" / "version")
        .and(warp::get())
        .and_then(health_handlers::version);

    let list_users_from_registry = warp::path!("api" / "v0.1" / "users")
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::list_users_from_registry);

    let get_user_from_registry = warp::path!("api" / "v0.1" / "users" / Uuid)
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::get_user_from_registry);

    let add_user_to_registry = warp::path!("api" / "v0.1" / "users" / "new")
        .and(warp::post())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::add_user_to_registry);

    let delete_user_from_registry = warp::path!("api" / "v0.1" / "users" / Uuid)
        .and(warp::delete())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::delete_user_from_registry);

    let list_characters_for_user = warp::path!("api" / "v0.1" / "users" / Uuid / "characters")
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::list_characters_for_user);

    let get_character_for_user = warp::path!("api" / "v0.1" / "users" / Uuid / "characters" / Uuid)
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::get_character_for_user);

    let claim_daily_for_user = warp::path!("api" / "v0.1" / "users" / Uuid / "daily")
        .and(warp::post())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::claim_daily_for_user);

    let list_cards_from_compendium = warp::path!("api" / "v0.1" / "compendium")
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::list_cards_from_compendium);

    let draw_card_to_stage_for_user = warp::path!("api" / "v0.1" / "users" / Uuid / "draw")
        .and(warp::post())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::draw_card_to_stage_for_user);

    let get_card_from_compendium = warp::path!("api" / "v0.1" / "compendium" / Uuid)
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::get_card_from_compendium);

    let put_card_to_compendium = warp::path!("api" / "v0.1" / "compendium" / Uuid)
        .and(warp::put())
        .and(with_engine_api(Arc::clone(&api)))
        .and(with_json_from_body())
        .and_then(engine_handlers::put_card_to_compendium);

    let get_staged_card = warp::path!("api" / "v0.1" / "users" / Uuid / "stage")
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::get_staged_card);

    let confirm_staged_card = warp::path!("api" / "v0.1" / "users" / Uuid / "stage")
        .and(warp::post())
        .and(with_engine_api(Arc::clone(&api)))
        .and(with_json_from_body())
        .and_then(engine_handlers::confirm_staged_card);

    let list_available_jobs = warp::path!("api" / "v0.1" / "jobs" / String)
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::list_available_jobs);

    let take_job = warp::path!("api" / "v0.1" / "users" / Uuid / "jobs" / "new")
        .and(warp::post())
        .and(with_engine_api(Arc::clone(&api)))
        .and(with_json_from_body())
        .and_then(engine_handlers::take_job);

    let list_jobs_for_user = warp::path!("api" / "v0.1" / "users" / Uuid / "jobs")
        .and(warp::get())
        .and(with_engine_api(Arc::clone(&api)))
        .and_then(engine_handlers::list_jobs_for_user);

    ping.or(version)
        .or(list_users_from_registry)
        .or(get_user_from_registry)
        .or(add_user_to_registry)
        .or(list_characters_for_user)
        .or(get_character_for_user)
        .or(claim_daily_for_user)
        .or(delete_user_from_registry)
        .or(draw_card_to_stage_for_user)
        .or(list_cards_from_compendium)
        .or(get_card_from_compendium)
        .or(put_card_to_compendium)
        .or(get_staged_card)
        .or(confirm_staged_card)
        .or(list_available_jobs)
        .or(take_job)
        .or(list_jobs_for_user)
        .recover(engine_handlers::handle_engine_error)
        .recover(handle_not_found)
        .recover(handle_method_not_allowed)
        .recover(unhandled)
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

async fn handle_method_not_allowed(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        Ok(warp::reply::with_status(
            warp::reply::reply(),
            StatusCode::METHOD_NOT_ALLOWED,
        ))
    } else {
        Err(err)
    }
}

async fn handle_not_found(err: Rejection) -> Result<impl Reply, Rejection> {
    if err.is_not_found() {
        Ok(warp::reply::with_status(
            warp::reply::reply(),
            StatusCode::NOT_FOUND,
        ))
    } else {
        Err(err)
    }
}

async fn unhandled(err: Rejection) -> Result<impl Reply, Infallible> {
    error!("Unhandled rejection {:?}", err);
    Ok(warp::reply::with_status(
        warp::reply::reply(),
        StatusCode::INTERNAL_SERVER_ERROR,
    ))
}
