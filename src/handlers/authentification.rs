use axum::{
    extract::Form,
    http::{HeaderMap, header},
    response::{IntoResponse, Redirect},
};
use crate::auth::LoginRequest;

pub async fn login_handler(Form(payload): Form<LoginRequest>) -> impl IntoResponse {
    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| {
        eprintln!("ADMIN_PASSWORD n'est pas défini dans le fichier .env");
        String::new()
    });

    if !admin_password.is_empty() && payload.password == admin_password {
        let mut headers = HeaderMap::new();
        // On définit un cookie simple. Dans une vraie prod, utilisez des cookies signés/sécurisés.
        headers.insert(header::SET_COOKIE, "auth_session=true; Path=/; HttpOnly; SameSite=Lax".parse().unwrap());
        (headers, Redirect::to("/"))
    } else {
        (HeaderMap::new(), Redirect::to("/"))
    }
}

pub async fn logout_handler() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, "auth_session=; Path=/; Max-Age=0".parse().unwrap());
    (headers, Redirect::to("/"))
}
