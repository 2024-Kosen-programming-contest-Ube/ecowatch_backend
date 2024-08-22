use anyhow::Result;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::{Method, Request, Response, StatusCode};

use crate::utils;

mod classroom;

use classroom::{handler_create, handler_get_now_status, handler_login};

pub async fn route(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/classroom/create") => handler_create(req).await,
        (&Method::POST, "/classroom/login") => handler_login(req).await,
        (&Method::GET, "/classroom/get_now_status") => handler_get_now_status(req).await,

        // Return the 404 Not Found for other routes.
        _ => utils::response_empty(StatusCode::NOT_FOUND),
    }
}
