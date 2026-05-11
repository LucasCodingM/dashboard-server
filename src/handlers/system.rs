use axum::{
    extract::Path,
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
};
use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;
use sysinfo::{System, Components, Disks};
use std::collections::{HashMap, HashSet};
use crate::utils;
use crate::state::{SYS, COMPONENTS, DISKS, POWER_CONSUMPTION, NET_DATA, NETWORKS, DOCKER_CACHE, SERVICE_CACHE, CachedContainerInfo, CachedServiceStatus};
use crate::templates::{DashboardTemplate, DiskInfo, ProcessInfo, NetworkInfo, ContainerInfo};
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


pub fn start_docker_polling() {
    std::thread::spawn(|| loop {
        let containers = poll_docker_containers();
        *DOCKER_CACHE.lock().unwrap() = containers;

        let status = poll_service_status();
        *SERVICE_CACHE.lock().unwrap() = status;

        std::thread::sleep(Duration::from_secs(5));
    });
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
    let top_cpu = get_top_cpu_processes(&sys);
    let top_mem = get_top_mem_processes(&sys);
    let network = get_network_info();

    let containers = {
        let cache = DOCKER_CACHE.lock().unwrap();
        cache.iter().map(|c| ContainerInfo {
            name: c.name.clone(),
            is_running: c.is_running,
            cpu: c.cpu.clone(),
            memory: c.memory.clone(),
            net_io: c.net_io.clone(),
        }).collect::<Vec<_>>()
    };
    let (declin_web_status, declin_discord_status, trading_status, samba_status, minidlna_status) = {
        let s = SERVICE_CACHE.lock().unwrap();
        (s.declin_web, s.declin_discord, s.trading, s.samba, s.minidlna)
    };

    let power_val = *POWER_CONSUMPTION.lock().unwrap();
    let server_power = format!("{:.2} W", power_val);
    let uptime_str = format_uptime(System::uptime());

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
        top_cpu,
        top_mem,
        network,
        containers,
        declin_web_status,
        declin_discord_status,
        trading_status,
        samba_status,
        minidlna_status,
        is_authenticated,
        server_power,
        uptime_str,
    }
}

fn get_network_info() -> NetworkInfo {
    let mut networks = NETWORKS.lock().unwrap();
    networks.refresh(true);
    
    let mut current_rx = 0;
    let mut current_tx = 0;

    for (_, data) in networks.iter() {
        current_rx += data.received();
        current_tx += data.transmitted();
    }

    let mut last_data = NET_DATA.lock().unwrap();
    let rx_speed = if current_rx >= last_data.0 { current_rx - last_data.0 } else { 0 };
    let tx_speed = if current_tx >= last_data.1 { current_tx - last_data.1 } else { 0 };
    
    *last_data = (current_rx, current_tx);

    NetworkInfo {
        rx_speed: format!("{}/s", utils::human_readable_bytes(rx_speed)),
        tx_speed: format!("{}/s", utils::human_readable_bytes(tx_speed)),
        rx_val: rx_speed,
        tx_val: tx_speed,
    }
}

fn poll_docker_containers() -> Vec<CachedContainerInfo> {
    let output = Command::new("docker")
        .args([
            "stats",
            "--no-stream",
            "--format",
            "{{.Name}}|{{.CPUPerc}}|{{.MemUsage}}|{{.NetIO}}"
        ])
        .output();

    let mut stats_map = HashMap::new();
    if let Ok(o) = output {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() == 4 {
                stats_map.insert(parts[0].to_string(), (
                    parts[1].to_string(),
                    parts[2].to_string(),
                    parts[3].to_string(),
                ));
            }
        }
    }

    let ps_output = Command::new("docker")
        .args(["ps", "-a", "--format", "{{.Names}}|{{.State}}"])
        .output();

    match ps_output {
        Ok(o) => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| {
                    let parts: Vec<&str> = line.split('|').collect();
                    let name = parts.get(0).unwrap_or(&"unknown").to_string();
                    let (cpu, mem, net) = stats_map.get(&name)
                        .cloned()
                        .unwrap_or(("--".into(), "-- / --".into(), "-- / --".into()));

                    CachedContainerInfo {
                        name,
                        is_running: parts.get(1).map(|&s| s == "running").unwrap_or(false),
                        cpu,
                        memory: mem,
                        net_io: net,
                    }
                })
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

