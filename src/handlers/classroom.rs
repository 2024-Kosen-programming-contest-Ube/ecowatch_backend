use anyhow::Result;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::{Request, Response, StatusCode};
use serde::Deserialize;
use ulid::Ulid;

use crate::{database, utils};

#[derive(Deserialize)]
struct ClassroomData {
    school_id: String,
    grade: i64,
    name: String,
    password: String,
}

pub async fn handler_create(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    let classroom_data = {
        let result = utils::parse_req_json::<ClassroomData>(req).await;
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

    let hash = utils::compute_password_hash(classroom_data.password);
    let id = Ulid::new().to_string();

    let pool = &database::get_pool().await;

    let result = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM classroom WHERE school_id=$1",
        classroom_data.school_id
    )
    .fetch_one(pool)
    .await;

    match result {
        Ok(count) => {
            if count <= 0 {
                return utils::response_error_message(
                    StatusCode::BAD_REQUEST,
                    "Invalid school_id".to_string(),
                );
            }
        }
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let result = sqlx::query!(
        "INSERT INTO classroom VALUES($1, $2, $3, $4, $5)",
        id,
        classroom_data.school_id,
        classroom_data.grade,
        classroom_data.name,
        hash
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        if let Some(dbe) = e.as_database_error() {
            println!("{}", dbe.message());
            return utils::response_error_message(
                StatusCode::BAD_REQUEST,
                "This classroom is already exist".to_string(),
            );
        }
        println!("{}", e.to_string());
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    utils::response_empty(StatusCode::OK)
}
