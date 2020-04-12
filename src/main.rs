use actix_web::{web, App, HttpServer};

use twitter_advanced_blocker::auth::*;
use twitter_advanced_blocker::blocklist::*;
use twitter_advanced_blocker::database::*;
use twitter_advanced_blocker::user::*;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let database = connect_database();
    database_init(database);

    HttpServer::new(|| {
        App::new().service(
            web::scope("/api")
                .route("/auth", web::get().to(get_auth_handler))
                .route("/callback", web::get().to(get_callback_handler))
                .route("/signout", web::get().to(get_signout_handler))
                .route("/blocklist", web::get().to(get_blocklist_handler))
                .route("/blocklist", web::post().to(post_blocklist_handler))
                .route("/user", web::get().to(get_user_handler)),
        )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
