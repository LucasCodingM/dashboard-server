use axum::{
    extract::{Path, Form},
    http::{StatusCode, HeaderMap, header},
    response::{IntoResponse, Redirect, Html},
};
use serde::Deserialize;
use std::ffi::OsStr;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

use crate::utils;
use crate::state::{SYS, DOWNLOAD_STATE};
use crate::templates::DashboardTemplate;
use crate::auth::{check_auth, LoginRequest};

pub async fn dashboard_handler(headers: HeaderMap) -> impl IntoResponse {
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

    let is_authenticated = check_auth(&headers);

    DashboardTemplate {
        cpu_usage,
        total_memory: total_mem,
        used_memory: used_mem,
        memory_percentage: mem_pct,
        bot_status,
        samba_status,
        minidlna_status,
        is_authenticated,
    }
}

pub async fn service_handler(Path((service, action)): Path<(String, String)>, headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let service_name = match service.as_str() {
        "bot" => "declin_bot",
        "samba" => "smbd",
        "minidlna" => "minidlna",
        _ => return (StatusCode::BAD_REQUEST, "Unknown service").into_response(),
    };

    let cmd = match action.as_str() {
        "start" => "start",
        "stop" => "stop",
        _ => return (StatusCode::BAD_REQUEST, "Invalid action").into_response(),
    };

    match Command::new("sudo")
        .arg("systemctl")
        .arg(cmd)
        .arg(service_name)
        .status()
    {
        Ok(status) if status.success() => StatusCode::OK.into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Command failed").into_response(),
    }
}

pub async fn login_handler(Form(payload): Form<LoginRequest>) -> impl IntoResponse {
    // REMPLACEZ "admin" PAR VOTRE MOT DE PASSE SOUHAITÉ
    if payload.password == "admin" {
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

pub async fn shutdown_handler(headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    match Command::new("sudo")
        .arg("shutdown")
        .arg("-h")
        .arg("now")
        .spawn()
    {
        Ok(_) => "Server is shutting down...".into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to execute shutdown command").into_response(),
    }
}

pub async fn reboot_handler(headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    match Command::new("sudo")
        .arg("reboot")
        .spawn()
    {
        Ok(_) => "Server is rebooting...".into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to execute reboot command").into_response(),
    }
}

#[derive(Deserialize)]
pub struct DownloadRequest {
    url: String,
    category: String,
}

pub async fn download_handler(headers: HeaderMap, Form(payload): Form<DownloadRequest>) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

        // Vérifier si un téléchargement est déjà en cours
    {
        let state = DOWNLOAD_STATE.lock().unwrap();
        if state.is_running {
            return Html("<div style='color: red;'>Un téléchargement est déjà en cours.</div>").into_response();
        }
    }

    let root_path = "/stockage";
    let target_dir = match payload.category.as_str() {
        "film" => format!("{}/videos/films", root_path),
        "video" => format!("{}/videos", root_path),
        _ => format!("{}/telechargements", root_path),
    };

/*     let root_path = "/home/lucas";
    let target_dir = match payload.category.as_str() {
        "film" => format!("{}/Vidéos", root_path),
        "video" => format!("{}/Vidéos", root_path),
        _ => format!("{}/Téléchargements", root_path),
    }; */

    let is_youtube = payload.url.contains("youtube.com") || payload.url.contains("youtu.be");

        // Réinitialiser l'état
    {
        let mut state = DOWNLOAD_STATE.lock().unwrap();
        state.is_running = true;
        state.logs.clear();
        state.logs.push(format!("Démarrage du téléchargement : {}", payload.url));
        state.child_pid = None;
    }

    std::thread::spawn(move || {
        let mut cmd = if is_youtube {
            let mut c = Command::new("yt-dlp");
            c.arg("--newline");
            c.arg("-o");
            c.arg(format!("{}/%(title)s.%(ext)s", target_dir));
            c.arg(&payload.url);
            c
        } else {
            let mut c = Command::new("wget");
            c.arg("-P");
            c.arg(&target_dir);
            c.arg(&payload.url);
            c
        };

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                let pid = child.id();
                {
                    let mut state = DOWNLOAD_STATE.lock().unwrap();
                    state.child_pid = Some(pid);
                    state.logs.push(format!("Processus lancé (PID: {})", pid));
                }

                let stdout = child.stdout.take();
                let stderr = child.stderr.take();

                fn handle_stream<R: std::io::Read + Send + 'static>(stream: Option<R>) {
                    if let Some(s) = stream {
                        std::thread::spawn(move || {
                            let reader = BufReader::new(s);
                            for line in reader.lines() {
                                if let Ok(l) = line {
                                    let mut state = DOWNLOAD_STATE.lock().unwrap();
                                    state.logs.push(l);
                                    if state.logs.len() > 200 {
                                        state.logs.remove(0);
                                    }
                                }
                            }
                        });
                    }
                }

                handle_stream(stdout);
                handle_stream(stderr);

                let status = child.wait();
                
                {
                    let mut state = DOWNLOAD_STATE.lock().unwrap();
                    state.is_running = false;
                    state.child_pid = None;
                    match status {
                        Ok(s) => state.logs.push(format!("Terminé avec le code : {}", s)),
                        Err(e) => state.logs.push(format!("Erreur lors de l'attente du processus : {}", e)),
                    }
                }
            }
            Err(e) => {
                let mut state = DOWNLOAD_STATE.lock().unwrap();
                state.is_running = false;
                state.logs.push(format!("Erreur au lancement : {}", e));
            }
        }
    });

     Html(r#"
        <div id="download-status" hx-get="/download/logs" hx-trigger="load, every 1s" hx-target="this" hx-swap="outerHTML">
            Initialisation...
        </div>
    "#).into_response()
}

pub async fn stop_download_handler(headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    
    let state = DOWNLOAD_STATE.lock().unwrap();
    if let Some(pid) = state.child_pid {
        let _ = Command::new("kill").arg(pid.to_string()).output();
    }
    
    "Arrêt demandé...".into_response()
}

pub async fn get_download_logs() -> impl IntoResponse {
    let state = DOWNLOAD_STATE.lock().unwrap();
    let logs_html = state.logs.iter()
        .map(|l| l.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;"))
        .collect::<Vec<_>>()
        .join("<br>");
    
    let stop_button = if state.is_running {
        r#"<button hx-post="/download/stop" class="btn-shutdown" style="margin-top: 10px; background-color: #d9534f;">Arrêter le téléchargement</button>"#
    } else {
        ""
    };

    Html(format!(
        r#"<div id="download-status" hx-get="/download/logs" hx-trigger="every 1s" hx-target="this" hx-swap="outerHTML">
            <div style="background: #f0f0f0; padding: 10px; border-radius: 5px; max-height: 300px; overflow-y: auto; font-family: monospace; font-size: 0.9em; white-space: pre-wrap;">
                {}
            </div>
            {}
        </div>"#,
        if logs_html.is_empty() { "Aucun log..." } else { &logs_html }, 
        stop_button
    ))
}