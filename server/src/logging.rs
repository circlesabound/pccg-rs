use warp::filters::log::{Info, Log};

pub fn log_incoming_request() -> Log<impl Fn(Info) + Copy> {
    warp::log("pccg_rs::server::incoming_request")
}
