use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

#[derive(Template)]
#[template(path = "index.html")]
pub struct DashboardTemplate {
    pub cpu_usage: u32,
    pub cpu_model: String,
    pub cpu_temp: String,
    pub cpu_temp_val: f32,
    pub total_memory: String,
    pub used_memory: String,
    pub memory_percentage: u32,
    pub disks: Vec<DiskInfo>,
    pub declin_web_status: bool,
    pub samba_status: bool,
    pub minidlna_status: bool,
    pub is_authenticated: bool,
    pub server_power: String,
}

pub struct DiskInfo {
    pub name: String,
    pub total: String,
    pub used: String,
    pub percentage: u32,
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