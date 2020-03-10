use warp::{Filter, Rejection, Reply};

pub fn get_routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

    warp::get().and(hello)
}