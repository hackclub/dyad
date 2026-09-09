use actix_files::Files;
use actix_web::{App, HttpServer, middleware::from_fn, web};
use sqlx::{Pool, Postgres};

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
mod logging;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    logging::init();

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
            .wrap(from_fn(logging::attach_conn_ctx))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
