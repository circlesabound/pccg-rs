use super::super::models;

pub async fn get_random(db: models::Db) -> Result<impl warp::Reply, std::convert::Infallible> {
    Ok(warp::reply::json((db.lock().await).get_random()))
}