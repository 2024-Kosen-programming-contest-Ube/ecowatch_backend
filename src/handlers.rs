use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::{Request, Response, StatusCode};
use std::convert::Infallible;
use std::net::SocketAddr;

use crate::utils;

pub async fn route(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Infallible> {
    let mut not_found = Response::new(utils::empty());
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    Ok(not_found)
}
