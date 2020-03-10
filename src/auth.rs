use actix_session::Session;
use actix_web::http;
use actix_web::{get, web, HttpResponse};
use bson::{bson, doc};
use serde::{Deserialize, Serialize};

use super::common::*;
use super::config::*;
use super::database::*;

// ユーザーの一時的なトークンとリクエストトークン
#[derive(Serialize, Deserialize)]
struct TmpUserToken {
    tmp_user_token: String,
    request_token_key: String,
    request_token_secret: String,
}

// Twitterの認証リンクを生成してリダイレクト
#[get("/auth")]
async fn auth(session: Session) -> HttpResponse {
    // 設定を読み込む
    let config = load_config();
    let con_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);

    // リクエストトークンを生成
    let request_token = egg_mode::request_token(&con_token, endpoint("/callback"))
        .await
        .unwrap();

    // 認証リンクを生成
    let auth_url = egg_mode::authorize_url(&request_token);

    // 一時的なユーザーのコレクションにアクセス
    let database = connect_database();
    let tmp_user_collection = database.collection("tmp_user_token");

    // トークンを生成し、リクエストトークンと結びつける
    let tmp_user_token = TmpUserToken {
        tmp_user_token: gen_random_string(32),
        request_token_key: request_token.key.to_string(),
        request_token_secret: request_token.secret.to_string(),
    };
    let serialized_tmp_user_token = bson::to_bson(&tmp_user_token).unwrap();

    // クッキーにトークンを保存
    match serialized_tmp_user_token {
        bson::Bson::Document(document) => {
            match session.set("tmp_user_token", &tmp_user_token.tmp_user_token) {
                Ok(_) => {
                    // データベースにトークンを保存
                    match tmp_user_collection.insert_one(document, None) {
                        Ok(_) => {
                            // 認証リンクにリダイレクト
                            HttpResponse::TemporaryRedirect()
                                .header(http::header::LOCATION, auth_url)
                                .finish()
                        }
                        Err(e) => {
                            println!("{:?}", e);
                            HttpResponse::InternalServerError().finish()
                        }
                    }
                }
                Err(e) => {
                    println!("{:?}", e);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        _ => HttpResponse::InternalServerError().finish(),
    }
}

// コールバックで受け取る情報
#[derive(Deserialize)]
struct CallbackQuery {
    oauth_token: String,
    oauth_verifier: String,
}

// コールバックの処理
#[get("/callback")]
async fn callback(web::Query(query): web::Query<CallbackQuery>, session: Session) -> HttpResponse {
    // 設定を読み込む
    let config = load_config();
    let con_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);

    let tmp_user_token = match session.get::<String>("tmp_user_token") {
        Ok(tmp_user_token) => match tmp_user_token {
            Some(tmp_user_token) => tmp_user_token,
            None => String::new(),
        },
        Err(e) => {
            println!("{:?}", e);
            String::new()
        }
    };

    // 一時的なユーザーのコレクションにアクセス
    let database = connect_database();
    let tmp_user_token_collection = database.collection("tmp_user_token");

    // 一時的なユーザーのコレクションからリクエストトークンを取り出す
    let filter = doc! {"tmp_user_token": &tmp_user_token};
    match tmp_user_token_collection.find_one(filter, None) {
        Ok(tmp_user_token_doc) => match tmp_user_token_doc {
            Some(tmp_user_token_doc) => {
                let doc: TmpUserToken =
                    bson::from_bson(bson::Bson::Document(tmp_user_token_doc)).unwrap();

                let request_token =
                    egg_mode::KeyPair::new(doc.request_token_key, doc.request_token_secret);

                let (token, user_id, screen_name) =
                    egg_mode::access_token(con_token, &request_token, query.oauth_verifier)
                        .await
                        .unwrap();

                println!("{:?}", token);
                println!("{:?}", user_id);
                println!("{:?}", screen_name);

                egg_mode::tweet::DraftTweet::new("Rustからツイート！こんにちは！")
                    .send(&token)
                    .await;
            }
            None => println!("Tmp user token is not found."),
        },
        Err(e) => println!("{:?}", e),
    }
    HttpResponse::TemporaryRedirect()
        .header(http::header::LOCATION, endpoint("/"))
        .finish()
}
