use anyhow::Result;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::{Method, Request, Response, StatusCode};

use crate::utils;

mod classroom;
mod school;

pub async fn route(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/classroom/create") => classroom::handler_create(req).await,
        (&Method::POST, "/classroom/login") => classroom::handler_login(req).await,
        (&Method::GET, "/classroom/get_now_status") => classroom::handler_get_now_status(req).await,
        (&Method::POST, "/school/create") => school::handler_create(req).await,

        // Return the 404 Not Found for other routes.
        _ => utils::response_empty(StatusCode::NOT_FOUND),
    }
}
