use actix_session::Session;
use actix_web::http;
use actix_web::{get, web, HttpResponse};
use bson::{bson, doc};
use serde::{Deserialize, Serialize};

use super::common::*;
use super::config::*;
use super::database::*;

// リクエストトークン
#[derive(Serialize, Deserialize)]
struct Token {
    key: String,
    secret: String,
}

// ユーザートークン
#[derive(Serialize, Deserialize)]
struct UserToken {
    id: String,
    token: Token,
}

// コールバックで受け取る情報
#[derive(Deserialize)]
struct CallbackQuery {
    oauth_token: String,
    oauth_verifier: String,
}

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

    // リクエストトークンのコレクションにアクセス
    let database = connect_database();
    let request_token_collection = database.collection("request_token");

    // リクエストトークンをBsonに変換する
    let request_token = Token {
        key: request_token.key.to_string(),
        secret: request_token.secret.to_string(),
    };
    match bson::to_bson(&request_token).unwrap() {
        bson::Bson::Document(request_token_doc) => {
            // データベースにリクエストトークンを保存
            match request_token_collection.insert_one(request_token_doc, None) {
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
        _ => HttpResponse::InternalServerError().finish(),
    }
}

// コールバックの処理
#[get("/callback")]
async fn callback(web::Query(query): web::Query<CallbackQuery>, session: Session) -> HttpResponse {
    // 設定を読み込む
    let config = load_config();
    let con_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);

    // リクエストトークンのコレクションにアクセス
    let database = connect_database();
    let request_token_collection = database.collection("request_token");

    // リクエストトークンを取り出す
    let filter = doc! {"key": query.oauth_token};
    match request_token_collection.find_one(filter, None) {
        Ok(request_token_doc) => match request_token_doc {
            Some(request_token_doc) => {
                // BsonをTokenに変換
                let request_token: Token =
                    bson::from_bson(bson::Bson::Document(request_token_doc.clone())).unwrap();

                // キーペアの作成
                let request_token = egg_mode::KeyPair::new(request_token.key, request_token.secret);

                // トークンが有効か確かめる
                match egg_mode::access_token(con_token, &request_token, query.oauth_verifier).await
                {
                    Ok(access_token) => {
                        let token: egg_mode::Token = access_token.0;
                        // MongoDBがUnsignedに対応していないため、ユーザーIDをStringに変換
                        let user_id_u64: u64 = access_token.1;
                        let user_id = format!("{}", user_id_u64);

                        match token {
                            egg_mode::Token::Access { access, .. } => {
                                // ユーザートークンをBsonに変換する
                                let user_token = UserToken {
                                    id: user_id,
                                    token: Token {
                                        key: access.key.to_string(),
                                        secret: access.secret.to_string(),
                                    },
                                };
                                match bson::to_bson(&user_token).unwrap() {
                                    bson::Bson::Document(request_token_doc) => {
                                        // クッキーにユーザートークンを保存
                                        match session.set("oauth_token", &user_token.token.key) {
                                            Ok(_) => {
                                                // データベースにリクエストトークンを保存
                                                let user_token_collection =
                                                    database.collection("user_token");
                                                match user_token_collection
                                                    .insert_one(request_token_doc, None)
                                                {
                                                    Ok(_) => {
                                                        // 認証リンクにリダイレクト
                                                        HttpResponse::TemporaryRedirect()
                                                            .header(
                                                                http::header::LOCATION,
                                                                endpoint("/"),
                                                            )
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
                            _ => HttpResponse::InternalServerError().finish(),
                        }
                    }
                    Err(e) => {
                        println!("{:?}", e);
                        HttpResponse::InternalServerError().finish()
                    }
                }
            }
            None => {
                println!("Tmp user token is not found.");
                HttpResponse::InternalServerError().finish()
            }
        },
        Err(e) => {
            println!("{:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

// セッションの情報に合うトークンをデータベースから取り出す
async fn get_token(session: Session) -> Result<egg_mode::Token, &'static str> {
    let oauth_token = match session.get::<String>("oauth_token") {
        Ok(tmp_user_token) => match tmp_user_token {
            Some(tmp_user_token) => tmp_user_token,
            None => String::new(),
        },
        Err(e) => {
            println!("{:?}", e);
            String::new()
        }
    };

    println!("{}", oauth_token);

    // ユーザートークンのコレクションにアクセス
    let database = connect_database();
    let user_token_collection = database.collection("user_token");

    let filter = doc! {"token.key": oauth_token};
    match user_token_collection.find_one(filter, None) {
        Ok(user_token_doc) => match user_token_doc {
            Some(user_token_doc) => {
                // 設定を読み込む
                let config = load_config();
                let con_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);

                // BsonをTokenに変換
                let user_token: UserToken =
                    bson::from_bson(bson::Bson::Document(user_token_doc)).unwrap();

                // キーペアの作成
                let access_token =
                    egg_mode::KeyPair::new(user_token.token.key, user_token.token.secret);

                Ok(egg_mode::Token::Access {
                    consumer: con_token,
                    access: access_token,
                })
            }
            None => Err("Token is not found."),
        },
        Err(_) => Err("Token is not found."),
    }
}
