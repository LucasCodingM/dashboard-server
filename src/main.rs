mod utils;

use askama::Template;
use std::sync::Mutex;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use sysinfo::{System};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use lazy_static::lazy_static;

use std::ffi::OsStr;

lazy_static! {
    /// This is an example for using doc comment attributes
    static ref SYS: Mutex<System> = Mutex::new(System::new_all());
}

#[derive(Template)]
#[template(path = "index.html")]
struct DashboardTemplate {
    cpu_usage: u32,
    total_memory: String,
    used_memory: String,
    memory_percentage: u32,
    bot_status: bool,
    samba_status: bool,
    minidlna_status: bool,
}

impl IntoResponse for DashboardTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template: {err}"),
            )
                .into_response(),
        }
    }
}



async fn dashboard_handler() -> impl IntoResponse {
    let mut sys = SYS.lock().unwrap();
    sys.refresh_all();
    
    let cpu_usage = sys.global_cpu_usage() as u32;
    
    let total_mem = utils::human_readable_bytes(sys.total_memory());
    let used_mem = utils::human_readable_bytes(sys.used_memory());
    
    let mem_pct = if sys.total_memory() > 0 {
        ((sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0) as u32
    } else {
        0
    };

    let bot_status = sys.processes_by_name(OsStr::new("declin_bot")).next().is_some();
    let samba_status = sys.processes_by_name(OsStr::new("smbd")).next().is_some();
    let minidlna_status = sys.processes_by_name(OsStr::new("minidlna")).next().is_some();

    DashboardTemplate {
        cpu_usage,
        total_memory: total_mem,
        used_memory: used_mem,
        memory_percentage: mem_pct,
        bot_status,
        samba_status,
        minidlna_status,
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(dashboard_handler))
        // Serves files from the "static" directory at the "/static" URL path
        .nest_service("/static", ServeDir::new("static"));

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}