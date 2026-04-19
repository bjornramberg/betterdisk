use std::path::PathBuf;

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

#[derive(Debug, Clone)]
pub struct TreemapCell {
    pub path: PathBuf,
    pub size: u64,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

pub struct AppState {
    pub mounts: Vec<MountInfo>,
    pub selected_mount: usize,
    pub current_path: PathBuf,
    pub root_path: PathBuf,
    pub current_entry: Option<DirEntry>,
    pub is_scanning: bool,
    pub scanner: Scanner,
    pub show_drive_selector: bool,
    pub selected_cell_idx: usize,
    pub treemap_cells: Vec<TreemapCell>,
    pub drive_filter: String,
}

impl AppState {
    pub fn new() -> Self {
        let mut state = Self {
            mounts: vec![],
            selected_mount: 0,
            current_path: PathBuf::from("/"),
            root_path: PathBuf::from("/"),
            current_entry: None,
            is_scanning: false,
            scanner: Scanner::new(),
            show_drive_selector: true,
            selected_cell_idx: 0,
            treemap_cells: vec![],
            drive_filter: String::new(),
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
               !mount_point.to_string_lossy().starts_with("/run") &&
               mount_point.to_string_lossy() != "/" {
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

        let disks = Disks::new_with_refreshed_list();
        for d in disks.list() {
            if d.mount_point().to_string_lossy() == "/" {
                mounts.insert(0, MountInfo {
                    mount_point: PathBuf::from("/"),
                    total_space: d.total_space(),
                    used_space: d.total_space() - d.available_space(),
                    available_space: d.available_space(),
                    fs_type: d.file_system().to_string_lossy().to_string(),
                    is_removable: d.is_removable(),
                });
                break;
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
            self.treemap_cells.clear();
            self.selected_cell_idx = 0;
            self.show_drive_selector = false;
        }
    }

    pub fn get_selected_mount(&self) -> Option<&MountInfo> {
        self.mounts.get(self.selected_mount)
    }

    pub fn toggle_drive_selector(&mut self) {
        self.show_drive_selector = !self.show_drive_selector;
        if self.show_drive_selector {
            self.drive_filter.clear();
        }
    }

    pub fn build_treemap(&mut self, width: u16, height: u16) {
        self.treemap_cells.clear();
        self.selected_cell_idx = 0;

        let entry = match &self.current_entry {
            Some(e) => e,
            None => return,
        };

        if entry.children.is_empty() {
            return;
        }

        let mut cells: Vec<TreemapCell> = vec![];
        self.squarify(
            &entry.children,
            0,
            0,
            width.saturating_sub(1),
            height.saturating_sub(1),
            entry.size,
            &mut cells,
        );

        self.treemap_cells = cells;
    }

    fn squarify(
        &self,
        children: &[DirEntry],
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        total_size: u64,
        cells: &mut Vec<TreemapCell>,
    ) {
        if children.is_empty() || width < 3 || height < 3 {
            return;
        }

        let total: u64 = children.iter().map(|c| c.size).sum();
        if total == 0 {
            return;
        }

        let horizontal = width >= height;
        let mut current_pos = if horizontal { x } else { y };
        let span = if horizontal { width } else { height };

        for child in children {
            let ratio = child.size as f64 / total as f64;
            let cell_size = (span as f64 * ratio) as u16;

            if cell_size < 2 {
                continue;
            }

            let cell = TreemapCell {
                path: child.path.clone(),
                size: child.size,
                x: if horizontal { current_pos } else { x },
                y: if horizontal { y } else { current_pos },
                width: if horizontal { cell_size } else { width },
                height: if horizontal { height } else { cell_size },
            };
            cells.push(cell);

            current_pos += cell_size;
        }
    }
}