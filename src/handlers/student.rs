use hyper::{
    header::{HeaderValue, SET_COOKIE},
    Request, StatusCode,
};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{database, utils};

#[derive(Deserialize)]
struct LoginRequest {
    class_id: String,
    student_id: String,
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

    // TODO: Verify class_id
    let token = Ulid::new().to_string();
    let result = sqlx::query!(
        "INSERT INTO student_token VALUES($1, $2, $3)",
        token,
        login_data.student_id,
        login_data.class_id
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        println!("{}", e.to_string());
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let token_cookie = utils::create_cookie(utils::STUDENT_TOKEN.to_string(), token);

    let mut response = utils::response_empty(StatusCode::OK)?;
    response
        .headers_mut()
        .append(SET_COOKIE, HeaderValue::from_str(token_cookie.as_str())?);
    Ok(response)
}

#[derive(Serialize)]
struct ExistChecklistResponse {
    exist: bool,
}

pub async fn handler_exist_checklist(
    req: Request<hyper::body::Incoming>,
) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let student_info = {
        let result = utils::get_student_info_from_token(pool, &req).await;
        match result {
            Ok(v) => v,
            Err(res) => return res,
        }
    };

    let result = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT * FROM checklist WHERE class_id=$1 AND student_id=$2 AND date=date('now', 'localtime'))",
        student_info.class_id, student_info.student_id
    )
    .fetch_one(pool)
    .await;

    let exist_checklist = match result {
        Ok(v) => v > 0,
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    utils::response_struct_json(
        StatusCode::OK,
        &ExistChecklistResponse {
            exist: exist_checklist,
        },
    )
}

pub async fn handler_checklist(req: Request<hyper::body::Incoming>) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let student_info = {
        let result = utils::get_student_info_from_token(pool, &req).await;
        match result {
            Ok(v) => v,
            Err(res) => return res,
        }
    };

    let checklist = {
        let result = utils::read_body_req(req).await;
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
        "INSERT INTO checklist VALUES($1, $2, $3, date('now', 'localtime'))",
        student_info.class_id,
        student_info.student_id,
        checklist
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        eprintln!("{}", e);
        return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
    }

    utils::response_empty(StatusCode::OK)
}

#[derive(Serialize)]
struct PointResponse {
    point: i64,
}

pub async fn handler_point(req: Request<hyper::body::Incoming>) -> utils::HandlerResponse {
    let pool = &database::get_pool().await;

    let student_info = {
        let result = utils::get_student_info_from_token(pool, &req).await;
        match result {
            Ok(v) => v,
            Err(res) => return res,
        }
    };

    let result = sqlx::query_scalar!(
        "SELECT point FROM day_status WHERE class_id=$1 AND date=date('now', 'localtime')",
        student_info.class_id
    )
    .fetch_optional(pool)
    .await;

    let point = match result {
        Ok(v) => match v {
            Some(_point) => _point,
            None => return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(e) => {
            println!("{}", e.to_string());
            return utils::response_empty(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    utils::response_struct_json(StatusCode::OK, &PointResponse { point })
}
