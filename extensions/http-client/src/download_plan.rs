//! Pure validation and verification rules for bounded large-artifact
//! downloads. Nothing here touches the network or the filesystem.

use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use mlpl_extension_sdk::{OwnedError, Value};
use url::Url;

use crate::fields::{bounded_i64, invalid, require_exact_fields, single_record, string_field};

/// Largest transfer the extension will plan (8 GiB).
pub(crate) const MAX_DOWNLOAD_BYTES: i64 = 8 * 1024 * 1024 * 1024;
/// Longest overall deadline for one transfer (one hour).
pub(crate) const MAX_DOWNLOAD_TIMEOUT_MS: i64 = 3_600_000;
/// Largest single read buffer (16 MiB).
pub(crate) const MAX_CHUNK_BYTES: i64 = 16 * 1024 * 1024;
/// Fixed redirect budget, matching the bounded `get` surface.
pub(crate) const DOWNLOAD_MAX_REDIRECTS: u32 = 3;

/// A fully validated download request. Construction never performs I/O.
pub(crate) struct DownloadPlan {
    pub(crate) url: String,
    pub(crate) expected_bytes: u64,
    /// Lowercase hexadecimal SHA-256 digest of the expected content.
    pub(crate) sha256: String,
    pub(crate) root: PathBuf,
    pub(crate) relative_path: PathBuf,
    pub(crate) chunk_bytes: usize,
    pub(crate) timeout: Duration,
}

impl DownloadPlan {
    pub(crate) fn parse(arguments: &[Value]) -> Result<Self, OwnedError> {
        let fields = single_record(arguments, "download")?;
        require_exact_fields(
            fields,
            &[
                "url",
                "expected_bytes",
                "sha256",
                "root",
                "path",
                "chunk_bytes",
                "timeout_ms",
            ],
            "download",
        )?;
        let url = string_field(fields, "url")?.to_owned();
        let parsed = Url::parse(&url).map_err(|_| invalid("url must be absolute"))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(invalid("url must be an absolute HTTP or HTTPS URL"));
        }
        let expected_bytes = bounded_i64(fields, "expected_bytes", 1, MAX_DOWNLOAD_BYTES)?;
        let sha256 = parse_digest(string_field(fields, "sha256")?)?;
        let root = PathBuf::from(string_field(fields, "root")?);
        if !root.is_absolute() {
            return Err(invalid("root must be an absolute directory"));
        }
        let relative_path = confined_relative_path(string_field(fields, "path")?)?.to_path_buf();
        let chunk_bytes = bounded_i64(fields, "chunk_bytes", 1, MAX_CHUNK_BYTES)?;
        let timeout_ms = bounded_i64(fields, "timeout_ms", 1, MAX_DOWNLOAD_TIMEOUT_MS)?;
        Ok(Self {
            url,
            expected_bytes: u64::try_from(expected_bytes)
                .map_err(|_| invalid("expected_bytes is invalid"))?,
            sha256,
            root,
            relative_path,
            chunk_bytes: usize::try_from(chunk_bytes)
                .map_err(|_| invalid("chunk_bytes is invalid"))?,
            timeout: Duration::from_millis(
                u64::try_from(timeout_ms).map_err(|_| invalid("timeout_ms is invalid"))?,
            ),
        })
    }

    /// Rejects a declared `Content-Length` that disagrees with the plan before
    /// any byte is written. A missing header is allowed; the streamed count is
    /// still verified afterwards.
    pub(crate) fn check_declared_length(
        &self,
        content_length: Option<&str>,
    ) -> Result<(), OwnedError> {
        let Some(declared) = content_length else {
            return Ok(());
        };
        let declared = declared
            .trim()
            .parse::<u64>()
            .map_err(|_| OwnedError::extension("HTTP content-length header is malformed"))?;
        if declared == self.expected_bytes {
            Ok(())
        } else {
            Err(OwnedError::extension(format!(
                "HTTP content-length {declared} does not match expected_bytes {}",
                self.expected_bytes
            )))
        }
    }

    /// Verifies a completed transfer against the declared length and digest.
    pub(crate) fn verify_transfer(
        &self,
        actual_bytes: u64,
        actual_sha256: &str,
    ) -> Result<(), OwnedError> {
        if actual_bytes < self.expected_bytes {
            return Err(OwnedError::extension(format!(
                "transfer truncated after {actual_bytes} of {} bytes",
                self.expected_bytes
            )));
        }
        if actual_bytes > self.expected_bytes {
            return Err(OwnedError::extension(format!(
                "transfer exceeds expected_bytes {}",
                self.expected_bytes
            )));
        }
        if actual_sha256 == self.sha256 {
            Ok(())
        } else {
            Err(OwnedError::extension(
                "transfer sha256 does not match the expected digest",
            ))
        }
    }
}

fn parse_digest(value: &str) -> Result<String, OwnedError> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value.to_ascii_lowercase())
    } else {
        Err(invalid("sha256 must be 64 hexadecimal characters"))
    }
}

fn confined_relative_path(value: &str) -> Result<&Path, OwnedError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(invalid("path must be a confined relative path"));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(expected_bytes: u64) -> DownloadPlan {
        DownloadPlan {
            url: "http://127.0.0.1/".into(),
            expected_bytes,
            sha256: "ab".repeat(32),
            root: PathBuf::from("/"),
            relative_path: PathBuf::from("file"),
            chunk_bytes: 1,
            timeout: Duration::from_millis(1),
        }
    }

    #[test]
    fn declared_length_must_match_when_present() {
        assert!(plan(10).check_declared_length(None).is_ok());
        assert!(plan(10).check_declared_length(Some(" 10 ")).is_ok());
        assert!(plan(10).check_declared_length(Some("11")).is_err());
        assert!(plan(10).check_declared_length(Some("ten")).is_err());
    }

    #[test]
    fn transfer_verification_orders_length_before_digest() {
        let digest = "ab".repeat(32);
        assert!(plan(10).verify_transfer(10, &digest).is_ok());
        assert!(
            plan(10)
                .verify_transfer(9, &digest)
                .unwrap_err()
                .message()
                .contains("truncated")
        );
        assert!(
            plan(10)
                .verify_transfer(11, &digest)
                .unwrap_err()
                .message()
                .contains("exceeds")
        );
        assert!(
            plan(10)
                .verify_transfer(10, &"cd".repeat(32))
                .unwrap_err()
                .message()
                .contains("sha256")
        );
    }
}
