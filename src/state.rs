use std::sync::Mutex;
use sysinfo::{System, Components, Disks, Networks};
use lazy_static::lazy_static;

pub struct DownloadTask {
    pub id: u32,
    pub url: String,
    pub is_running: bool,
    pub logs: Vec<String>,
    pub child_pid: Option<u32>,
    pub target_dir: Option<String>,
}

pub struct DownloadState {
    pub tasks: Vec<DownloadTask>,
    pub next_id: u32,
}

impl DownloadState {
    pub fn add_task(&mut self, url: String, target_dir: String) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(DownloadTask {
            id,
            url,
            is_running: true,
            logs: Vec::new(),
            child_pid: None,
            target_dir: Some(target_dir),
        });
        id
    }
}

pub struct CachedContainerInfo {
    pub name: String,
    pub is_running: bool,
    pub cpu: String,
    pub memory: String,
    pub net_io: String,
}

pub struct CachedServiceStatus {
    pub declin_web: bool,
    pub declin_discord: bool,
    pub trading: bool,
    pub samba: bool,
    pub minidlna: bool,
}

lazy_static! {
    pub static ref SYS: Mutex<System> = Mutex::new(System::new_all());
    pub static ref COMPONENTS: Mutex<Components> = Mutex::new(Components::new_with_refreshed_list());
    pub static ref DISKS: Mutex<Disks> = Mutex::new(Disks::new_with_refreshed_list());
    pub static ref NETWORKS: Mutex<Networks> = Mutex::new(Networks::new_with_refreshed_list());
    pub static ref DOWNLOAD_STATE: Mutex<DownloadState> = Mutex::new(DownloadState {
        tasks: Vec::new(),
        next_id: 0,
    });
    pub static ref POWER_CONSUMPTION: Mutex<f32> = Mutex::new(0.0);
    pub static ref NET_DATA: Mutex<(u64, u64)> = Mutex::new((0, 0));
    pub static ref DOCKER_CACHE: Mutex<Vec<CachedContainerInfo>> = Mutex::new(Vec::new());
    pub static ref SERVICE_CACHE: Mutex<CachedServiceStatus> = Mutex::new(CachedServiceStatus {
        declin_web: false,
        declin_discord: false,
        trading: false,
        samba: false,
        minidlna: false,
    });
}
