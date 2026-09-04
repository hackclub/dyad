use actix_web::HttpRequest;
use hc_helper::auth;

#[actix_web::get("/dashboard")]
pub async fn serve_dashboard(req: HttpRequest) -> impl actix_web::Responder {
    if req.cookie(auth::login::TOKEN_COOKIE).is_none() {
        return actix_web::HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish();
    }

    match std::fs::read_to_string("./ui/build/dashboard.html") {
        Ok(body) => actix_web::HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(body),
        Err(_) => actix_web::HttpResponse::NotFound().finish(),
    }
}
