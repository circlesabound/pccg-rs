use super::super::models;
use super::handlers;

use warp::{Filter, Rejection, Reply};

pub fn get_routes(db: models::Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let ping = warp::path!("ping")
        .and(warp::get())
        .map(|| "Pong!");
    let rnd = warp::path!("rnd")
        .and(warp::get())
        .and(with_db(db))
        .and_then(handlers::get_random);

    ping.or(rnd)
}

fn with_db(db: models::Db) -> impl Filter<Extract = (models::Db,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}