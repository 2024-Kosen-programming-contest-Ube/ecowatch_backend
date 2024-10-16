use chrono::Utc;
use hyper::{
    header::{HeaderName, HeaderValue, SET_COOKIE},
    Request, StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ulid::Ulid;

use crate::{
    database,
    utils::{self, calc_leftovers_point, DayStatus, Sensor},
};

#[derive(Deserialize)]
struct CreateRequest {
    school_id: String,
    grade: i64,
    name: String,
    password: String,
}

pub async fn handler_create(req: Request<hyper::body::Incoming>) -> utils::HandlerResponse {
    let create_data = {
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

    let hash = utils::compute_password_hash(create_data.password);
    let id = Ulid::new().to_string();

    let pool = &database::get_pool().await;

    let result = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM school WHERE id=$1)",
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
struct LoginRequest {
    class_id: String,
    password: String,
}

pub async fn handler_login(req: Request<hyper::body::Incoming>) -> utils::HandlerResponse {
    let login_data = {
        let result = utils::parse_req_json::<LoginRequest>(req).await;
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
        login_data.class_id
    )
    .fetch_optional(pool)
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
        login_data.class_id
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

pub async fn handler_logout(_: Request<hyper::body::Incoming>) -> utils::HandlerResponse {
    let mut response = utils::response_empty(StatusCode::OK)?;
    response.headers_mut().append(
        HeaderName::from_static("clear-site-data"),
        HeaderValue::from_str("\"cache\", \"cookies\"")?,
    );
    Ok(response)
}

pub async fn handler_get_now_status(req: Request<hyper::body::Incoming>) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let class_id = {
        let result = utils::get_class_id_from_token(pool, &req).await;
        match result {
            Ok(v) => v,
            Err(res) => return res,
        }
    };

    let result = sqlx::query_as!(
        DayStatus,
        "SELECT * FROM day_status WHERE class_id=$1 AND date=date('now', 'localtime')",
        class_id
    )
    .fetch_optional(pool)
    .await;

    let day_status = match result {
        Ok(v) => match v {
            Some(_day_status) => _day_status,
            None => return utils::response_json(StatusCode::OK, "{}".to_string()),
        },
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    utils::response_struct_json::<DayStatus>(StatusCode::OK, &day_status)
}

pub async fn handler_day_status_history(
    req: Request<hyper::body::Incoming>,
) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let class_id = {
        let result = utils::get_class_id_from_token(pool, &req).await;
        match result {
            Ok(v) => v,
            Err(res) => return res,
        }
    };

    let result = sqlx::query_as!(
        DayStatus,
        "SELECT * FROM day_status WHERE class_id=$1 AND date >= date('now', 'localtime', '-30 days')",
        class_id
    )
    .fetch_all(pool)
    .await;

    let day_status_list = match result {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    println!("c: {}", day_status_list.len());

    utils::response_json(StatusCode::OK, json!(day_status_list).to_string())
}

#[derive(Deserialize)]
struct RegistAttendanceRequest {
    attendees: i64,
}

pub async fn handler_regist_attendance(
    req: Request<hyper::body::Incoming>,
) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let class_id = {
        let result = utils::get_class_id_from_token(pool, &req).await;
        match result {
            Ok(v) => v,
            Err(res) => return res,
        }
    };

    let req_data = {
        let result = utils::parse_req_json::<RegistAttendanceRequest>(req).await;
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

    let result = sqlx::query_as!(
        DayStatus,
        "SELECT * FROM day_status WHERE class_id=$1 AND date=date('now', 'localtime')",
        class_id
    )
    .fetch_optional(pool)
    .await;

    let prev_status = match result {
        Ok(v) => match v {
            Some(_prev_status) => _prev_status,
            None => DayStatus {
                class_id: "".to_string(),
                point: 0,
                attend: None,
                leftovers: None,
                date: "".to_string(),
            },
        },
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let result = sqlx::query!(
        "INSERT INTO day_status values ($1, 0, $2, date('now', 'localtime'), NULL) ON CONFLICT(class_id, date) DO UPDATE SET attend = $2",
        class_id,
        req_data.attendees
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        eprintln!("{}", e);
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let result = sqlx::query_as!(
        DayStatus,
        "SELECT * FROM day_status WHERE class_id=$1 AND date=date('now', 'localtime')",
        class_id
    )
    .fetch_optional(pool)
    .await;

    let status = match result {
        Ok(v) => match v {
            Some(_prev_status) => _prev_status,
            None => return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let point = std::cmp::max(
        0,
        calc_leftovers_point(&prev_status, &status) + status.point,
    );

    let result = sqlx::query!(
        "UPDATE day_status SET point = $1 WHERE class_id=$2 AND date=date('now', 'localtime')",
        point,
        class_id
    )
    .execute(pool)
    .await;

    if result.is_err() {
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    utils::response_empty(StatusCode::OK)
}

#[derive(Deserialize)]
struct RegistLeftoversRequest {
    leftovers: i64,
}

pub async fn handler_regist_leftovers(
    req: Request<hyper::body::Incoming>,
) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let class_id = {
        let result = utils::get_class_id_from_token(pool, &req).await;
        match result {
            Ok(v) => v,
            Err(res) => return res,
        }
    };

    let req_data = {
        let result = utils::parse_req_json::<RegistLeftoversRequest>(req).await;
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

    let result = sqlx::query_as!(
        DayStatus,
        "SELECT * FROM day_status WHERE class_id=$1 AND date=date('now', 'localtime')",
        class_id
    )
    .fetch_optional(pool)
    .await;

    let prev_status = match result {
        Ok(v) => match v {
            Some(_prev_status) => _prev_status,
            None => DayStatus {
                class_id: "".to_string(),
                point: 0,
                attend: None,
                leftovers: None,
                date: "".to_string(),
            },
        },
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let result = sqlx::query!(
        "INSERT INTO day_status values ($1, 0, NULL, date('now', 'localtime'), $2) ON CONFLICT(class_id, date) DO UPDATE SET leftovers = $2",
        class_id,
        req_data.leftovers
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        eprintln!("{}", e);
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let result = sqlx::query_as!(
        DayStatus,
        "SELECT * FROM day_status WHERE class_id=$1 AND date=date('now', 'localtime')",
        class_id
    )
    .fetch_optional(pool)
    .await;

    let status = match result {
        Ok(v) => match v {
            Some(_prev_status) => _prev_status,
            None => return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let point = std::cmp::max(
        0,
        calc_leftovers_point(&prev_status, &status) + status.point,
    );

    let result = sqlx::query!(
        "UPDATE day_status SET point = $1 WHERE class_id=$2 AND date=date('now', 'localtime')",
        point,
        class_id
    )
    .execute(pool)
    .await;

    if result.is_err() {
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    utils::response_empty(StatusCode::OK)
}

#[derive(Serialize)]
struct Classroom {
    id: String,
    school_id: String,
    grade: i64,
    name: String,
}

#[derive(Serialize)]
struct School {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct AllClassrooms {
    schools: Vec<School>,
    classrooms: Vec<Classroom>,
}

pub async fn handler_get_all(_: Request<hyper::body::Incoming>) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let result = sqlx::query_as!(School, "SELECT * FROM school")
        .fetch_all(pool)
        .await;

    let schools = match result {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let result = sqlx::query_as!(
        Classroom,
        "SELECT id, school_id, grade, name FROM classroom"
    )
    .fetch_all(pool)
    .await;

    let classrooms = match result {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let all = AllClassrooms {
        schools,
        classrooms,
    };

    utils::response_struct_json(StatusCode::OK, &all)
}

#[derive(Serialize)]
struct SensorResponse {
    point: i64,
}

pub async fn handler_sensor(req: Request<hyper::body::Incoming>) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let class_id = {
        let result = utils::get_class_id_from_token(pool, &req).await;
        match result {
            Ok(v) => v,
            Err(res) => return res,
        }
    };

    let req_data = {
        let result = utils::parse_req_json::<Sensor>(req).await;
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

    let result = sqlx::query_scalar!(
        "SELECT time FROM latest_sensor_time WHERE class_id=$1",
        class_id
    )
    .fetch_optional(pool)
    .await;

    let time_diff_msec = match result {
        Ok(time_option) => match time_option {
            Some(time) => {
                println!("{}", time.as_str());
                let latest_result = utils::parse_str_time(time.as_str());
                match latest_result {
                    Ok(latest) => {
                        println!("{} : {}", Utc::now(), latest);
                        (Utc::now() - latest).num_milliseconds()
                    }
                    Err(e) => {
                        println!("e0: {}", e.to_string());
                        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
            None => 0,
        },
        Err(e) => {
            println!("e1: {}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Update latest time
    let result = sqlx::query!(
        "REPLACE INTO latest_sensor_time values($1, datetime('now', 'localtime'))",
        class_id
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        println!("e2: {}", e.to_string());
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Calc point
    let airconditionaer_point = utils::calc_airconditionaer_point(&req_data, time_diff_msec);
    println!("airp :{}", airconditionaer_point);

    let lux_point = utils::calc_lux_point(&req_data, time_diff_msec);
    println!("luxp :{}", lux_point);

    let result = sqlx::query_scalar!(
        "SELECT point FROM day_status WHERE class_id=$1 AND date=date('now', 'localtime')",
        class_id
    )
    .fetch_optional(pool)
    .await;

    let point_option = match result {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let point_diff = airconditionaer_point;

    let result_point = match point_option {
        // Some(point) => std::cmp::min(900, point + point_diff),
        Some(point) => std::cmp::max(0, point + point_diff),
        None => std::cmp::max(0, point_diff),
    };

    let result = sqlx::query!(
        "INSERT INTO day_status values ($1, $2, NULL, date('now', 'localtime'), NULL) ON CONFLICT(class_id, date) DO UPDATE SET point = $2",
        class_id,
        result_point
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        eprintln!("{}", e);
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    utils::response_struct_json::<SensorResponse>(
        StatusCode::OK,
        &SensorResponse {
            point: result_point,
        },
    )
}

struct ClassroomPoint {
    class_id: String,
    point: i64,
}

#[derive(Serialize)]
struct PointResponse {
    point: i64,
    rank: i64,
    class_num: i64,
}

pub async fn handler_point(req: Request<hyper::body::Incoming>) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let class_id = {
        let result = utils::get_class_id_from_token(pool, &req).await;
        match result {
            Ok(v) => v,
            Err(res) => return res,
        }
    };

    let result = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM classroom WHERE classroom.school_id = (SELECT school_id FROM classroom WHERE id=$1)"
    , class_id).fetch_one(pool).await;

    let class_num = match result {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let result = sqlx::query_as!(
        ClassroomPoint,
        "SELECT class_id, point FROM day_status
        JOIN classroom ON classroom.id = day_status.class_id
        WHERE date=date('now', 'localtime') AND classroom.school_id = (SELECT school_id FROM classroom WHERE id=$1)
        ORDER BY point DESC"
    , class_id).fetch_all(pool).await;

    let point_list = match result {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut rank = class_num;
    let mut point = 0;
    for class_point in point_list.iter().enumerate() {
        if class_point.1.class_id == class_id {
            point = class_point.1.point;
            rank = class_point.0 as i64 + 1;
        }
    }

    utils::response_struct_json::<PointResponse>(
        StatusCode::OK,
        &PointResponse {
            point,
            rank,
            class_num,
        },
    )
}

#[derive(Deserialize)]
struct SetPointRequest {
    point: i64,
}

pub async fn handler_setpoint(req: Request<hyper::body::Incoming>) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let class_id = {
        let result = utils::get_class_id_from_token(pool, &req).await;
        match result {
            Ok(v) => v,
            Err(res) => return res,
        }
    };

    let req_data = {
        let result = utils::parse_req_json::<SetPointRequest>(req).await;
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

    let result = sqlx::query!(
        "UPDATE day_status SET point=$1 WHERE class_id=$2 AND date=date('now', 'localtime')",
        req_data.point,
        class_id
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        println!("{}", e.to_string());
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    utils::response_empty(StatusCode::OK)
}
