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

pub fn response_struct_json<T>(
    status: StatusCode,
    value: &T,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>>
where
    T: ?Sized + Serialize,
{
    let json = serde_json::to_string(value)?;
    response_json(status, json)
}

pub fn response_empty(status: StatusCode) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    let response = Response::builder().status(status).body(empty())?;
    Ok(response)
}

#[derive(Serialize)]
struct ResponseErrorMessage {
    error: String,
}

pub fn response_error_message(
    status: StatusCode,
    msg: String,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
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
