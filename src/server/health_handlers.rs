use std::convert::Infallible;
use std::env;
use warp::Reply;

pub async fn ping() -> Result<impl Reply, Infallible> {
    Ok(warp::reply::json(&"pong"))
}

pub async fn version() -> Result<impl Reply, Infallible> {
    match env::var("GIT_COMMIT_HASH") {
        Ok(commit_hash) => {
            Ok(warp::reply::json(&Version { commit_hash }))
        },
        Err(_) => {
            Ok(warp::reply::json(&Version { commit_hash: "unversioned".into() }))
        }
    }
}

#[derive(serde::Serialize)]
struct Version {
    commit_hash: String
}
