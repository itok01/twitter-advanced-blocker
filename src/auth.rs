use actix_web::http;
use actix_web::{cookie::Cookie, web, HttpMessage, HttpRequest, HttpResponse};
use bson::{bson, doc};
use serde::{Deserialize, Serialize};

use super::common::*;
use super::config::*;
use super::database::*;

// リクエストトークン
#[derive(Serialize, Deserialize)]
pub struct Token {
    pub key: String,
    pub secret: String,
}

// ユーザートークン
#[derive(Serialize, Deserialize)]
pub struct UserToken {
    pub id: String,
    pub token: Token,
}

// コールバックで受け取る情報
#[derive(Deserialize)]
pub struct CallbackQuery {
    oauth_token: String,
    oauth_verifier: String,
}

// Twitterの認証リンクを生成してリダイレクト
pub async fn get_auth_factory() -> HttpResponse {
    // 設定を読み込む
    let config = load_config();
    let con_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);

    // リクエストトークンを生成
    let request_token = egg_mode::request_token(&con_token, config.callback)
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
pub async fn get_callback_factory(web::Query(query): web::Query<CallbackQuery>) -> HttpResponse {
    // 設定を読み込む
    let config = load_config();
    let con_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);

    // リクエストトークンのコレクションにアクセス
    let database = connect_database();
    let request_token_collection = database.collection("request_token");

    // リクエストトークンを取り出す
    let filter = doc! {"key": query.oauth_token};
    match request_token_collection.find_one(filter.clone(), None) {
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
                        // リクエストトークンを削除
                        request_token_collection.delete_many(filter, None).ok();

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
                                    bson::Bson::Document(user_token_doc) => {
                                        // ユーザートークンのコレクションにアクセス
                                        let user_token_collection =
                                            database.collection("user_token");

                                        // 既存のユーザートークンを削除
                                        let filter = doc! {"id": &user_token.id};
                                        user_token_collection.delete_many(filter, None).ok();

                                        // データベースにユーザートークンを保存
                                        match user_token_collection.insert_one(user_token_doc, None)
                                        {
                                            Ok(_) => {
                                                // 認証リンクにリダイレクト
                                                // クッキーにユーザートークンを保存
                                                HttpResponse::TemporaryRedirect()
                                                    .header(http::header::LOCATION, endpoint("/"))
                                                    .cookie(
                                                        Cookie::build(
                                                            "oauth_token",
                                                            user_token.token.key,
                                                        )
                                                        .domain(config.domain)
                                                        .path("/")
                                                        .secure(config.secure)
                                                        .http_only(true)
                                                        .finish(),
                                                    )
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

// リクエストのクッキーからユーザートークンを取り出す
pub fn get_user_token_from_request(req: HttpRequest) -> Result<String, &'static str> {
    for cookie in req.cookies().unwrap().to_owned() {
        if cookie.name() == "oauth_token" {
            return Ok(cookie.value().to_string());
        }
    }
    Err("User token is not found.")
}

// ユーザートークンに合うトークンをデータベースから取り出す
pub async fn get_token(user_token: String) -> Result<egg_mode::Token, &'static str> {
    // ユーザートークンのコレクションにアクセス
    let database = connect_database();
    let user_token_collection = database.collection("user_token");

    let filter = doc! {"token.key": user_token};
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

// ユーザートークンをアクセストークンに変換
pub fn user_token_to_access_token(user_token: egg_mode::KeyPair) -> egg_mode::Token {
    // 設定を読み込む
    let config = load_config();
    let con_token = egg_mode::KeyPair::new(config.consumer_key, config.consumer_secret);

    egg_mode::Token::Access {
        consumer: con_token,
        access: user_token,
    }
}