fn get_top_cpu_processes(sys: &System) -> Vec<ProcessInfo> {
    let mut processes: Vec<_> = sys.processes().values().collect();
    processes.sort_by(|a, b| b.cpu_usage().partial_cmp(&a.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal));

    let total_mem = sys.total_memory() as f64;

    processes.iter().take(5).map(|p| {
        let mem_pct = if total_mem > 0.0 { (p.memory() as f64 / total_mem) * 100.0 } else { 0.0 };
        ProcessInfo {
            name: p.name().to_string_lossy().to_string(),
            memory: utils::human_readable_bytes(p.memory()),
            cpu: format!("{:.1}%", p.cpu_usage()),
            memory_pct: format!("{:.1}%", mem_pct),
        }
    }).collect()
}

fn get_top_mem_processes(sys: &System) -> Vec<ProcessInfo> {
    let mut processes: Vec<_> = sys.processes().values().collect();
    processes.sort_by_key(|p| p.memory());
    processes.reverse();

    let total_mem = sys.total_memory() as f64;

    processes.iter().take(5).map(|p| {
        let mem_pct = if total_mem > 0.0 { (p.memory() as f64 / total_mem) * 100.0 } else { 0.0 };
        ProcessInfo {
            name: p.name().to_string_lossy().to_string(),
            memory: utils::human_readable_bytes(p.memory()),
            cpu: format!("{:.1}%", p.cpu_usage()),
            memory_pct: format!("{:.1}%", mem_pct),
        }
    }).collect()
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{}j {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
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

fn tcp_up(addr: &str) -> bool {
    TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(300)).is_ok()
}

fn check_declin_web_status(container_name: &str) -> bool {
    Command::new("docker")
        .args(["inspect", "--format={{.State.Running}}", container_name])
        .output()
        .map(|o| o.stdout.starts_with(b"true"))
        .unwrap_or(false)
}

fn check_trading_status() -> bool {
    Command::new("docker")
        .args(["inspect", "declin-discord", "--format", "{{range .Config.Env}}{{println .}}{{end}}"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l == "ENABLE_TRADING=true"))
        .unwrap_or(false)
}

fn poll_service_status() -> CachedServiceStatus {
    CachedServiceStatus {
        declin_web:     check_declin_web_status("declin-web"),
        declin_discord: check_declin_web_status("declin-discord"),
        trading:        check_trading_status(),
        samba:          tcp_up("127.0.0.1:445"),
        minidlna:       tcp_up("127.0.0.1:8200"),
    }
}

pub async fn service_handler(Path((service, action)): Path<(String, String)>, headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    if service == "declin-discord" {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let compose_dir = std::env::var("DECLIN_DISCORD_PATH")
            .unwrap_or_else(|_| format!("{}/izeria/declin-discord", home));
        let args: &[&str] = match action.as_str() {
            "start"          => &["compose", "-f", "docker-compose.yml", "up", "-d", "--build"],
            "stop"           => &["compose", "-f", "docker-compose.yml", "down"],
            "trading-enable" => &["compose", "-f", "docker-compose.yml", "-f", "docker-compose.trading.yml", "up", "-d", "--force-recreate"],
            "trading-disable"=> &["compose", "-f", "docker-compose.yml", "up", "-d", "--force-recreate"],
            _ => return (StatusCode::BAD_REQUEST, "Invalid action").into_response(),
        };
        return match Command::new("docker").args(args).current_dir(&compose_dir).status() {
            Ok(status) if status.success() => StatusCode::OK.into_response(),
            Ok(status) => (StatusCode::INTERNAL_SERVER_ERROR, format!("docker exited with {status}")).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run docker: {e}")).into_response(),
        };
    }

    if service == "declin-web" {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let path = std::env::var("DECLIN_WEB_PATH")
            .unwrap_or_else(|_| format!("{}/izeria/declin-web", home));
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
