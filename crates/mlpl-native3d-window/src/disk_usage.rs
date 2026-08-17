//! Bounded, metadata-only filesystem snapshots for MLPL applications.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotBudgets {
    pub max_entries: usize,
    pub max_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotEntry {
    pub path: String,
    pub kind: &'static str,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskUsageSnapshot {
    pub root: String,
    pub entries: Vec<SnapshotEntry>,
    pub excluded_entries: u64,
    pub excluded_bytes: u64,
    pub inaccessible_entries: u64,
    pub budgets: SnapshotBudgets,
}

impl DiskUsageSnapshot {
    /// Serializes this bounded data-only snapshot as one MLPL binding.
    #[must_use]
    pub fn to_mlpl_binding(&self) -> String {
        let paths = string_list(self.entries.iter().map(|entry| entry.path.as_str()));
        let kinds = string_list(self.entries.iter().map(|entry| entry.kind));
        let bytes = self
            .entries
            .iter()
            .map(|entry| entry.bytes.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "disk_usage_snapshot={{root:\"{}\",paths:{paths},kinds:{kinds},bytes:[{bytes}],excluded_entries:{},excluded_bytes:{},inaccessible_entries:{},budgets:{{max_entries:{},max_depth:{}}}}};",
            escape_mlpl(&self.root),
            self.excluded_entries,
            self.excluded_bytes,
            self.inaccessible_entries,
            self.budgets.max_entries,
            self.budgets.max_depth
        )
    }
}

fn string_list<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let values = values
        .map(|value| format!("\"{}\"", escape_mlpl(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn escape_mlpl(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Captures one deterministic, metadata-only snapshot under `root`.
///
/// Symlinks are excluded and never followed. `excluded_bytes` is a known-byte
/// lower bound; the collector never walks beyond its limits merely to estimate
/// excluded content.
///
/// # Errors
///
/// Returns an error if the root cannot be canonicalized or is not a directory.
pub fn capture_snapshot(
    root: &Path,
    budgets: SnapshotBudgets,
) -> Result<DiskUsageSnapshot, std::io::Error> {
    let root = root.canonicalize()?;
    if !root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "snapshot root is not a directory",
        ));
    }
    let mut snapshot = DiskUsageSnapshot {
        root: root.display().to_string(),
        entries: Vec::new(),
        excluded_entries: 0,
        excluded_bytes: 0,
        inaccessible_entries: 0,
        budgets,
    };
    visit_breadth_first(&root, &mut snapshot);
    Ok(snapshot)
}

fn visit_breadth_first(root: &Path, snapshot: &mut DiskUsageSnapshot) {
    let mut pending = VecDeque::from([(PathBuf::new(), 0_usize)]);
    while let Some((relative, depth)) = pending.pop_front() {
        let Ok(read) = std::fs::read_dir(root.join(&relative)) else {
            snapshot.inaccessible_entries += 1;
            continue;
        };
        let mut paths: Vec<PathBuf> = read
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        paths.sort();
        for path in paths {
            if snapshot.entries.len() >= snapshot.budgets.max_entries {
                snapshot.excluded_entries += 1;
                continue;
            }
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                snapshot.inaccessible_entries += 1;
                continue;
            };
            let Ok(relative_path) = path.strip_prefix(root) else {
                snapshot.inaccessible_entries += 1;
                continue;
            };
            let relative_text = relative_path.to_string_lossy().replace('\\', "/");
            if metadata.file_type().is_symlink() {
                snapshot.excluded_entries += 1;
            } else if metadata.is_dir() {
                snapshot.entries.push(SnapshotEntry {
                    path: relative_text,
                    kind: "dir",
                    bytes: 0,
                });
                if depth < snapshot.budgets.max_depth {
                    pending.push_back((relative_path.to_path_buf(), depth + 1));
                } else {
                    snapshot.excluded_entries += 1;
                }
            } else if metadata.is_file() {
                snapshot.entries.push(SnapshotEntry {
                    path: relative_text,
                    kind: "file",
                    bytes: metadata.len(),
                });
            } else {
                snapshot.excluded_entries += 1;
                snapshot.excluded_bytes = snapshot.excluded_bytes.saturating_add(metadata.len());
            }
        }
    }
}
