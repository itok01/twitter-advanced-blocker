use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};

use super::auth::*;

#[derive(Deserialize)]
pub struct GetUserRequest {
    token: String,
    user: String,
}

#[derive(Serialize)]
pub struct GetUserResponse {
    ok: bool,
    id: Option<String>,
    name: Option<String>,
    icon: Option<String>,
}

// ユーザーの情報を取得
pub async fn get_user_handler(get_user_request: web::Query<GetUserRequest>) -> HttpResponse {
    // トークンを取得
    match get_token(get_user_request.token.clone()).await {
        Ok(token) => {
            let user = get_user_from_twitter(&token, get_user_request.user.clone()).await;

            println!("{:?}", user);

            HttpResponse::Ok().json(GetUserResponse {
                ok: true,
                id: Option::from(user.screen_name),
                name: Option::from(user.name),
                icon: Option::from(user.profile_image_url_https),
            })
        }
        Err(e) => {
            println!("{}", e);
            HttpResponse::InternalServerError().json(GetUserResponse {
                ok: false,
                id: None,
                name: None,
                icon: None,
            })
        }
    }
}

// ユーザーの情報を取得
pub async fn get_user_from_twitter(
    token: &egg_mode::Token,
    user: String,
) -> egg_mode::user::TwitterUser {
    egg_mode::user::show(user.parse::<u64>().unwrap(), &token)
        .await
        .unwrap()
        .response
}
