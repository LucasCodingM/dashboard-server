use std::sync::Mutex;
use sysinfo::{System, Components, Disks};
use lazy_static::lazy_static;

pub struct DownloadState {
    pub is_running: bool,
    pub logs: Vec<String>,
    pub child_pid: Option<u32>,
    pub target_dir: Option<String>,
}

lazy_static! {
    pub static ref SYS: Mutex<System> = Mutex::new(System::new_all());
    pub static ref COMPONENTS: Mutex<Components> = Mutex::new(Components::new_with_refreshed_list());
    pub static ref DISKS: Mutex<Disks> = Mutex::new(Disks::new_with_refreshed_list());
    pub static ref DOWNLOAD_STATE: Mutex<DownloadState> = Mutex::new(DownloadState {
        is_running: false,
        logs: Vec::new(),
        child_pid: None,
        target_dir: None,
    });
    pub static ref POWER_CONSUMPTION: Mutex<f32> = Mutex::new(0.0);
}