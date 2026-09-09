use actix_files::Files;
use actix_web::{App, HttpServer, web};

use crate::{
    dashboard::serve_dashboard,
    login::{
        callback_handler, hackatime_callback_handler, hackatime_linked_handler,
        hackatime_login_handler, login_handler,
    },
};

mod dashboard;
mod login;
mod db;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = db::init::init_db()
        .await
        .map_err(std::io::Error::other)?;
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(login_handler)
            .service(callback_handler)
            .service(hackatime_login_handler)
            .service(hackatime_callback_handler)
            .service(hackatime_linked_handler)
            .service(serve_dashboard)
            .service(Files::new("/", "./ui/build").index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
