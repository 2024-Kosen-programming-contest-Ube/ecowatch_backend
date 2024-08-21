use anyhow::Result;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::{
    header::{HeaderValue, SET_COOKIE},
    Request, Response, StatusCode,
};
use serde::Deserialize;
use ulid::Ulid;

use crate::{database, utils};

#[derive(Deserialize)]
struct CreateData {
    school_id: String,
    grade: i64,
    name: String,
    password: String,
}

pub async fn handler_create(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    let create_data = {
        let result = utils::parse_req_json::<CreateData>(req).await;
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

    let hash = utils::compute_password_hash(create_data.password);
    let id = Ulid::new().to_string();

    let pool = &database::get_pool().await;

    let result = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM classroom WHERE school_id=$1",
        create_data.school_id
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
        create_data.school_id,
        create_data.grade,
        create_data.name,
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

#[derive(Deserialize)]
struct LoginData {
    id: String,
    password: String,
}

pub async fn handler_login(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    let login_data = {
        let result = utils::parse_req_json::<LoginData>(req).await;
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

    let result = sqlx::query_scalar!(
        "SELECT password_hash FROM classroom WHERE id=$1",
        login_data.id
    )
    .fetch_one(pool)
    .await;

    let hashed_password = match result {
        Ok(v) => match v {
            Some(hash) => hash,
            None => {
                return utils::response_error_message(
                    StatusCode::BAD_REQUEST,
                    "Specified school_id is not found.".to_string(),
                )
            }
        },
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Check password
    let result = utils::verify_password(login_data.password, hashed_password);
    match result {
        Ok(verified) => {
            if !verified {
                return utils::response_error_message(
                    StatusCode::BAD_REQUEST,
                    "Incorrect password".to_string(),
                );
            }
        }
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let token = Ulid::new().to_string();
    let result = sqlx::query!(
        "INSERT INTO class_token VALUES($1, $2)",
        token,
        login_data.id
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        println!("{}", e.to_string());
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let token_cookie = utils::create_cookie("class_token".to_string(), token);

    let mut response = utils::response_empty(StatusCode::OK)?;
    response
        .headers_mut()
        .append(SET_COOKIE, HeaderValue::from_str(token_cookie.as_str())?);
    Ok(response)
}
