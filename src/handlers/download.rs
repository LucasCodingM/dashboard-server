use axum::{
    extract::{Form},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Html},
};
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

use crate::state::{ DOWNLOAD_STATE};
use crate::auth::{check_auth,};

#[derive(Deserialize)]
pub struct DownloadRequest {
    url: String,
    category: String,
}

pub async fn download_handler(headers: HeaderMap, Form(payload): Form<DownloadRequest>) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    {
        let state = DOWNLOAD_STATE.lock().unwrap();
        if state.is_running {
            return Html("<div style='color: red;'>Download already in progress.</div>").into_response();
        }
    }

    let movie_path = std::env::var("MOVIE_PATH").unwrap_or_else(|_| {
        eprintln!("MOVIE_PATH n'est pas défini dans le fichier .env");
        "/stockage/videos/films".to_string()
    });
    let video_path: String = std::env::var("VIDEO_PATH").unwrap_or_else(|_| {
        eprintln!("VIDEO_PATH n'est pas défini dans le fichier .env");
        "/stockage/videos".to_string()
    });
    let download_path = std::env::var("DOWNLOAD_PATH").unwrap_or_else(|_| {
        eprintln!("DOWNLOAD_PATH n'est pas défini dans le fichier .env");
        "/stockage/telechargements".to_string()
    });
    let target_dir = match payload.category.as_str() {
        "film" => movie_path,
        "video" => video_path,
        _ => download_path,
    };

    let is_youtube = payload.url.contains("youtube.com") || payload.url.contains("youtu.be");

        // Réinitialiser l'état
    {
        let mut state = DOWNLOAD_STATE.lock().unwrap();
        state.is_running = true;
        state.logs.clear();
        state.logs.push(format!("Download starting : {}", payload.url));
        state.child_pid = None;
        state.target_dir = Some(target_dir.clone());
    }

    std::thread::spawn(move || {
        let mut cmd = if is_youtube {
            let mut c = Command::new("yt-dlp");
            c.arg("--newline");
            c.arg("--no-colors");
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
    
    let mut state = DOWNLOAD_STATE.lock().unwrap();
    if let Some(pid) = state.child_pid {
        let _ = Command::new("kill").arg(pid.to_string()).output();

        // Nettoyage simple des fichiers temporaires (.part, .ytdl) dans le dossier cible
        if let Some(target_dir) = &state.target_dir {
            if let Ok(entries) = std::fs::read_dir(target_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "part" || ext == "ytdl" {
                            if let Ok(_) = std::fs::remove_file(&path) {
                                state.logs.push(format!("Fichier résiduel supprimé : {:?}", path.file_name().unwrap()));
                            }
                        }
                    }
                }
            }
        }
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
        r#"<button hx-post="/download/stop" class="btn-shutdown" style="margin-top: 10px; background-color: #d9534f;">Stop download</button>"#
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