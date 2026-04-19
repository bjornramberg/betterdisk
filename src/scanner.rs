use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub children: Vec<DirEntry>,
}

pub struct Scanner {
    total_size: Arc<AtomicU64>,
    cancel_flag: Arc<AtomicBool>,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            total_size: Arc::new(AtomicU64::new(0)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub fn scan_dir(&self, path: &PathBuf, max_depth: usize) -> DirEntry {
        self.cancel_flag.store(false, Ordering::SeqCst);
        self.total_size.store(0, Ordering::SeqCst);

        self.scan_recursive(path, max_depth, 0)
    }

    fn scan_recursive(&self, path: &PathBuf, max_depth: usize, current_depth: usize) -> DirEntry {
        if self.cancel_flag.load(Ordering::SeqCst) {
            return DirEntry {
                path: path.clone(),
                size: 0,
                is_dir: true,
                children: vec![],
            };
        }

        let mut total_size: u64 = 0;
        let mut children: Vec<DirEntry> = vec![];

        if !path.is_dir() {
            if let Ok(meta) = path.metadata() {
                total_size = meta.len();
            }
            return DirEntry {
                path: path.clone(),
                size: total_size,
                is_dir: path.is_dir(),
                children: vec![],
            };
        }

        if current_depth < max_depth || max_depth == 0 {
            for entry in WalkDir::new(path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if self.cancel_flag.load(Ordering::SeqCst) {
                    break;
                }

                let entry_path = entry.path().to_path_buf();
                let entry_meta = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if entry_path.is_symlink() {
                    continue;
                }

                if entry_meta.is_dir() {
                    let child =
                        self.scan_recursive(&entry_path, if max_depth == 0 { 0 } else { max_depth - 1 },
 current_depth + 1);
                    total_size += child.size;
                    children.push(child);
                } else if entry_meta.is_file() {
                    total_size += entry_meta.len();
                }
            }

            children.sort_by(|a, b| b.size.cmp(&a.size));
        } else {
            for entry in WalkDir::new(path)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if self.cancel_flag.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        total_size += meta.len();
                    }
                }
            }
        }

        self.total_size.fetch_add(total_size, Ordering::SeqCst);

        DirEntry {
            path: path.clone(),
            size: total_size,
            is_dir: true,
            children,
        }
    }

    pub fn get_total_size(&self) -> u64 {
        self.total_size.load(Ordering::SeqCst)
    }
}

pub fn calculate_dir_size(path: &PathBuf) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}