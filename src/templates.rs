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
    pub top_cpu: Vec<ProcessInfo>,
    pub top_mem: Vec<ProcessInfo>,
    pub network: NetworkInfo,
    pub containers: Vec<ContainerInfo>,
    pub declin_web_status: bool,
    pub declin_discord_status: bool,
    pub trading_status: bool,
    pub samba_status: bool,
    pub minidlna_status: bool,
    pub is_authenticated: bool,
    pub server_power: String,
    pub uptime_str: String,
}

pub struct NetworkInfo {
    pub rx_speed: String,
    pub tx_speed: String,
    pub rx_val: u64,
    pub tx_val: u64,
}

pub struct ContainerInfo {
    pub name: String,
    pub is_running: bool,
    pub cpu: String,
    pub memory: String,
    pub net_io: String,
}

pub struct DiskInfo {
    pub name: String,
    pub total: String,
    pub used: String,
    pub percentage: u32,
}

pub struct ProcessInfo {
    pub name: String,
    pub memory: String,
    pub cpu: String,
    pub memory_pct: String,
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