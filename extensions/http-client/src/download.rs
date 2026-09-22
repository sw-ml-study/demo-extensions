//! Filesystem and network effects for bounded large-artifact downloads:
//! confinement under an explicit root, verified reuse, streamed hashing into
//! a temporary file, and atomic publication.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use mlpl_extension_sdk::{OwnedError, Value};
use sha2::{Digest, Sha256};
use tempfile::{Builder, NamedTempFile};

use crate::download_plan::{DOWNLOAD_MAX_REDIRECTS, DownloadPlan};
use crate::fields::{invalid, record};

/// Downloads one artifact of declared length and SHA-256 digest into a
/// confined path beneath `root`, streaming bounded chunks through a temporary
/// file and renaming it into place only after verification.
///
/// An existing target is reused, without network access, when its size and
/// digest already verify; otherwise it is replaced by the verified transfer.
/// Every failure removes the temporary file and leaves no partial output.
///
/// # Errors
///
/// Returns an invalid-argument error for malformed or unconfined requests and
/// an extension error for status, transport, length, digest, or filesystem
/// failures.
pub fn download_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let plan = DownloadPlan::parse(arguments)?;
    let target = confine(&plan)?;
    if let Some(sha256) = verified_existing(&plan, &target)? {
        return Ok(outcome(&plan, &target, sha256, true));
    }
    let sha256 = fetch_into(&plan, &target)?;
    Ok(outcome(&plan, &target, sha256, false))
}

fn confine(plan: &DownloadPlan) -> Result<PathBuf, OwnedError> {
    let root = plan
        .root
        .canonicalize()
        .map_err(|_| invalid("root must be an existing directory"))?;
    if !root.is_dir() {
        return Err(invalid("root must be an existing directory"));
    }
    let target = root.join(&plan.relative_path);
    let parent = target
        .parent()
        .ok_or_else(|| invalid("path has no parent directory"))?;
    fs::create_dir_all(parent).map_err(|error| {
        OwnedError::extension(format!("cannot create parent directory: {error}"))
    })?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|_| invalid("path parent cannot be resolved"))?;
    if !canonical_parent.starts_with(&root) {
        return Err(invalid("path escapes the configured root"));
    }
    if target.exists() {
        let canonical = target
            .canonicalize()
            .map_err(|_| invalid("path cannot be resolved"))?;
        if !canonical.starts_with(&root) {
            return Err(invalid("path escapes the configured root"));
        }
        if canonical.is_dir() {
            return Err(invalid("path names an existing directory"));
        }
    }
    Ok(target)
}

fn verified_existing(plan: &DownloadPlan, target: &Path) -> Result<Option<String>, OwnedError> {
    let Ok(metadata) = fs::metadata(target) else {
        return Ok(None);
    };
    if !metadata.is_file() || metadata.len() != plan.expected_bytes {
        return Ok(None);
    }
    let file = File::open(target)
        .map_err(|error| OwnedError::extension(format!("cannot read existing file: {error}")))?;
    let (bytes, sha256) = hash_reader(file, plan.chunk_bytes, plan.expected_bytes, |_| Ok(()))?;
    if bytes == plan.expected_bytes && sha256 == plan.sha256 {
        Ok(Some(sha256))
    } else {
        Ok(None)
    }
}

fn fetch_into(plan: &DownloadPlan, target: &Path) -> Result<String, OwnedError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(plan.timeout)
        .redirects(DOWNLOAD_MAX_REDIRECTS)
        .build();
    let response = match agent.get(&plan.url).call() {
        Ok(response) => response,
        Err(ureq::Error::Status(status, _)) => {
            return Err(OwnedError::extension(format!(
                "HTTP status {status} is not a successful download"
            )));
        }
        Err(ureq::Error::Transport(error)) => {
            return Err(OwnedError::extension(format!(
                "HTTP transport failure: {}",
                error.kind()
            )));
        }
    };
    // A redirect past the budget is returned as a successful call, so the
    // status is checked here rather than trusting the transport's result.
    if !(200..300).contains(&response.status()) {
        return Err(OwnedError::extension(format!(
            "HTTP status {} is not a successful download",
            response.status()
        )));
    }
    plan.check_declared_length(response.header("content-length"))?;
    let mut temporary = temporary_file(target)?;
    let (bytes, sha256) = hash_reader(
        response.into_reader(),
        plan.chunk_bytes,
        plan.expected_bytes,
        |chunk| {
            temporary.write_all(chunk).map_err(|error| {
                OwnedError::extension(format!("cannot write temporary file: {error}"))
            })
        },
    )?;
    plan.verify_transfer(bytes, &sha256)?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| OwnedError::extension(format!("cannot flush temporary file: {error}")))?;
    temporary.persist(target).map_err(|error| {
        OwnedError::extension(format!("cannot publish download: {}", error.error))
    })?;
    Ok(sha256)
}

fn temporary_file(target: &Path) -> Result<NamedTempFile, OwnedError> {
    let parent = target
        .parent()
        .ok_or_else(|| invalid("path has no parent directory"))?;
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid("path has no file name"))?;
    Builder::new()
        .prefix(&format!(".{name}.part-"))
        .tempfile_in(parent)
        .map_err(|error| OwnedError::extension(format!("cannot create temporary file: {error}")))
}

/// Streams `reader` in bounded chunks through `sink` while hashing, stopping
/// as soon as more than `limit` bytes arrive. A read failure before `limit`
/// bytes is reported as a truncated transfer so callers see one vocabulary.
fn hash_reader(
    mut reader: impl Read,
    chunk_bytes: usize,
    limit: u64,
    mut sink: impl FnMut(&[u8]) -> Result<(), OwnedError>,
) -> Result<(u64, String), OwnedError> {
    let mut buffer = vec![0_u8; chunk_bytes];
    let mut stream = Sha256::new();
    let mut total = 0_u64;
    loop {
        let size = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(size) => size,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(OwnedError::extension(format!(
                    "transfer truncated after {total} of {limit} bytes: {error}"
                )));
            }
        };
        total = total.saturating_add(u64::try_from(size).unwrap_or(u64::MAX));
        if total > limit {
            return Err(OwnedError::extension(format!(
                "transfer exceeds expected_bytes {limit}"
            )));
        }
        stream.update(&buffer[..size]);
        sink(&buffer[..size])?;
    }
    Ok((total, format!("{:x}", stream.finalize())))
}

fn outcome(plan: &DownloadPlan, target: &Path, sha256: String, reused: bool) -> Value {
    record([
        ("path", Value::String(target.to_string_lossy().into_owned())),
        (
            "bytes",
            Value::I64(i64::try_from(plan.expected_bytes).unwrap_or(i64::MAX)),
        ),
        ("sha256", Value::String(sha256)),
        ("reused", Value::Bool(reused)),
    ])
}
