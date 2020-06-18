use actix_web::{web, HttpResponse};
use bson::{bson, doc};
use serde::{Deserialize, Serialize};

use super::auth::*;
use super::database::*;

// ブロックリスト
#[derive(Serialize, Deserialize)]
pub struct Blocklist {
    pub id: String,
    pub blocklist: std::vec::Vec<String>,
}

#[derive(Deserialize)]
pub struct GetBlocklistRequest {
    token: String,
    user: String,
}

#[derive(Deserialize)]
pub struct PostBlocklistRequest {
    token: String,
}

#[derive(Serialize)]
pub struct GetBlocklistResponse {
    ok: bool,
    id: Option<String>,
    blocklist: Option<std::vec::Vec<String>>,
}

#[derive(Serialize)]
pub struct PostBlocklistResponse {
    ok: bool,
}

// ユーザーのブロックリストを取得
pub async fn get_blocklist_handler(
    blocklist_request: web::Query<GetBlocklistRequest>,
) -> HttpResponse {
    // トークンを取得
    match get_token(blocklist_request.token.clone()).await {
        Ok(token) => {
            let database = connect_database();
            let blocklist_collection = database.collection("blocklist");

            let filter = if blocklist_request.user == "" {
                doc! {"id": get_user_id(&token).unwrap()}
            } else {
                doc! {"id": blocklist_request.user.clone()}
            };
            match blocklist_collection.find_one(filter, None) {
                Ok(blocklist_doc) => match blocklist_doc {
                    Some(blocklist_doc) => {
                        // BsonをTokenに変換
                        let blocklist: Blocklist =
                            bson::from_bson(bson::Bson::Document(blocklist_doc)).unwrap();

                        HttpResponse::Ok().json(GetBlocklistResponse {
                            ok: true,
                            id: Option::from(blocklist.id),
                            blocklist: Option::from(blocklist.blocklist),
                        })
                    }
                    None => HttpResponse::InternalServerError().json(GetBlocklistResponse {
                        ok: false,
                        id: None,
                        blocklist: None,
                    }),
                },
                Err(e) => {
                    println!("{}", e);
                    HttpResponse::InternalServerError().json(GetBlocklistResponse {
                        ok: false,
                        id: None,
                        blocklist: None,
                    })
                }
            }
        }
        Err(e) => {
            println!("{}", e);
            HttpResponse::InternalServerError().json(GetBlocklistResponse {
                ok: false,
                id: None,
                blocklist: None,
            })
        }
    }
}

// ユーザーのブロックリストを更新
pub async fn post_blocklist_handler(
    blocklist_request: web::Json<PostBlocklistRequest>,
) -> HttpResponse {
    // トークンを取得
    match get_token(blocklist_request.token.clone()).await {
        Ok(token) => {
            let blocklist = Blocklist {
                id: get_user_id(&token).unwrap(),
                blocklist: get_blocklist_from_twitter(&token).await,
            };

            let database = connect_database();
            let blocklist_collection = database.collection("blocklist");

            let filter = doc! {"id": &blocklist.id};
            blocklist_collection.delete_many(filter, None).ok();

            match bson::to_bson(&blocklist).unwrap() {
                bson::Bson::Document(blocklist_doc) => {
                    // データベースにブロックリストを保存
                    blocklist_collection.insert_one(blocklist_doc, None).ok();
                }
                _ => {}
            }

            HttpResponse::Ok().json(PostBlocklistResponse { ok: true })
        }
        Err(e) => {
            println!("{}", e);
            HttpResponse::InternalServerError().json(PostBlocklistResponse { ok: false })
        }
    }
}

#[derive(Deserialize)]
pub struct PostBlocklistImportRequest {
    token: String,
    user: String,
}

#[derive(Serialize)]
pub struct PostBlocklistImportResponse {
    ok: bool,
}

// ブロックリストをインポート
pub async fn post_blocklist_import_handler(
    post_blocklist_import_request: web::Json<PostBlocklistImportRequest>,
) -> HttpResponse {
    // トークンを取得
    match get_token(post_blocklist_import_request.token.clone()).await {
        Ok(token) => {
            let database = connect_database();
            let blocklist_collection = database.collection("blocklist");

            let filter = doc! {"id": post_blocklist_import_request.user.clone()};
            match blocklist_collection.find_one(filter, None) {
                Ok(blocklist_doc) => match blocklist_doc {
                    Some(blocklist_doc) => {
                        // BsonをTokenに変換
                        let blocklist: Blocklist =
                            bson::from_bson(bson::Bson::Document(blocklist_doc)).unwrap();

                        for target_user in blocklist.blocklist {
                            egg_mode::user::block(target_user.parse::<u64>().unwrap(), &token)
                                .await
                                .ok();
                        }

                        HttpResponse::Ok().json(PostBlocklistImportResponse { ok: true })
                    }
                    None => HttpResponse::InternalServerError()
                        .json(PostBlocklistImportResponse { ok: false }),
                },
                Err(e) => {
                    println!("{}", e);
                    HttpResponse::InternalServerError()
                        .json(PostBlocklistImportResponse { ok: false })
                }
            }
        }
        Err(e) => {
            println!("{}", e);
            HttpResponse::InternalServerError().json(PostBlocklistImportResponse { ok: false })
        }
    }
}

// ユーザーのブロックリストを取得
pub async fn get_blocklist_from_twitter(token: &egg_mode::Token) -> std::vec::Vec<String> {
    let blocks_ids = egg_mode::user::blocks_ids(&token);
    let blocklist: std::vec::Vec<String> = blocks_ids
        .call()
        .await
        .unwrap()
        .response
        .ids
        .iter()
        .map(|x| format!("{}", x))
        .collect();
    blocklist
}

// データベースにあるユーザーのブロックリストを更新
pub async fn update_database_blocklists() {
    println!("HELLO!");
    // ユーザートークンとブロックリストのコレクションにアクセス
    let database = connect_database();
    let user_token_collection = database.collection("user_token");
    let blocklist_collection = database.collection("blocklist");

    // トークンを取り出す
    for result in user_token_collection.find(None, None).unwrap() {
        match result {
            Ok(user_token_doc) => {
                let user_token: UserToken =
                    bson::from_bson(bson::Bson::Document(user_token_doc.clone())).unwrap();
                let token = user_token_to_access_token(egg_mode::KeyPair::new(
                    user_token.token.key,
                    user_token.token.secret,
                ));

                println!("{}", user_token.id);
                // 古いブロックリストを削除
                let filter = doc! {"id": &user_token.id};
                blocklist_collection.delete_many(filter, None).ok();

                // ブロックリストをBsonに変換
                let blocklist = Blocklist {
                    id: user_token.id,
                    blocklist: get_blocklist_from_twitter(&token).await,
                };
                match bson::to_bson(&blocklist).unwrap() {
                    bson::Bson::Document(blocklist_doc) => {
                        // データベースにブロックリストを保存
                        blocklist_collection.insert_one(blocklist_doc, None).ok();
                    }
                    _ => {}
                }
            }
            Err(e) => println!("{}", e),
        }
    }
}
