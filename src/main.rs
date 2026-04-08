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
use crate::{handlers::{authentification, download, system}, state::POWER_CONSUMPTION};


fn monitor_consumption() {
    // Thread de monitoring de la consommation électrique (RAPL)
    std::thread::spawn(|| {
        let path = "/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj";
        loop {
            match std::fs::read_to_string(path) {
                Ok(content1) => {
                    if let Ok(e1) = content1.trim().parse::<u64>() {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        if let Ok(content2) = std::fs::read_to_string(path) {
                            if let Ok(e2) = content2.trim().parse::<u64>() {
                                // Gestion du wrap-around du compteur (u64)
                                let diff = if e2 >= e1 { e2 - e1 } else { (u64::MAX - e1) + e2 };
                                let pkg_watts = diff as f32 / 1_000_000.0;
                                
                                // Formule: Ptotale = (Ppowerstat + Cfixes) * Kalim
                                // Cfixes = 20W (Disques + CM/RAM/Fans), Kalim = 1.12 (PSU 80+ Gold)
                                let total_watts = (pkg_watts + 20.0) * 1.12;
                                *POWER_CONSUMPTION.lock().unwrap() = total_watts;
                            }
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Erreur lecture RAPL: {}", e);
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        eprintln!("Tips: Run the program with sudo privileges. {}", path);
                    }
                    // Si le fichier n'existe pas (ex: pas de support RAPL), on attend avant de réessayer
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
            }
        }
    });
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    if std::env::var("IS_DOCKER").as_deref() != Ok("true") {
        monitor_consumption();
    }

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