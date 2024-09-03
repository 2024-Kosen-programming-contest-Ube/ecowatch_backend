use anyhow::Result;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier};
use bytes::{Buf, Bytes};
use cookie::time::Duration;
use cookie::Cookie;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::header::COOKIE;
use hyper::{header, Request, Response, StatusCode};
use serde::Serialize;
use sqlx::{Pool, Sqlite};

pub const CLASS_TOKEN: &str = "class_token";

pub type HandlerResponse = Result<Response<BoxBody<Bytes, hyper::Error>>>;

pub fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

pub async fn parse_req_json<T: for<'de> serde::de::Deserialize<'de>>(
    req: Request<hyper::body::Incoming>,
) -> Result<T> {
    let body = req.collect().await?.aggregate();
    let data = serde_json::from_reader::<_, T>(body.reader())?;
    Ok(data)
}

pub fn compute_password_hash(password: String) -> String {
    let salt = SaltString::generate(&mut rand::thread_rng());
    Argon2::new(
        Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.as_bytes(), &salt)
    .unwrap()
    .to_string()
}

pub fn response_json(
    status: StatusCode,
    json: String,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    let response = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(full(json))?;
    Ok(response)
}

pub fn response_struct_json<T>(status: StatusCode, value: &T) -> HandlerResponse
where
    T: ?Sized + Serialize,
{
    let json = serde_json::to_string(value)?;
    response_json(status, json)
}

pub fn response_empty(status: StatusCode) -> HandlerResponse {
    let response = Response::builder().status(status).body(empty())?;
    Ok(response)
}

#[derive(Serialize)]
struct ResponseErrorMessage {
    error: String,
}

pub fn response_error_message(status: StatusCode, msg: String) -> HandlerResponse {
    let res = ResponseErrorMessage { error: msg };
    let json = serde_json::to_string(&res)?;
    response_json(status, json)
}

pub fn verify_password(password: String, hash: String) -> Result<bool> {
    let expected_password_hash = PasswordHash::new(hash.as_str())?;
    let result = Argon2::default().verify_password(password.as_bytes(), &expected_password_hash);
    Ok(result.is_ok())
}

pub fn create_cookie(key: String, value: String) -> String {
    let cookie = Cookie::build((key, value))
        .path("/")
        .secure(false)
        .http_only(true)
        .max_age(Duration::days(365))
        .build();
    cookie.encoded().to_string()
}

pub fn get_cookie(req: &Request<hyper::body::Incoming>, key: String) -> Option<String> {
    for cookie_header in req.headers().get_all(COOKIE).iter() {
        match cookie_header.to_str() {
            Ok(header_str) => match Cookie::parse(header_str) {
                Ok(cookie) => {
                    if cookie.name() == key {
                        return Some(cookie.value().to_string());
                    }
                }
                Err(e) => println!("{}", e.to_string()),
            },
            Err(e) => println!("{}", e.to_string()),
        }
    }

    return None;
}

pub async fn get_class_id_from_token(
    pool: &Pool<Sqlite>,
    req: &Request<hyper::body::Incoming>,
) -> Result<String, HandlerResponse> {
    let token = match get_cookie(req, CLASS_TOKEN.to_string()) {
        Some(token) => token,
        None => return Err(response_empty(StatusCode::UNAUTHORIZED)),
    };
    let result = sqlx::query_scalar!("SELECT class_id FROM class_token WHERE token=$1", token)
        .fetch_optional(pool)
        .await;
    let class_id = match result {
        Ok(v) => match v {
            Some(class_id) => class_id,
            None => {
                return Err(response_error_message(
                    StatusCode::UNAUTHORIZED,
                    "Invalid token".to_string(),
                ))
            }
        },
        Err(e) => {
            println!("{}", e.to_string());
            return Err(response_empty(StatusCode::INTERNAL_SERVER_ERROR));
        }
    };
    return Ok(class_id);
}
