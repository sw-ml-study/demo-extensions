//! Deterministic, bounded discovery of model containers for generic MLPL apps.

use std::path::Path;

/// Discovers Safetensors and GGUF files beneath `root` without following symlinks.
///
/// # Errors
///
/// Returns an error when the root or a contained directory cannot be read or
/// when a result cannot be represented relative to the confined root.
pub fn discover_model_paths(root: &Path, limit: usize) -> Result<Vec<String>, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let mut pending = vec![root.clone()];
    let mut paths = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = std::fs::read_dir(&directory)
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries.into_iter().rev() {
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                pending.push(entry.path());
                continue;
            }
            let extension = entry
                .path()
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if kind.is_file() && matches!(extension.as_str(), "safetensors" | "gguf") {
                let relative = entry
                    .path()
                    .strip_prefix(&root)
                    .map_err(|error| error.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                paths.push(relative);
            }
        }
    }
    paths.sort();
    paths.truncate(limit);
    Ok(paths)
}
