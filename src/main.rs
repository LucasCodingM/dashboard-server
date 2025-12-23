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
use crate::handlers::{authentification, download, system};


#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let app = Router::new()
        .route("/", get(system::dashboard_handler))
        .route("/service/{name}/{action}", post(system::service_handler))
        .route("/login", post(authentification::login_handler))
        .route("/logout", post(authentification::logout_handler))
        .route("/shutdown", post(system::shutdown_handler))
        .route("/reboot", post(system::reboot_handler))
        .route("/download", post(download::download_handler))
        .route("/download/stop", post(download::stop_download_handler))
        .route("/download/logs", get(download::get_download_logs))
        // Serves files from the "static" directory at the "/static" URL path
        .nest_service("/static", ServeDir::new("static"));

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}