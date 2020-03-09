use actix_web::http::header;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer};
use serde::Deserialize;

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

    // 認証リンクにリダイレクト
    HttpResponse::TemporaryRedirect()
        .header(header::LOCATION, auth_url)
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
async fn callback(web::Query(query): web::Query<CallbackQuery>) -> HttpResponse {
    // 設定を読み込む
    let config = load_config();
    let con_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);

    // ホームにリダイレクト
    HttpResponse::TemporaryRedirect()
        .header(header::LOCATION, endpoint("/"))
        .finish()
}

// 設定の構成
#[derive(Deserialize)]
struct Config {
    consumer_key: String,
    consumer_secret: String,
    domain: String,
    database_name: String,
    database_username: String,
    database_password: String,
}

// config.jsonから設定を読み込む
fn load_config() -> Config {
    let config_json =
        std::fs::read_to_string("./config.json").expect("Something went wrong reading the file");

    let config: Config = serde_json::from_str(config_json.as_str()).unwrap();

    config
}

// ドメインとパスを合成してURLを生成
fn endpoint(path: &str) -> String {
    let config = load_config();
    format!("http://{}{}", config.domain, path)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(auth).service(callback))
        .bind("0.0.0.0:80")?
        .run()
        .await
}
