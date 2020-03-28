use actix_web::{web, App, HttpServer};

use twitter_advanced_blocker::auth::*;
use twitter_advanced_blocker::database::*;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let database = connect_database();
    database_init(database);
    HttpServer::new(|| {
        App::new()
            .route("/api/auth", web::get().to(get_auth_factory))
            .route("/api/callback", web::get().to(get_callback_factory))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
