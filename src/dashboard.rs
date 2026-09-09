use actix_web::{HttpRequest, HttpResponse, cookie::Cookie, web};
use hc_helper::{auth, hackatime};
use sqlx::{Pool, Postgres};
use log::warn;

use crate::db::requests::get_hackatime_token_from_db;

#[actix_web::get("/dashboard")]
pub async fn serve_dashboard(
    req: HttpRequest,
    pool_data: web::Data<Pool<Postgres>>,
) -> impl actix_web::Responder {
    if req.cookie(auth::login::TOKEN_COOKIE).is_none() {
        warn!("Auth token not found, redirecting");
        return actix_web::HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    let auth_data = auth::data::get_auth_data(&req).await.unwrap();

    let present_in_db = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM hacker WHERE hca_id=$1)", auth_data.id).fetch_one(&**pool_data).await.unwrap();
    if !present_in_db.unwrap_or(false) {
        warn!("Entry in database missing, redirecting");
        return actix_web::HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    let hackatime_token_option = get_hackatime_token_from_db(auth_data.id, &pool_data).await;

    let should_return_remove_cookie = (|| async move {
        if hackatime_token_option.is_err() {
            log::error!("{:?}", hackatime_token_option);
            return false;
        }
        let hackatime_token_ok = hackatime_token_option.unwrap();
        let hackatime_token = hackatime_token_ok.unwrap_or(String::new());
        let cookie_maybe = req.cookie("hackatime-token");
        if cookie_maybe.is_none() {
            return false;
        }
        if hackatime_token != cookie_maybe.unwrap().value().to_string() {
            log::warn!("Stored Hackatime token does not match cookie");
            return true;
        }
        false
    })()
    .await;

    let mut response = match std::fs::read_to_string("./ui/build/dashboard.html") {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(body),
        Err(_) => HttpResponse::NotFound().finish(),
    };

    if should_return_remove_cookie {
        response
            .add_removal_cookie(
                &Cookie::build(hackatime::login::TOKEN_COOKIE, "")
                    .path("/")
                    .finish(),
            )
            .unwrap();
    }

    response
}
