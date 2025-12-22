use std::sync::Mutex;
use sysinfo::System;
use lazy_static::lazy_static;

pub struct DownloadState {
    pub is_running: bool,
    pub logs: Vec<String>,
    pub child_pid: Option<u32>,
}

lazy_static! {
    pub static ref SYS: Mutex<System> = Mutex::new(System::new_all());
    pub static ref DOWNLOAD_STATE: Mutex<DownloadState> = Mutex::new(DownloadState {
        is_running: false,
        logs: Vec::new(),
        child_pid: None,
    });
}