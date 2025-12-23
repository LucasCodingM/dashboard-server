use axum::{
    extract::Path,
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
};
use std::ffi::OsStr;
use std::process::Command;
use crate::utils;
use crate::state::SYS;
use crate::templates::DashboardTemplate;
use crate::auth::check_auth;



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



