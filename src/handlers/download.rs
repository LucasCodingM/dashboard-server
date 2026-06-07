use axum::{
    extract::{Form, Path},
    http::{StatusCode, HeaderMap},
    response::{sse::{Event, Sse, KeepAlive}, IntoResponse, Html},
};
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::convert::Infallible;
use std::time::Duration;
use futures_util::stream::Stream;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::state::DOWNLOAD_STATE;
use crate::auth::check_auth;

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct StringOrVec;

    impl<'de> serde::de::Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("string or sequence of strings")
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(vec![v.to_owned()])
        }

        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut v = Vec::new();
            while let Some(s) = seq.next_element()? {
                v.push(s);
            }
            Ok(v)
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

#[derive(Deserialize)]
pub struct DownloadRequest {
    #[serde(deserialize_with = "deserialize_string_or_vec")]
    url: Vec<String>,
    category: String,
    custom_path: Option<String>,
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
        "custom" => payload.custom_path
            .filter(|p| !p.trim().is_empty())
            .map(|p| p.trim().to_string())
            .unwrap_or(download_path),
        _ => download_path,
    };

    let urls: Vec<String> = payload.url
        .into_iter()
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
        .collect();

    if urls.is_empty() {
        return Html("<div style='color:red;'>Aucune URL fournie.</div>").into_response();
    }

    let mut task_ids: Vec<u32> = Vec::new();

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
        task_ids.push(task_id);

        let target_dir_clone = target_dir.clone();
        std::thread::spawn(move || {
            let mut cmd = if use_ytdlp {
                let mut c = Command::new("yt-dlp");
                c.arg("--newline").arg("--no-colors");
                c.arg("--merge-output-format").arg("mp4");
                c.arg("-o").arg(format!("{}/%(title)s.%(ext)s", target_dir_clone));
                c.arg(&url);
                c
            } else {
                let mut c = Command::new("wget");
                c.arg("-P").arg(&target_dir_clone).arg(&url);
                c
            };

            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

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
                                Err(e) => task.logs.push(format!("Erreur : {}", e)),
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

    // Retourne une carte par task avec log vide — SSE remplira les logs côté client
    let cards: String = task_ids.iter().map(|&id| {
        let (url_display, url_escaped) = {
            let state = DOWNLOAD_STATE.lock().unwrap();
            let url = state.tasks.iter().find(|t| t.id == id)
                .map(|t| t.url.clone())
                .unwrap_or_default();
            let escaped = url.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
            let display = if escaped.len() > 80 { format!("{}...", &escaped[..80]) } else { escaped.clone() };
            (display, escaped)
        };
        format!(r#"<div class="task-card" id="task-card-{id}" style="margin-bottom:1rem;border:1px solid #333;border-radius:4px;padding:8px;">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:6px;gap:8px;">
                <span id="task-badge-{id}" style="color:#4CAF50;font-family:monospace;font-size:0.8em;flex-shrink:0;">[EN COURS]</span>
                <span style="font-family:monospace;font-size:0.8em;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1;" title="{url_escaped}">{url_display}</span>
                <button id="task-stop-{id}" hx-post="/download/stop" hx-vals='{{"task_id":{id}}}' hx-swap="none" class="btn-shutdown" style="flex-shrink:0;font-size:0.8em;padding:3px 10px;background:#d9534f;">Stop</button>
            </div>
            <div id="task-log-{id}" style="background:#1a1a1a;color:#ccc;padding:8px;border-radius:4px;height:200px;overflow-y:auto;font-family:monospace;font-size:0.82em;white-space:pre-wrap;"></div>
        </div>
        <script>initTaskSSE({id});</script>"#)
    }).collect();

    Html(format!(r#"<div id="download-tasks-container">{cards}
        <button onclick="clearDoneTasks()" class="btn-primary" style="font-size:0.8em;padding:4px 12px;margin-top:4px;">Effacer terminés</button>
    </div>"#)).into_response()
}

// SSE : pousse les lignes de log au fur et à mesure pour un task donné
pub async fn task_log_sse(Path(task_id): Path<u32>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Event>(256);

    tokio::spawn(async move {
        let mut offset = 0usize;
        loop {
            let (new_lines, is_done) = {
                let state = DOWNLOAD_STATE.lock().unwrap();
                match state.tasks.iter().find(|t| t.id == task_id) {
                    Some(task) => {
                        let start = offset.min(task.logs.len());
                        let new: Vec<String> = task.logs[start..].to_vec();
                        let done = !task.is_running;
                        offset = start + new.len();
                        (new, done)
                    }
                    None => (vec![], true),
                }
            };

            for line in new_lines {
                if tx.send(Event::default().event("log").data(line)).await.is_err() {
                    return;
                }
            }

            if is_done {
                // Assure que toutes les lignes ont été envoyées avant de signaler la fin
                tokio::time::sleep(Duration::from_millis(50)).await;
                let remaining = {
                    let state = DOWNLOAD_STATE.lock().unwrap();
                    state.tasks.iter().find(|t| t.id == task_id)
                        .map(|t| t.logs[offset.min(t.logs.len())..].to_vec())
                        .unwrap_or_default()
                };
                for line in remaining {
                    if tx.send(Event::default().event("log").data(line)).await.is_err() {
                        return;
                    }
                }
                let exit_msg = {
                    let state = DOWNLOAD_STATE.lock().unwrap();
                    state.tasks.iter().find(|t| t.id == task_id)
                        .map(|t| t.logs.last().cloned().unwrap_or_default())
                        .unwrap_or_default()
                };
                let _ = tx.send(Event::default().event("done").data(exit_msg)).await;
                return;
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });

    let stream = ReceiverStream::new(rx).map(Ok::<Event, Infallible>);
    Sse::new(stream).keep_alive(KeepAlive::default())
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

// Endpoint legacy conservé pour compatibilité (non utilisé par le front SSE)
pub async fn get_download_logs() -> impl IntoResponse {
    let state = DOWNLOAD_STATE.lock().unwrap();
    let running_count = state.tasks.iter().filter(|t| t.is_running).count();
    Html(format!(
        r#"<span style="font-family:monospace;color:var(--text-muted);">{} actif(s), {} terminé(s)</span>"#,
        running_count,
        state.tasks.iter().filter(|t| !t.is_running).count()
    )).into_response()
}

