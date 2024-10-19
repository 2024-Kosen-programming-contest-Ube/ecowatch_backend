use std::io::Read;

use anyhow::Result;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier};
use bytes::{Buf, Bytes};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use cookie::{Cookie, SameSite};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::header::COOKIE;
use hyper::{header, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

use crate::config::CONFIG;

pub const CLASS_TOKEN: &str = "class_token";
pub const STUDENT_TOKEN: &str = "student_token";

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

pub async fn read_body_req(req: Request<hyper::body::Incoming>) -> Result<String> {
    let body = req.collect().await?.aggregate();
    let mut body_str = String::new();
    body.reader().read_to_string(&mut body_str);
    Ok(body_str)
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
    let samesite = if CONFIG.cookie_cross {
        SameSite::None
    } else {
        SameSite::Strict
    };
    let cookie = Cookie::build((key, value))
        .path("/")
        .secure(CONFIG.cookie_cross)
        .http_only(true)
        .max_age(cookie::time::Duration::days(365))
        .domain(CONFIG.cookie_domain.as_str())
        .same_site(samesite)
        .build();
    cookie.encoded().to_string()
}

pub fn get_cookie(req: &Request<hyper::body::Incoming>, key: String) -> Option<String> {
    for cookie_header in req.headers().get_all(COOKIE).iter() {
        match cookie_header.to_str() {
            Ok(header_str) => {
                for cookie_result in Cookie::split_parse(header_str) {
                    match cookie_result {
                        Ok(cookie) => {
                            if cookie.name() == key {
                                return Some(cookie.value().to_string());
                            }
                        }
                        Err(e) => println!("{}", e.to_string()),
                    }
                }
            }

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

pub struct StudentInfo {
    pub class_id: String,
    pub student_id: i64,
}

pub async fn get_student_info_from_token(
    pool: &Pool<Sqlite>,
    req: &Request<hyper::body::Incoming>,
) -> Result<StudentInfo, HandlerResponse> {
    let token = match get_cookie(req, STUDENT_TOKEN.to_string()) {
        Some(token) => token,
        None => return Err(response_empty(StatusCode::UNAUTHORIZED)),
    };
    let result = sqlx::query_as!(
        StudentInfo,
        "SELECT class_id, student_id FROM student_token WHERE token=$1",
        token
    )
    .fetch_optional(pool)
    .await;
    let info = match result {
        Ok(v) => match v {
            Some(info) => info,
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
    return Ok(info);
}

pub fn parse_str_time(str_time: &str) -> Result<DateTime<Utc>> {
    let latest_naive = NaiveDateTime::parse_from_str(str_time, "%Y-%m-%d %H:%M:%S")?;
    Ok(Utc.from_utc_datetime(&latest_naive) + chrono::Duration::hours(-9))
}

#[derive(Deserialize)]
pub struct Sensor {
    temperature: f64,
    humidity: f64,
    #[serde(alias = "isPeople")]
    is_people: bool,
    lux: f64,
    useairconditioner: bool,
    airconditioner_time: String,
}

pub fn calc_airconditionaer_point(sensor: &Sensor, duraton_msec: i64) -> i64 {
    let discomfort_index = 0.81 * sensor.temperature
        + 0.01 * sensor.humidity * (0.99 * sensor.temperature - 14.3)
        + 46.3;

    // Check satisfy air conditioner usage standards
    // let satisfy_airconditionaer = if sensor.is_people == false {
    //     false
    // } else if sensor.temperature < 18.0 || sensor.temperature > 28.0 {
    //     true
    // } else if discomfort_index < 60.0 || discomfort_index > 75.0 {
    //     true
    // } else {
    //     false
    // };

    // let should_add_point = if satisfy_airconditionaer && sensor.useairconditioner {
    //     true
    // } else if !satisfy_airconditionaer && !sensor.useairconditioner {
    //     true
    // } else {
    //     false
    // };

    // if !should_add_point {
    //     return 0.0;
    // }

    let co2p = 1.5 * 0.378;
    let n = i64::clamp(duraton_msec, 0, CONFIG.sensor_interval as i64) as f64 / (1000.0 * 60.0);
    println!(
        "read d {}",
        i64::clamp(duraton_msec, 0, CONFIG.sensor_interval as i64) as f64
    );
    println!("co2p:{} n:{} duration:{}", co2p, n, duraton_msec);
    let point = co2p * (10.0 - (discomfort_index - 69.5).abs()) * n;
    if point > 0.5 {
        point.ceil() as i64
    } else {
        return 0;
    }
}

pub fn calc_lux_point(sensor: &Sensor, duraton_msec: i64) -> i64 {
    let n = i64::clamp(duraton_msec, 0, CONFIG.sensor_interval as i64) as f64 / (1000.0 * 60.0);
    if sensor.is_people == false && sensor.lux < 30.0 {
        return (5.4 * 0.378 * 2.0 * n) as i64;
    } else {
        return 0;
    }
}

#[derive(Serialize)]
pub struct DayStatus {
    pub class_id: String,
    pub point: i64,
    pub attend: Option<i64>,
    pub leftovers: Option<i64>,
    pub date: String,
}

pub fn calc_leftovers_point(prev_daystatus: &DayStatus, daystatus: &DayStatus) -> i64 {
    let prev_point = if prev_daystatus.attend.is_none() || prev_daystatus.leftovers.is_none() {
        0
    } else {
        (((1030.0 * prev_daystatus.attend.unwrap() as f64) / 30.0
            - prev_daystatus.leftovers.unwrap() as f64)
            * 2.501
            * (1.0 / 10.0)) as i64
    };

    let current_point = (((1030.0 * daystatus.attend.unwrap() as f64) / 30.0
        - daystatus.leftovers.unwrap() as f64)
        * 2.501
        * (1.0 / 10.0)) as i64;

    println!("left c:{} prev:{}", current_point, prev_point);
    current_point - prev_point
}
