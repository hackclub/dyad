use actix_files::Files;
use actix_web::{App, HttpServer, middleware::from_fn, web};
use hc_helper::airtable::{AirtableTable, spawn_airtable_queue_handler};

use crate::{
    dashboard::{
        delete_project, get_games, get_invite, get_me, get_project_card_data, get_projects_list,
        invite_link, join_project, respond_invite, serve_dashboard, set_reminders, ship_project,
        upsert_project,
    },
    login::{
        callback_handler, hackatime_callback_handler, hackatime_login_handler, login_handler,
        logout_handler,
    },
};

mod airtable;
mod dashboard;
mod db;
mod error;
mod games;
mod hackatime;
mod logging;
mod login;
mod reminders;
mod review;
mod slack;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    logging::init();

    spawn_airtable_queue_handler().map_err(std::io::Error::other)?;
    std::sync::LazyLock::force(&games::CATALOG);
    let airtable_table = AirtableTable {
        token: std::env::var("AIRTABLE_TOKEN").unwrap_or_default(),
        base_id: std::env::var("AIRTABLE_BASE_ID").unwrap_or_default(),
        table_id_or_name: std::env::var("AIRTABLE_TABLE").unwrap_or_default(),
    };
    if airtable_table.token.is_empty()
        || airtable_table.base_id.is_empty()
        || airtable_table.table_id_or_name.is_empty()
    {
        log::warn!("Airtable env vars not set; skipping Airtable init");
    } else {
        airtable::init(&airtable_table)
            .await
            .map_err(std::io::Error::other)?;
    }

    let pool = db::init::init_db().await.map_err(std::io::Error::other)?;

    slack::init();
    tokio::spawn(reminders::poll(pool.clone()));
    if !airtable_table.token.is_empty() {
        tokio::spawn(review::poll(airtable_table.clone(), pool.clone()));
    }
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(airtable_table.clone()))
            .service(login_handler)
            .service(callback_handler)
            .service(hackatime_login_handler)
            .service(hackatime_callback_handler)
            .service(logout_handler)
            .service(serve_dashboard)
            .service(get_me)
            .service(get_games)
            .service(get_projects_list)
            .service(get_project_card_data)
            .service(upsert_project)
            .service(respond_invite)
            .service(ship_project)
            .service(delete_project)
            .service(get_invite)
            .service(join_project)
            .service(set_reminders)
            .service(invite_link)
            .service(Files::new("/", "./ui/build").index_file("index.html"))
            .wrap(from_fn(logging::attach_conn_ctx))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
