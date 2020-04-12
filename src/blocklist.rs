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
pub struct BlocklistRequest {
    token: String,
}

#[derive(Serialize)]
pub struct GetBlocklistResponse {
    ok: bool,
    blocklist: Option<Blocklist>,
}

#[derive(Serialize)]
pub struct PostBlocklistResponse {
    ok: bool,
}

// ユーザーのブロックリストを更新
pub async fn post_blocklist_factory(
    blocklist_request: web::Json<BlocklistRequest>,
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
