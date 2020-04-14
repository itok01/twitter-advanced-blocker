use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};

use super::auth::*;
use super::database::*;

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

#[derive(Deserialize)]
pub struct GetAllUserRequest {
    token: String,
}

#[derive(Serialize)]
pub struct GetAllUserResponse {
    ok: bool,
    id: Option<std::vec::Vec<String>>,
}

// 登録済みユーザーの情報を取得
pub async fn get_all_user_handler(get_user_request: web::Query<GetAllUserRequest>) -> HttpResponse {
    // トークンを取得
    match get_token(get_user_request.token.clone()).await {
        Ok(_) => HttpResponse::Ok().json(GetAllUserResponse {
            ok: true,
            id: Option::from(get_all_user().await),
        }),
        Err(e) => {
            println!("{}", e);
            HttpResponse::InternalServerError().json(GetAllUserResponse {
                ok: false,
                id: None,
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

// 登録済みユーザーのIDを取得
pub async fn get_all_user() -> std::vec::Vec<String> {
    let mut user = std::vec::Vec::<String>::new();

    let database = connect_database();
    let user_token_collection = database.collection("user_token");

    for result in user_token_collection.find(None, None).unwrap() {
        match result {
            Ok(user_token_doc) => {
                let user_token: UserToken =
                    bson::from_bson(bson::Bson::Document(user_token_doc.clone())).unwrap();

                user.push(user_token.id);
            }
            Err(e) => println!("{}", e),
        }
    }

    user
}
