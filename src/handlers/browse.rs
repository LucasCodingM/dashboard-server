use axum::{
    extract::Query,
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Html},
};
use serde::Deserialize;
use std::path::Path;

use crate::auth::check_auth;

#[derive(Deserialize)]
pub struct BrowseQuery {
    path: Option<String>,
}

pub async fn browse_handler(headers: HeaderMap, Query(q): Query<BrowseQuery>) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let root = std::env::var("BROWSE_ROOT").unwrap_or_else(|_| "/stockage".to_string());
    let root = root.trim_end_matches('/');

    let raw_path = q.path.unwrap_or_else(|| root.to_string());
    let path = Path::new(&raw_path);

    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return Html(r##"<div style="color:#d9534f;font-family:monospace;padding:8px;">Chemin invalide ou inaccessible.</div>"##.to_string()).into_response(),
    };

    if !canonical.starts_with(root) {
        return Html(format!(
            r##"<div style="color:#d9534f;font-family:monospace;padding:8px;">Accès refusé : hors de {}.</div>"##,
            root
        )).into_response();
    }

    let path_str = canonical.to_string_lossy().to_string();

    let mut dirs: Vec<String> = match std::fs::read_dir(&canonical) {
        Ok(rd) => rd
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| !name.starts_with('.'))
            .collect(),
        Err(_) => return Html(r##"<div style="color:#d9534f;font-family:monospace;padding:8px;">Permission refusée.</div>"##.to_string()).into_response(),
    };
    dirs.sort();

    let parent = canonical.parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string());

    let item_style = "padding:7px 10px;cursor:pointer;border-radius:4px;font-family:monospace;font-size:0.88em;display:flex;align-items:center;gap:8px;border-bottom:1px solid #1e1e1e;";

    let mut html = format!(
        r##"<div data-current-path="{}" style="display:none;"></div>"##,
        esc(&path_str)
    );

    if path_str != root {
        html.push_str(&format!(
            r##"<div class="fb-item" hx-get="/browse?path={}" hx-target="#browser-content" hx-swap="innerHTML" style="{}">&uarr;&nbsp;..</div>"##,
            esc(&parent), item_style
        ));
    }

    if dirs.is_empty() {
        html.push_str(r##"<div style="color:#666;font-family:monospace;font-size:0.85em;padding:10px;">Aucun sous-dossier</div>"##);
    }

    for dir in &dirs {
        let full_path = format!("{}/{}", path_str.trim_end_matches('/'), dir);
        html.push_str(&format!(
            r##"<div class="fb-item" hx-get="/browse?path={}" hx-target="#browser-content" hx-swap="innerHTML" style="{}">&#128193;&nbsp;{}</div>"##,
            esc(&full_path), item_style, esc(dir)
        ));
    }

    Html(html).into_response()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}
