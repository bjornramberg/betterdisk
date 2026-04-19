use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sysinfo::Disks;

use crate::scanner::{DirEntry, Scanner};

#[derive(Debug, Clone)]
pub struct MountInfo {
    pub mount_point: PathBuf,
    pub total_space: u64,
    pub used_space: u64,
    pub available_space: u64,
    pub fs_type: String,
    pub is_removable: bool,
}

pub struct AppState {
    pub mounts: Vec<MountInfo>,
    pub selected_mount: usize,
    pub current_path: PathBuf,
    pub root_path: PathBuf,
    pub current_entry: Option<DirEntry>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub is_scanning: bool,
    pub scan_complete: Arc<Mutex<bool>>,
    pub scanner: Scanner,
}

impl AppState {
    pub fn new() -> Self {
        let mut state = Self {
            mounts: vec![],
            selected_mount: 0,
            current_path: PathBuf::from("/"),
            root_path: PathBuf::from("/"),
            current_entry: None,
            selected_index: 0,
            scroll_offset: 0,
            is_scanning: false,
            scan_complete: Arc::new(Mutex::new(false)),
            scanner: Scanner::new(),
        };
        state.refresh_mounts();
        state
    }

    pub fn refresh_mounts(&mut self) {
        let disks = Disks::new_with_refreshed_list();
        let mut mounts: Vec<MountInfo> = vec![];

        for disk in disks.list() {
            let mount_point = disk.mount_point().to_path_buf();
            if !mount_point.to_string_lossy().starts_with("/sys") &&
               !mount_point.to_string_lossy().starts_with("/proc") &&
               !mount_point.to_string_lossy().starts_with("/dev") &&
               !mount_point.to_string_lossy().starts_with("/run") {
                mounts.push(MountInfo {
                    mount_point,
                    total_space: disk.total_space(),
                    used_space: disk.total_space() - disk.available_space(),
                    available_space: disk.available_space(),
                    fs_type: disk.file_system().to_string_lossy().to_string(),
                    is_removable: disk.is_removable(),
                });
            }
        }

        self.mounts = mounts;
    }

    pub fn select_mount(&mut self, index: usize) {
        if index < self.mounts.len() {
            self.selected_mount = index;
            self.root_path = self.mounts[index].mount_point.clone();
            self.current_path = self.root_path.clone();
            self.current_entry = None;
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
    }

    pub fn get_selected_mount(&self) -> Option<&MountInfo> {
        self.mounts.get(self.selected_mount)
    }
}