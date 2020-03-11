use warp::Reply;
use std::convert::Infallible;

pub async fn ping() -> Result<impl Reply, Infallible> {
    Ok(warp::reply::json(&"pong"))
}