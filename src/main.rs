mod utils;
mod state;
mod templates;
mod auth;
mod handlers;

use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use crate::handlers::{dashboard_handler, service_handler, login_handler, logout_handler, shutdown_handler, reboot_handler, download_handler, stop_download_handler, get_download_logs};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(dashboard_handler))
        .route("/service/{name}/{action}", post(service_handler))
        .route("/login", post(login_handler))
        .route("/logout", post(logout_handler))
        .route("/shutdown", post(shutdown_handler))
        .route("/reboot", post(reboot_handler))
        .route("/download", post(download_handler))
        .route("/download/stop", post(stop_download_handler))
        .route("/download/logs", get(get_download_logs))
        // Serves files from the "static" directory at the "/static" URL path
        .nest_service("/static", ServeDir::new("static"));

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}