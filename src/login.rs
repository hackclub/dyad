use actix_web::HttpRequest;
use actix_web::web::Query;
use hc_helper::{
    auth::login::{CallbackArgs, Scopes},
    hackatime::{self},
};

#[actix_web::get("/login")]
pub async fn login_handler(req: HttpRequest) -> actix_web::HttpResponse {
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
) -> actix_web::HttpResponse {
    let redirect_url = {
        let conn = req.connection_info();
        format!("{}://{}/callback", conn.scheme(), conn.host())
    };
    hc_helper::auth::login::handle_callback(
        &req,
        query,
        redirect_url,
        "/dashboard".to_string(),
    )
    .await
}

#[actix_web::get("/hackatime-login")]
pub async fn hackatime_login_handler(req: HttpRequest) -> actix_web::HttpResponse {
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
        false
    )
    .await
}

#[actix_web::get("/hackatime-callback")]
pub async fn hackatime_callback_handler(
    req: HttpRequest,
    query: Query<hackatime::login::CallbackArgs>,
) -> actix_web::HttpResponse {
    let redirect_url = {
        let conn = req.connection_info();
        format!("{}://{}/hackatime-callback", conn.scheme(), conn.host())
    };
    hc_helper::hackatime::login::handle_callback(
        &req,
        query,
        redirect_url,
        "/dashboard".to_string(),
    )
    .await
}

#[actix_web::get("/api/hackatime-linked")]
pub async fn hackatime_linked_handler(req: HttpRequest) -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok()
        .json(req.cookie(hackatime::login::TOKEN_COOKIE).is_some())
}
