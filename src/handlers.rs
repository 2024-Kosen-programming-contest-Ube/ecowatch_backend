use anyhow::Result;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::{Method, Request, Response, StatusCode};

use crate::utils;

mod classroom;
mod school;
mod student;

pub async fn route(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    if req.method() == &Method::OPTIONS {
        return utils::response_empty(StatusCode::OK);
    }

    match (req.method(), req.uri().path()) {
        (&Method::POST, "/classroom/create") => classroom::handler_create(req).await,
        (&Method::GET, "/classroom/get_all") => classroom::handler_get_all(req).await,
        (&Method::POST, "/classroom/login") => classroom::handler_login(req).await,
        (&Method::POST, "/classroom/logout") => classroom::handler_logout(req).await,
        (&Method::GET, "/classroom/get_now_status") => classroom::handler_get_now_status(req).await,
        (&Method::GET, "/classroom/get_status_history") => {
            classroom::handler_day_status_history(req).await
        }
        (&Method::GET, "/classroom/point") => classroom::handler_point(req).await,
        (&Method::POST, "/classroom/regist_attendance") => {
            classroom::handler_regist_attendance(req).await
        }
        (&Method::POST, "/classroom/regist_leftovers") => {
            classroom::handler_regist_leftovers(req).await
        }
        (&Method::POST, "/classroom/sensor") => classroom::handler_sensor(req).await,
        (&Method::POST, "/school/create") => school::handler_create(req).await,
        (&Method::POST, "/student/login") => student::handler_login(req).await,
        (&Method::GET, "/student/exist_checklist") => student::handler_exist_checklist(req).await,
        (&Method::POST, "/student/checklist") => student::handler_checklist(req).await,

        // Return the 404 Not Found for other routes.
        _ => utils::response_empty(StatusCode::NOT_FOUND),
    }
}
