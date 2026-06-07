use axum::{
    extract::Form,
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Html},
};
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

use crate::state::DOWNLOAD_STATE;
use crate::auth::check_auth;

#[derive(Deserialize)]
pub struct DownloadRequest {
    url: String,
    category: String,
}

#[derive(Deserialize)]
pub struct StopRequest {
    task_id: u32,
}

pub async fn download_handler(headers: HeaderMap, Form(payload): Form<DownloadRequest>) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let movie_path = std::env::var("MOVIE_PATH").unwrap_or_else(|_| "/stockage/videos/films".to_string());
    let video_path = std::env::var("VIDEO_PATH").unwrap_or_else(|_| "/stockage/videos".to_string());
    let download_path = std::env::var("DOWNLOAD_PATH").unwrap_or_else(|_| "/stockage/telechargements".to_string());

    let target_dir = match payload.category.as_str() {
        "film" => movie_path,
        "video" => video_path,
        _ => download_path,
    };

    let urls: Vec<String> = payload.url
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if urls.is_empty() {
        return Html("<div style='color: red;'>Aucune URL fournie.</div>").into_response();
    }

    for url in urls {
        let use_ytdlp = url.contains("youtube.com")
            || url.contains("youtu.be")
            || url.contains(".m3u8")
            || url.contains("m3u8");

        let task_id = {
            let mut state = DOWNLOAD_STATE.lock().unwrap();
            let id = state.add_task(url.clone(), target_dir.clone());
            state.tasks.last_mut().unwrap().logs.push(format!("Download starting : {}", url));
            id
        };

        let target_dir_clone = target_dir.clone();
        std::thread::spawn(move || {
            let mut cmd = if use_ytdlp {
                let mut c = Command::new("yt-dlp");
                c.arg("--newline");
                c.arg("--no-colors");
                c.arg("-o");
                c.arg(format!("{}/%(title)s.%(ext)s", target_dir_clone));
                c.arg(&url);
                c
            } else {
                let mut c = Command::new("wget");
                c.arg("-P");
                c.arg(&target_dir_clone);
                c.arg(&url);
                c
            };

            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            match cmd.spawn() {
                Ok(mut child) => {
                    let pid = child.id();
                    {
                        let mut state = DOWNLOAD_STATE.lock().unwrap();
                        if let Some(task) = state.tasks.iter_mut().find(|t| t.id == task_id) {
                            task.child_pid = Some(pid);
                            task.logs.push(format!("Processus lancé (PID: {})", pid));
                        }
                    }

                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();

                    fn handle_stream<R: std::io::Read + Send + 'static>(stream: Option<R>, task_id: u32) {
                        if let Some(s) = stream {
                            std::thread::spawn(move || {
                                let reader = BufReader::new(s);
                                for line in reader.lines() {
                                    if let Ok(l) = line {
                                        let mut state = DOWNLOAD_STATE.lock().unwrap();
                                        if let Some(task) = state.tasks.iter_mut().find(|t| t.id == task_id) {
                                            task.logs.push(l);
                                            if task.logs.len() > 200 {
                                                task.logs.remove(0);
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    }

                    handle_stream(stdout, task_id);
                    handle_stream(stderr, task_id);

                    let status = child.wait();
                    {
                        let mut state = DOWNLOAD_STATE.lock().unwrap();
                        if let Some(task) = state.tasks.iter_mut().find(|t| t.id == task_id) {
                            task.is_running = false;
                            task.child_pid = None;
                            match status {
                                Ok(s) => task.logs.push(format!("Terminé avec le code : {}", s)),
                                Err(e) => task.logs.push(format!("Erreur lors de l'attente du processus : {}", e)),
                            }
                        }
                    }
                }
                Err(e) => {
                    let mut state = DOWNLOAD_STATE.lock().unwrap();
                    if let Some(task) = state.tasks.iter_mut().find(|t| t.id == task_id) {
                        task.is_running = false;
                        task.logs.push(format!("Erreur au lancement : {}", e));
                    }
                }
            }
        });
    }

    Html(r#"
        <div id="download-status" hx-get="/download/logs" hx-trigger="load, every 1s" hx-target="this" hx-swap="outerHTML">
            Initialisation...
        </div>
    "#).into_response()
}

pub async fn stop_download_handler(headers: HeaderMap, Form(payload): Form<StopRequest>) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let mut state = DOWNLOAD_STATE.lock().unwrap();
    if let Some(task) = state.tasks.iter_mut().find(|t| t.id == payload.task_id) {
        if let Some(pid) = task.child_pid {
            let _ = Command::new("kill").arg(pid.to_string()).output();

            if let Some(target_dir) = &task.target_dir.clone() {
                if let Ok(entries) = std::fs::read_dir(target_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(ext) = path.extension() {
                            if ext == "part" || ext == "ytdl" {
                                if std::fs::remove_file(&path).is_ok() {
                                    task.logs.push(format!("Fichier résiduel supprimé : {:?}", path.file_name().unwrap()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    "".into_response()
}

pub async fn clear_completed_handler(headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let mut state = DOWNLOAD_STATE.lock().unwrap();
    state.tasks.retain(|t| t.is_running);

    "".into_response()
}

pub async fn get_download_logs() -> impl IntoResponse {
    let state = DOWNLOAD_STATE.lock().unwrap();

    if state.tasks.is_empty() {
        return Html(r#"<div id="download-status" hx-get="/download/logs" hx-trigger="every 2s" hx-target="this" hx-swap="outerHTML">
            <span style="color: var(--text-muted); font-family: monospace;">Aucun téléchargement...</span>
        </div>"#.to_string()).into_response();
    }

    let has_running = state.tasks.iter().any(|t| t.is_running);
    let has_completed = state.tasks.iter().any(|t| !t.is_running);

    let tasks_html: String = state.tasks.iter().map(|task| {
        let logs_html = task.logs.iter()
            .map(|l| l.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"))
            .collect::<Vec<_>>()
            .join("<br>");

        let url_display = {
            let escaped = task.url.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
            if escaped.len() > 80 {
                format!("{}...", &escaped[..80])
            } else {
                escaped
            }
        };

        let (status_color, status_label) = if task.is_running {
            ("#4CAF50", "EN COURS")
        } else {
            ("#888888", "TERMINÉ")
        };

        let stop_button = if task.is_running {
            format!(
                r#"<button hx-post="/download/stop" hx-vals='{{"task_id": {}}}' hx-swap="none" class="btn-shutdown" style="font-size:0.8em;padding:3px 10px;background-color:#d9534f;">Stop</button>"#,
                task.id
            )
        } else {
            String::new()
        };

        format!(
            r#"<div style="margin-bottom:1rem;border:1px solid #333;border-radius:4px;padding:8px;">
                <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:6px;gap:8px;">
                    <span style="color:{};font-family:monospace;font-size:0.8em;flex-shrink:0;">[{}]</span>
                    <span style="font-family:monospace;font-size:0.8em;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1;">{}</span>
                    {}
                </div>
                <div style="background:#1a1a1a;color:#ccc;padding:8px;border-radius:4px;max-height:180px;overflow-y:auto;font-family:monospace;font-size:0.82em;white-space:pre-wrap;">{}</div>
            </div>"#,
            status_color, status_label, url_display, stop_button,
            if logs_html.is_empty() { "Initialisation...".to_string() } else { logs_html }
        )
    }).collect();

    let clear_button = if has_completed {
        r#"<button hx-post="/download/clear" hx-swap="none" class="btn-primary" style="font-size:0.8em;padding:3px 10px;margin-bottom:10px;">Effacer terminés</button>"#
    } else {
        ""
    };

    let trigger = if has_running { "every 1s" } else { "every 5s" };

    Html(format!(
        r#"<div id="download-status" hx-get="/download/logs" hx-trigger="{}" hx-target="this" hx-swap="outerHTML">
            {}
            {}
        </div>"#,
        trigger, clear_button, tasks_html
    )).into_response()
}
