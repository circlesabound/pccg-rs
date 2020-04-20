use crate::engine;
use warp::http::StatusCode;
use warp::reply::{self, Json, WithStatus};

pub fn reply_empty(status_code: StatusCode) -> WithStatus<Json> {
    // TODO make this actually return an empty body
    reply::with_status(reply::json(&""), status_code)
}

pub fn reply_with_value<T: serde::Serialize>(
    value: &T,
    status_code: StatusCode,
) -> WithStatus<Json> {
    reply::with_status(reply::json(value), status_code)
}

pub fn reply_with_engine_error(error: &engine::Error, status_code: StatusCode) -> WithStatus<Json> {
    reply::with_status(reply::json(error), status_code)
}

pub fn reply_with_error_message(message: String, status_code: StatusCode) -> WithStatus<Json> {
    reply::with_status(
        reply::json(&ErrorResponse {
            error_message: message,
        }),
        status_code,
    )
}

#[derive(serde::Serialize)]
struct ErrorResponse {
    error_message: String,
}
