use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use sysinfo::{System};
use tokio::net::TcpListener;

#[derive(Template)]
#[template(path = "index.html")]
struct DashboardTemplate {
    cpu_usage: u32,
    total_memory: u64,
    used_memory: u64,
    memory_percentage: u32,
}

// Manually implement IntoResponse for our template
// This replaces the old askama_axum functionality
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
    let mut sys = System::new_all();
    
    // Refresh system information to get latest stats
    sys.refresh_all();
    
    // Important: CPU usage needs a small delay or a second refresh to be calculated
    // For a real app, you'd probably maintain a global 'System' instance
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    sys.refresh_cpu_all();
    
    let cpu_usage = sys.global_cpu_usage() as u32;
    
    // Convert bytes to GB
    let bytes_to_gb = 1024 * 1024 * 1024;
    let total_mem = sys.total_memory() / bytes_to_gb;
    let used_mem = sys.used_memory() / bytes_to_gb;
    
    let mem_pct = if sys.total_memory() > 0 {
        ((sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0) as u32
    } else {
        0
    };

    DashboardTemplate {
        cpu_usage,
        total_memory: total_mem,
        used_memory: used_mem,
        memory_percentage: mem_pct,
    }
}

#[tokio::main]
async fn main() {
    // Build application with Axum
    let app = Router::new().route("/", get(dashboard_handler));

    // Server setup
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Dashboard live at http://localhost:3000");
    
    axum::serve(listener, app).await.unwrap();
}