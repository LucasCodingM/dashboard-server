use axum::{
    extract::Path,
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
};
use std::ffi::OsStr;
use std::process::Command;
use sysinfo::{System, Components, Disks};
use std::collections::{HashMap, HashSet};
use crate::utils;
use crate::state::{SYS, COMPONENTS, DISKS, POWER_CONSUMPTION};
use crate::templates::{DashboardTemplate, DiskInfo};
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

    let mut components = COMPONENTS.lock().unwrap();
    components.refresh(true);

    let mut disks = DISKS.lock().unwrap();
    disks.refresh(true);
    
    let (cpu_usage, cpu_model, cpu_temp, cpu_temp_val) = get_cpu_info(&sys, &components);
    let (total_memory, used_memory, memory_percentage) = get_memory_info(&sys);
    let disks_info = get_disks_info(&disks);
    let (declin_web_status, samba_status, minidlna_status) = get_services_status(&sys);

    let power_val = *POWER_CONSUMPTION.lock().unwrap();
    let server_power = format!("{:.2} W", power_val);

    let is_authenticated = check_auth(&headers);

    DashboardTemplate {
        cpu_usage,
        cpu_model,
        cpu_temp,
        cpu_temp_val,
        total_memory,
        used_memory,
        memory_percentage,
        disks: disks_info,
        declin_web_status,
        samba_status,
        minidlna_status,
        is_authenticated,
        server_power,
    }
}

fn get_cpu_info(sys: &System, components: &Components) -> (u32, String, String, f32) {
    let usage = sys.global_cpu_usage() as u32;
    
    let model = sys.cpus().first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let mut temp_val = 0.0;
    for component in components.iter() {
        let label = component.label().to_lowercase();
        if label.contains("cpu") || label.contains("core") || label.contains("package") || label.contains("tctl") {
            if let Some(t) = component.temperature(){ // sysinfo retourne f32
            if t > temp_val {
                temp_val = t;
            }
        }
        }
    }
    let temp = format!("{:.0}°C", temp_val);

    (usage, model, temp, temp_val)
}

fn get_memory_info(sys: &System) -> (String, String, u32) {
    let total_mem = utils::human_readable_bytes(sys.total_memory());
    let used_mem = utils::human_readable_bytes(sys.used_memory());
    
    let mem_pct = if sys.total_memory() > 0 {
        ((sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0) as u32
    } else {
        0
    };
    
    (total_mem, used_mem, mem_pct)
}

fn get_disks_info(disks: &Disks) -> Vec<DiskInfo> {
    let mut disk_map: HashMap<String, (u64, u64)> = HashMap::new();
    let mut processed_partitions: HashSet<String> = HashSet::new();

    for disk in disks {
        let name = disk.name().to_string_lossy();

        // Éviter de compter deux fois la même partition si elle est montée à plusieurs endroits
        if processed_partitions.contains(name.as_ref()) {
            continue;
        }

        // Filtrer les périphériques virtuels (loop, ram, cd-rom)
        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("sr") {
            continue;
        }

        // Filtrer les systèmes de fichiers virtuels
        let fs = disk.file_system().to_string_lossy();
        if fs == "squashfs" || fs == "tmpfs" || fs == "overlay" || fs == "devtmpfs" {
            continue;
        }

        processed_partitions.insert(name.to_string());

        if fs == "zfs" {
            let pool_name = name.split('/').next().unwrap_or(&name).to_string();
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available); // Espace consommé par ce dataset précis

            let used_for_display = std::cmp::min(used, total);

            let entry = disk_map.entry(pool_name).or_insert((0, 0));
            
            // 1. L'espace TOTAL du pool est le MAX rapporté (hors quotas)
            // On prend le max car les datasets avec quotas afficheront une taille inférieure.
            entry.0 = std::cmp::max(entry.0, total);
            
            // 2. L'espace UTILISÉ est la SOMME de tous les datasets du pool
            entry.1 += used_for_display; 
            
            continue;
        }

        // Regrouper par nom de disque physique (ex: sda1 -> sda, nvme0n1p1 -> nvme0n1)
        let mut base_name = name.to_string();
        if base_name.starts_with("nvme") || base_name.starts_with("mmcblk") {
             if let Some(idx) = base_name.rfind('p') {
                 if base_name[idx+1..].chars().all(|c| c.is_ascii_digit()) {
                     base_name = base_name[..idx].to_string();
                 }
             }
        } else {
            let trimmed = base_name.trim_end_matches(|c: char| c.is_ascii_digit());
            if !trimmed.is_empty() {
                base_name = trimmed.to_string();
            }
        }

        let total = disk.total_space();
        let available = disk.available_space();
        let used = total - available;

        let entry = disk_map.entry(base_name).or_insert((0, 0));
        entry.0 += total;
        entry.1 += used;
    }

    let mut result: Vec<DiskInfo> = disk_map.into_iter().map(|(name, (total, used))| {
        let percentage = if total > 0 {
            ((used as f64 / total as f64) * 100.0) as u32
        } else {
            0
        };
        
        DiskInfo {
            name,
            total: utils::human_readable_bytes(total),
            used: utils::human_readable_bytes(used),
            percentage,
        }
    }).collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

fn check_declin_web_status() -> bool {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let path = format!("{}/izeria/declin-web", home);
    Command::new("docker")
        .args(["compose", "--profile", "mt5", "ps", "-q"])
        .current_dir(&path)
        .output()
        .map(|output| !output.stdout.trim_ascii().is_empty())
        .unwrap_or(false)
}

fn get_services_status(sys: &System) -> (bool, bool, bool) {
    let declin_web_status = check_declin_web_status();
    let samba_status = sys.processes_by_name(OsStr::new("smbd")).next().is_some();
    let minidlna_status = sys.processes_by_name(OsStr::new("minidlna")).next().is_some();

    (declin_web_status, samba_status, minidlna_status)
}

pub async fn service_handler(Path((service, action)): Path<(String, String)>, headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    if service == "declin-web" {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let path = format!("{}/izeria/declin-web", home);
        let args: &[&str] = match action.as_str() {
            "start" => &["compose", "--profile", "mt5", "up", "-d", "--build"],
            "stop"  => &["compose", "--profile", "mt5", "down"],
            _ => return (StatusCode::BAD_REQUEST, "Invalid action").into_response(),
        };
        return match Command::new("docker").args(args).current_dir(&path).status() {
            Ok(status) if status.success() => StatusCode::OK.into_response(),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Command failed").into_response(),
        };
    }

    let service_name = match service.as_str() {
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
