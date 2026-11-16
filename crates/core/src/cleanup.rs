//! Finds files that are safe candidates for cleanup.
//!
//! This module only *plans*: it never deletes anything. Deleting (to the
//! Recycle Bin, after confirmation) arrives in a later milestone.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use walkdir::WalkDir;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupCategory {
    /// The current user's temporary files (`%TEMP%` on Windows).
    UserTemp,
    /// System-wide temporary files (`C:\Windows\Temp`; may need administrator rights).
    SystemTemp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate {
    pub path: String,
    pub size: u64,
    pub category: CleanupCategory,
    pub age_days: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CleanupPlan {
    pub candidates: Vec<Candidate>,
    pub total_bytes: u64,
    /// Entries that could not be read (in use, permissions).
    pub skipped: u64,
}

/// Plans cleanup of the given categories, keeping files newer than `min_age`.
pub fn plan(categories: &[CleanupCategory], min_age: Duration) -> Result<CleanupPlan> {
    let roots: Vec<(CleanupCategory, PathBuf)> = categories
        .iter()
        .flat_map(|&category| {
            default_roots(category)
                .into_iter()
                .map(move |root| (category, root))
        })
        .collect();
    plan_for_roots(&roots, min_age)
}

/// Like [`plan`], with explicit roots. Every root must pass [`validate_root`].
pub fn plan_for_roots(
    roots: &[(CleanupCategory, PathBuf)],
    min_age: Duration,
) -> Result<CleanupPlan> {
    let now = SystemTime::now();
    let mut candidates = Vec::new();
    let mut skipped = 0u64;

    for (category, root) in roots {
        if !root.is_dir() {
            continue;
        }
        validate_root(root)?;

        for entry in WalkDir::new(root).min_depth(1).follow_links(false) {
            let Ok(entry) = entry else {
                skipped += 1;
                continue;
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                skipped += 1;
                continue;
            };
            let Ok(modified) = meta.modified() else {
                skipped += 1;
                continue;
            };
            let age = now.duration_since(modified).unwrap_or_default();
            if age >= min_age {
                candidates.push(Candidate {
                    path: entry.path().to_string_lossy().into_owned(),
                    size: meta.len(),
                    category: *category,
                    age_days: age.as_secs() / 86_400,
                });
            }
        }
    }

    candidates.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
    let total_bytes = candidates.iter().map(|c| c.size).sum();
    Ok(CleanupPlan {
        candidates,
        total_bytes,
        skipped,
    })
}

/// Safety guard: only directories that look like temp folders may be planned.
pub fn validate_root(root: &Path) -> Result<()> {
    let depth = root.components().count();
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if depth < 2 || !(name.contains("temp") || name.contains("tmp")) {
        return Err(Error::UnsafeRoot(root.to_path_buf()));
    }
    Ok(())
}

fn default_roots(category: CleanupCategory) -> Vec<PathBuf> {
    match category {
        CleanupCategory::UserTemp => vec![std::env::temp_dir()],
        CleanupCategory::SystemTemp => {
            if cfg!(windows) {
                std::env::var_os("SystemRoot")
                    .map(|root| PathBuf::from(root).join("Temp"))
                    .into_iter()
                    .collect()
            } else {
                vec![PathBuf::from("/var/tmp")]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn make_file(path: &Path, bytes: usize, age: Duration) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = File::create(path).unwrap();
        file.write_all(&vec![0u8; bytes]).unwrap();
        file.set_modified(SystemTime::now() - age).unwrap();
    }

    const DAY: Duration = Duration::from_secs(86_400);

    #[test]
    fn plan_keeps_recent_files_and_sorts_by_size() {
        let dir = tempfile::tempdir().unwrap();
        make_file(&dir.path().join("old_big.dat"), 900, DAY * 10);
        make_file(&dir.path().join("sub/old_small.dat"), 100, DAY * 8);
        make_file(
            &dir.path().join("recent.dat"),
            5000,
            Duration::from_secs(60),
        );

        let roots = vec![(CleanupCategory::UserTemp, dir.path().to_path_buf())];
        let plan = plan_for_roots(&roots, DAY * 7).unwrap();

        let sizes: Vec<u64> = plan.candidates.iter().map(|c| c.size).collect();
        assert_eq!(sizes, vec![900, 100]);
        assert_eq!(plan.total_bytes, 1000);
        assert_eq!(plan.candidates[0].age_days, 10);
    }

    #[test]
    fn zero_min_age_includes_everything() {
        let dir = tempfile::tempdir().unwrap();
        make_file(&dir.path().join("a"), 10, Duration::ZERO);
        make_file(&dir.path().join("b"), 20, Duration::ZERO);
        let roots = vec![(CleanupCategory::UserTemp, dir.path().to_path_buf())];
        let plan = plan_for_roots(&roots, Duration::ZERO).unwrap();
        assert_eq!(plan.candidates.len(), 2);
    }

    #[test]
    fn rejects_roots_that_are_not_temp_folders() {
        assert!(validate_root(Path::new("/")).is_err());
        assert!(validate_root(Path::new("/home/user/Documents")).is_err());
        assert!(validate_root(Path::new("C:\\")).is_err());
        assert!(validate_root(Path::new("/tmp")).is_ok());
        assert!(validate_root(Path::new("/var/tmp")).is_ok());

        let dir = tempfile::tempdir().unwrap();
        let docs = dir.path().join("Documents");
        fs::create_dir(&docs).unwrap();
        let roots = vec![(CleanupCategory::UserTemp, docs)];
        assert!(matches!(
            plan_for_roots(&roots, Duration::ZERO),
            Err(Error::UnsafeRoot(_))
        ));
    }
}
