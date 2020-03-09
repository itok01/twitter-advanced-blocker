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

    println!("{:?}", request_token);

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
        .header(header::LOCATION, endpoint("/"))
        .finish()
}

// 設定の構成
#[derive(Deserialize)]
struct Config {
    consumer_key: String,
    consumer_secret: String,
    domain: String,
    database_host: String,
    database_username: String,
    database_password: String,
    database_name: String,
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

// データベースに接続
fn connect_database() -> mongodb::Database {
    // 設定を読み込む
    let config = load_config();

    // データベースサーバーに接続
    let database_uri = format!(
        "mongodb://{}:{}@{}",
        config.database_username, config.database_password, config.database_host
    );
    let client = mongodb::Client::with_uri_str(database_uri.as_str()).unwrap();

    // データベースに接続
    let database = client.database(config.database_name.as_str());
    database
}

// データベースの初期化
fn database_init(database: mongodb::Database) {
    // 必要なコレクション
    let necessary_collection_names: Vec<&str> = vec!["user"];

    // 存在するコレクション
    let collection_names = database.list_collection_names(None).unwrap();

    // delete_meコレクションがあれば消す
    if collection_names.contains(&"delete_me".to_string()) {
        match database.collection("delete_me").drop(None) {
            Ok(_) => println!("Deleted \"delete_me\" collection."),
            Err(e) => println!("{}", e),
        }
    }

    // 足りないコレクションを作成
    for necessary_collection_name in necessary_collection_names {
        if !collection_names.contains(&necessary_collection_name.to_string()) {
            match database
                .create_collection("user", mongodb::options::CreateCollectionOptions::default())
            {
                Ok(_) => println!("Created \"{}\" collection.", necessary_collection_name),
                Err(e) => println!("{}", e),
            }
        }
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let database = connect_database();
    database_init(database);
    HttpServer::new(|| App::new().service(auth).service(callback))
        .bind("0.0.0.0:80")?
        .run()
        .await
}
