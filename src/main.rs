use actix_session::CookieSession;
use actix_web::{App, HttpServer};

use twitter_advanced_blocker::auth::*;
use twitter_advanced_blocker::block_list::*;
use twitter_advanced_blocker::database::*;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let database = connect_database();
    database_init(database);
    HttpServer::new(|| {
        App::new()
            .wrap(CookieSession::signed(&[0; 32]))
            .service(get_auth_factory)
            .service(get_callback_factory)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
