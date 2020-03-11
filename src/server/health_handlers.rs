use std::convert::Infallible;
use warp::Reply;

pub async fn ping() -> Result<impl Reply, Infallible> {
    Ok(warp::reply::json(&"pong"))
}
