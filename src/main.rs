use actix_files::Files;
use actix_web::{App, HttpServer};

use crate::{
    dashboard::serve_dashboard,
    login::{callback_handler, hackatime_callback_handler, hackatime_login_handler, login_handler},
};

mod dashboard;
mod login;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(login_handler)
            .service(callback_handler)
            .service(hackatime_login_handler)
            .service(hackatime_callback_handler)
            .service(serve_dashboard)
            .service(Files::new("/", "./ui/build").index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
