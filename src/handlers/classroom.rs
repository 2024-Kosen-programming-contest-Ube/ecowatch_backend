use hyper::{
    header::{HeaderValue, SET_COOKIE},
    Request, StatusCode,
};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{database, utils};

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

#[derive(Serialize)]
struct DayStatus {
    class_id: String,
    point: i64,
    attend: Option<i64>,
    date: String,
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

    let result = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT * FROM day_status WHERE class_id=$1 AND date=date('now', 'localtime'))",
        class_id
    )
    .fetch_one(pool)
    .await;

    let exist_status = match result {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if exist_status > 0 {
        let result = sqlx::query!(
            "UPDATE day_status SET attend = $1 WHERE class_id=$2 AND date=date('now', 'localtime')",
            req_data.attendees,
            class_id
        )
        .execute(pool)
        .await;

        if let Err(e) = result {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    } else {
        let result = sqlx::query!(
            "INSERT INTO day_status VALUES($1, 0, $2, date('now', 'localtime'))",
            class_id,
            req_data.attendees
        )
        .execute(pool)
        .await;

        if let Err(e) = result {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
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

#[derive(Deserialize)]
struct SensorRequest {
    temperature: f64,
    humidity: f64,
    #[serde(alias = "isPeople")]
    is_people: bool,
    lux: f64,
    useairconditionaer: bool,
    airconditionaertime: String,
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
        let result = utils::parse_req_json::<SensorRequest>(req).await;
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

    let point_diff = 10;
    let mut result_point = 0;

    match point_option {
        Some(point) => {
            result_point = point + point_diff;
            let result = sqlx::query!(
                "UPDATE day_status SET point = $1 WHERE class_id=$2 AND date=date('now', 'localtime')",
                result_point,
                class_id
            )
            .execute(pool)
            .await;

            if let Err(e) = result {
                println!("{}", e.to_string());
                return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
        None => {
            result_point = std::cmp::max(0, point_diff);
            let result = sqlx::query!(
                "INSERT INTO day_status VALUES($1, $2, NULL, date('now', 'localtime'))",
                class_id,
                result_point
            )
            .execute(pool)
            .await;

            if let Err(e) = result {
                println!("{}", e.to_string());
                return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    };

    utils::response_struct_json::<SensorResponse>(
        StatusCode::OK,
        &SensorResponse {
            point: result_point,
        },
    )
}
