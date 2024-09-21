use hyper::{
    header::{HeaderValue, SET_COOKIE},
    Request, StatusCode,
};
use serde::Deserialize;
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

    let token_cookie = utils::create_cookie("student_token".to_string(), token);

    let mut response = utils::response_empty(StatusCode::OK)?;
    response
        .headers_mut()
        .append(SET_COOKIE, HeaderValue::from_str(token_cookie.as_str())?);
    Ok(response)
}
