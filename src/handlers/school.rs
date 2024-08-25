use anyhow::Result;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::{Request, Response, StatusCode};
use serde::Deserialize;
use ulid::Ulid;

use crate::{database, utils};

#[derive(Deserialize)]
struct CreateRequest {
    name: String,
}

pub async fn handler_create(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    let req_data = {
        let result = utils::parse_req_json::<CreateRequest>(req).await;
        match result {
            Ok(r) => r,
            Err(e) => {
                println!("{}", e.to_string());
                return utils::response_error_message(
                    StatusCode::BAD_REQUEST,
                    "Invalid params".to_string(),
                );
            }
        }
    };

    let pool = &database::get_pool().await;

    let id = Ulid::new().to_string();

    let result = sqlx::query!("INSERT INTO school VALUES($1, $2)", id, req_data.name)
        .execute(pool)
        .await;

    if let Err(e) = result {
        println!("{}", e.to_string());
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    utils::response_empty(StatusCode::OK)
}
