use std::cell::Cell;

use actix_web::web::Query;
use actix_web::{
    HttpRequest, HttpResponse,
    cookie::Cookie,
    http::{
        StatusCode,
        header::{HeaderValue, LOCATION},
    },
    web,
};
use hc_helper::{
    auth::{
        self,
        login::{CallbackArgs, Scopes},
    },
    hackatime::{self},
};
use log::info;
use sqlx::{Pool, Postgres};

use crate::{
    cloudflare::origin,
    db::requests::{LinkOutcome, on_link_hackatime, on_login_push_or_update_hacker},
};

#[actix_web::get("/login")]
pub async fn login_handler(req: HttpRequest) -> actix_web::HttpResponse {
    info!("Handling new login request");
    let redirect_url = format!("{}/callback", origin(&req));
    hc_helper::auth::login::handle_login(
        &req,
        Scopes {
            openid: true,
            profile: true,
            email: true,
            name: true,
            slack_id: true,
            phone_number: false,
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
    let redirect_url = format!("{}/callback", origin(&req));
    let failed = Cell::new(false);
    let mut response = hc_helper::auth::login::handle_callback(
        &req,
        query,
        redirect_url,
        "/dashboard".to_string(),
        Some(|token| async {
            if let Err(e) = on_login_push_or_update_hacker(token, &pool_data).await {
                log::error!("Failed to update hacker: {:?}", e);
                failed.set(true);
            }
        }),
        true,
    )
    .await;
    if failed.get() {
        response.del_cookie(auth::login::TOKEN_COOKIE);
        let _ = response.add_removal_cookie(
            &Cookie::build(auth::login::TOKEN_COOKIE, "")
                .path("/")
                .finish(),
        );
    }
    if failed.get() || !response.status().is_redirection() {
        redirect(&mut response, "/?login_error=1");
    }
    response
}

fn redirect(response: &mut HttpResponse, location: &'static str) {
    *response.status_mut() = StatusCode::FOUND;
    response
        .headers_mut()
        .insert(LOCATION, HeaderValue::from_static(location));
}

#[actix_web::get("/hackatime-login")]
pub async fn hackatime_login_handler(req: HttpRequest) -> actix_web::HttpResponse {
    info!("Started new Hackatime login request");
    let redirect_url = format!("{}/hackatime-callback", origin(&req));
    hc_helper::hackatime::login::handle_login(
        &req,
        hackatime::login::Scopes {
            profile: true,
            read: true,
            admin: false,
        },
        redirect_url,
        true,
    )
    .await
}

#[actix_web::get("/hackatime-callback")]
pub async fn hackatime_callback_handler(
    req: HttpRequest,
    query: Query<hackatime::login::CallbackArgs>,
    pool_data: web::Data<Pool<Postgres>>,
) -> actix_web::HttpResponse {
    info!("Received respones for Hackatime OAuth");
    let redirect_url = format!("{}/hackatime-callback", origin(&req));
    let failed_redirect = Cell::new(None);
    let mut response = hc_helper::hackatime::login::handle_callback(
        &req,
        query,
        redirect_url,
        "/dashboard".to_string(),
        Some(|hackatime_token| async {
            match on_link_hackatime(hackatime_token, &req, &pool_data).await {
                Ok(LinkOutcome::Linked) => {}
                Ok(LinkOutcome::Taken) => {
                    failed_redirect.set(Some("/dashboard?hackatime_error=taken"))
                }
                Err(e) => {
                    log::error!("Failed to link Hackatime account: {:?}", e);
                    failed_redirect.set(Some("/dashboard?hackatime_error=failed"));
                }
            }
        }),
        false,
    )
    .await;
    if let Some(location) = failed_redirect.get() {
        redirect(&mut response, location);
    } else if !response.status().is_redirection() {
        redirect(&mut response, "/dashboard?hackatime_error=failed");
    }
    response
}

#[actix_web::post("/logout")]
pub async fn logout_handler(req: HttpRequest, _: web::Json<serde_json::Value>) -> HttpResponse {
    if let Some(token) = auth::login::get_auth_token_with_handling(&req) {
        auth::data::forget_auth_token(&token);
    }
    let mut response = HttpResponse::Ok().finish();
    for name in [auth::login::TOKEN_COOKIE, hackatime::login::TOKEN_COOKIE] {
        let _ = response.add_removal_cookie(&Cookie::build(name, "").path("/").finish());
    }
    response
}
