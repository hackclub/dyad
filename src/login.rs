use actix_web::{HttpRequest, web};
use actix_web::web::Query;
use hc_helper::{
    auth::login::{CallbackArgs, Scopes},
    hackatime::{self},
};
use sqlx::{Pool, Postgres};
use log::info;

use crate::db::requests::{on_link_hackatime, on_login_push_or_update_hacker};

#[actix_web::get("/login")]
pub async fn login_handler(req: HttpRequest) -> actix_web::HttpResponse {
    info!("Handling new login request");
    let redirect_url = {
        let conn = req.connection_info();
        format!("{}://{}/callback", conn.scheme(), conn.host())
    };
    hc_helper::auth::login::handle_login(
        &req,
        Scopes {
            openid: true,
            profile: true,
            email: true,
            name: true,
            slack_id: true,
            verification_status: true,
            basic_info: true,
            addresses: true,
        },
        redirect_url,
    )
    .await
}

#[actix_web::get("/callback")]
pub async fn callback_handler(
    req: HttpRequest,
    query: Query<CallbackArgs>,
    pool_data: web::Data<Pool<Postgres>>,
) -> actix_web::HttpResponse {
    info!("Received response from HCA OAuth");
    let redirect_url = {
        let conn = req.connection_info();
        format!("{}://{}/callback", conn.scheme(), conn.host())
    };
    hc_helper::auth::login::handle_callback(&req, query, redirect_url, "/dashboard".to_string(), Some(|token| async {
        if let Err(e) = on_login_push_or_update_hacker(token, &req, &pool_data).await {
            log::error!("Failed to update hacker: {:?}", e);
        }
    }))
        .await
}

#[actix_web::get("/hackatime-login")]
pub async fn hackatime_login_handler(req: HttpRequest) -> actix_web::HttpResponse {
    info!("Started new Hackatime login request");
    let redirect_url = {
        let conn = req.connection_info();
        format!("{}://{}/hackatime-callback", conn.scheme(), conn.host())
    };
    hc_helper::hackatime::login::handle_login(
        &req,
        hackatime::login::Scopes {
            profile: true,
            read: true,
            admin: false,
        },
        redirect_url,
        false,
    )
    .await
}

#[actix_web::get("/hackatime-callback")]
pub async fn hackatime_callback_handler(
    req: HttpRequest,
    query: Query<hackatime::login::CallbackArgs>,
    pool_data: web::Data<Pool<Postgres>>
) -> actix_web::HttpResponse {
    info!("Received respones for Hackatime OAuth");
    let redirect_url = {
        let conn = req.connection_info();
        format!("{}://{}/hackatime-callback", conn.scheme(), conn.host())
    };
    hc_helper::hackatime::login::handle_callback(
        &req,
        query,
        redirect_url,
        "/dashboard".to_string(),
        Some(|hackatime_token| async {
            if let Err(e) = on_link_hackatime(
                hackatime_token,
                &req,
                &pool_data,
            ).await {
                log::error!("Failed to link Hackatime account: {:?}", e);
            }
        }),
    )
    .await
}

#[actix_web::get("/api/hackatime-linked")]
pub async fn hackatime_linked_handler(req: HttpRequest) -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(req.cookie(hackatime::login::TOKEN_COOKIE).is_some())
}
