use std::fmt;
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

pub fn reply_with_error<T: fmt::Debug>(error: &T, status_code: StatusCode) -> WithStatus<Json> {
    // TODO fix double serialisation
    reply::with_status(
        reply::json(&ErrorResponse {
            error_message: format!("{:?}", error),
        }),
        status_code,
    )
}

#[derive(serde::Serialize)]
struct ErrorResponse {
    error_message: String,
}
