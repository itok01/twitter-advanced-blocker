use actix_web::http;
use actix_web::{get, web, HttpRequest, HttpResponse};
use serde::Deserialize;

use super::config::*;

// Twitterの認証リンクを生成してリダイレクト
#[get("/auth")]
async fn auth() -> HttpResponse {
    // 設定を読み込む
    let config = load_config();
    let con_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);

    // リクエストトークンを生成
    let request_token = egg_mode::request_token(&con_token, endpoint("/callback"))
        .await
        .unwrap();

    // 認証リンクを生成
    let auth_url = egg_mode::authorize_url(&request_token);

    println!("{:?}", request_token);

    // 認証リンクにリダイレクト
    HttpResponse::TemporaryRedirect()
        .header(http::header::LOCATION, auth_url)
        .cookie(
            http::Cookie::build("token", "12345678")
                .domain(config.domain)
                .path("/")
                .http_only(true)
                .finish(),
        )
        .finish()
}

// コールバックで受け取る情報
#[derive(Deserialize)]
struct CallbackQuery {
    oauth_token: String,
    oauth_verifier: String,
}

// コールバックの処理
#[get("/callback")]
async fn callback(req: HttpRequest, web::Query(query): web::Query<CallbackQuery>) -> HttpResponse {
    // 設定を読み込む
    let config = load_config();
    let con_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);

    println!("{:?}", query.oauth_token);
    println!("{:?}", query.oauth_verifier);

    //let (token, user_id, screen_name) =
    //    egg_mode::access_token(con_token, query.oauth_token, query.oauth_verifier)
    //        .await
    //        .unwrap();
    //
    //println!("{}", token);
    //println!("{}", user_id);
    //println!("{}", screen_name);

    // ホームにリダイレクト
    HttpResponse::TemporaryRedirect()
        .header(http::header::LOCATION, endpoint("/"))
        .finish()
}
