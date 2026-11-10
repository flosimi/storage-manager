use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sysinfo::{DiskKind, Disks};
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::util::percent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveKind {
    Ssd,
    Hdd,
    Unknown,
}

/// A mounted volume and its capacity, in bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub kind: DriveKind,
    pub total: u64,
    pub available: u64,
    pub removable: bool,
}

impl DiskInfo {
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }

    pub fn used_percent(&self) -> f64 {
        percent(self.used(), self.total)
    }
}

/// Lists mounted volumes, sorted by mount point.
pub fn list_disks() -> Result<Vec<DiskInfo>> {
    let disks = Disks::new_with_refreshed_list().map_err(|e| Error::Sys(e.to_string()))?;
    let mut out: Vec<DiskInfo> = disks
        .iter()
        .map(|d| DiskInfo {
            name: d.name().to_string_lossy().into_owned(),
            mount_point: d.mount_point().to_string_lossy().into_owned(),
            file_system: d.file_system().to_string_lossy().into_owned(),
            kind: match d.kind() {
                DiskKind::SSD => DriveKind::Ssd,
                DiskKind::HDD => DriveKind::Hdd,
                DiskKind::Unknown(_) => DriveKind::Unknown,
            },
            total: d.total_space(),
            available: d.available_space(),
            removable: d.is_removable(),
        })
        .collect();
    out.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    Ok(out)
}

/// A file or folder with its total size in bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
}

/// Result of scanning a directory tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScanResult {
    pub root: String,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    /// Entries that could not be read (permissions, races, etc.).
    pub skipped: u64,
    /// Immediate children of `root` with aggregated sizes, largest first.
    /// Scan a child to drill down.
    pub children: Vec<Entry>,
    /// The largest individual files anywhere under `root`, largest first.
    pub largest_files: Vec<Entry>,
}

/// Walks `root` and reports where the space goes.
///
/// Sizes are apparent file sizes (not on-disk allocation). Symbolic links are
/// not followed. Hard links to the same file are counted once per link.
pub fn scan(root: &Path, top_files: usize) -> Result<ScanResult> {
    let meta = fs::metadata(root).map_err(|source| Error::Io {
        path: root.to_path_buf(),
        source,
    })?;
    if !meta.is_dir() {
        return Err(Error::NotADirectory(root.to_path_buf()));
    }

    let mut children: HashMap<OsString, (u64, bool)> = HashMap::new();
    let mut heap: BinaryHeap<Reverse<(u64, PathBuf)>> = BinaryHeap::new();
    let (mut total_bytes, mut file_count, mut dir_count, mut skipped) = (0u64, 0u64, 0u64, 0u64);

    for entry in WalkDir::new(root).min_depth(1).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let Some(child_key) = entry
            .path()
            .strip_prefix(root)
            .ok()
            .and_then(|rel| rel.components().next())
            .map(|c| c.as_os_str().to_owned())
        else {
            continue;
        };

        let file_type = entry.file_type();
        if file_type.is_dir() {
            dir_count += 1;
            if entry.depth() == 1 {
                children.entry(child_key).or_insert((0, true));
            }
        } else if file_type.is_file() {
            let size = match entry.metadata() {
                Ok(m) => m.len(),
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            file_count += 1;
            total_bytes += size;
            children
                .entry(child_key)
                .or_insert((0, entry.depth() > 1))
                .0 += size;

            if top_files > 0 {
                heap.push(Reverse((size, entry.path().to_path_buf())));
                if heap.len() > top_files {
                    heap.pop();
                }
            }
        }
    }

    let mut children: Vec<Entry> = children
        .into_iter()
        .map(|(name, (size, is_dir))| Entry {
            path: root.join(&name).to_string_lossy().into_owned(),
            name: name.to_string_lossy().into_owned(),
            size,
            is_dir,
        })
        .collect();
    children.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));

    let largest_files = heap
        .into_sorted_vec()
        .into_iter()
        .map(|Reverse((size, path))| Entry {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.to_string_lossy().into_owned(),
            size,
            is_dir: false,
        })
        .collect();

    Ok(ScanResult {
        root: root.to_string_lossy().into_owned(),
        total_bytes,
        file_count,
        dir_count,
        skipped,
        children,
        largest_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::File::create(path)
            .unwrap()
            .write_all(&vec![0u8; bytes])
            .unwrap();
    }

    #[test]
    fn scan_aggregates_sizes_per_child() {
        let dir = tempfile::tempdir().unwrap();
        write_file(&dir.path().join("big/a.bin"), 5000);
        write_file(&dir.path().join("big/nested/b.bin"), 3000);
        write_file(&dir.path().join("small/c.bin"), 100);
        write_file(&dir.path().join("top.bin"), 700);
        fs::create_dir(dir.path().join("empty")).unwrap();

        let result = scan(dir.path(), 2).unwrap();

        assert_eq!(result.total_bytes, 8800);
        assert_eq!(result.file_count, 4);
        assert_eq!(result.dir_count, 4); // big, big/nested, small, empty

        let summary: Vec<(&str, u64, bool)> = result
            .children
            .iter()
            .map(|e| (e.name.as_str(), e.size, e.is_dir))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("big", 8000, true),
                ("top.bin", 700, false),
                ("small", 100, true),
                ("empty", 0, true),
            ]
        );

        let largest: Vec<(&str, u64)> = result
            .largest_files
            .iter()
            .map(|e| (e.name.as_str(), e.size))
            .collect();
        assert_eq!(largest, vec![("a.bin", 5000), ("b.bin", 3000)]);
    }

    #[test]
    fn scan_rejects_files_and_missing_paths() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f.txt");
        write_file(&file, 1);
        assert!(matches!(scan(&file, 1), Err(Error::NotADirectory(_))));
        assert!(matches!(
            scan(&dir.path().join("missing"), 1),
            Err(Error::Io { .. })
        ));
    }

    #[test]
    fn disk_usage_math() {
        let disk = DiskInfo {
            name: "C:".into(),
            mount_point: "C:\\".into(),
            file_system: "NTFS".into(),
            kind: DriveKind::Ssd,
            total: 1000,
            available: 250,
            removable: false,
        };
        assert_eq!(disk.used(), 750);
        assert_eq!(disk.used_percent(), 75.0);
    }

    #[test]
    fn list_disks_succeeds() {
        // Containers may expose no volumes, so only check that the call works.
        assert!(list_disks().is_ok());
    }
}
