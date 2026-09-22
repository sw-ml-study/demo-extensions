//! Streaming SHA-256 over in-memory bytes and confined files.
//!
//! This is a native primitive because SHA-256 is 64 rounds of 32-bit modular
//! addition, rotation, and exclusive-or per 64-byte block. A gigabyte input is
//! roughly 16 million blocks, and MLPL has no native 32-bit integer type to
//! express the rounds in, so the algorithm is not merely slow in the
//! interpreter but impractical. Nothing here is application-specific.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use mlpl_extension_sdk::{OwnedError, Value};
use sha2::{Digest, Sha256};

/// Largest single read buffer (16 MiB).
const MAX_CHUNK_BYTES: i64 = 16 * 1024 * 1024;
/// Default read buffer when a caller does not choose one (1 MiB).
pub const DEFAULT_CHUNK_BYTES: usize = 1024 * 1024;

/// Hashes one in-memory byte string.
///
/// # Errors
///
/// Returns an invalid-argument error unless called with exactly one bytes
/// value.
pub fn sha256_bytes_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let bytes = match arguments {
        [Value::Bytes(bytes)] => bytes,
        [_] => return Err(invalid("sha256_bytes argument must be bytes")),
        _ => return Err(invalid("sha256_bytes expects exactly one argument")),
    };
    let (_, digest) = hash_reader(bytes.as_slice(), DEFAULT_CHUNK_BYTES)?;
    Ok(Value::String(digest))
}

/// Hashes an existing file beneath an explicit root, streaming it in bounded
/// chunks so the whole artifact is never resident.
///
/// # Errors
///
/// Returns an invalid-argument error for a malformed record, a non-canonical
/// root, or an unconfined path, and an extension error when the file cannot be
/// opened or read.
pub fn sha256_file_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let request = FileRequest::parse(arguments)?;
    let path = request.confine()?;
    let file = File::open(&path)
        .map_err(|error| OwnedError::extension(format!("cannot open file: {error}")))?;
    let metadata = file
        .metadata()
        .map_err(|error| OwnedError::extension(format!("cannot read file metadata: {error}")))?;
    if !metadata.is_file() {
        return Err(invalid("path does not name a regular file"));
    }
    let (bytes, digest) = hash_reader(file, request.chunk_bytes)?;
    Ok(Value::Record(
        [
            ("sha256".to_owned(), Value::String(digest)),
            (
                "bytes".to_owned(),
                Value::I64(i64::try_from(bytes).unwrap_or(i64::MAX)),
            ),
        ]
        .into_iter()
        .collect(),
    ))
}

struct FileRequest {
    root: PathBuf,
    relative_path: PathBuf,
    chunk_bytes: usize,
}

impl FileRequest {
    fn parse(arguments: &[Value]) -> Result<Self, OwnedError> {
        let fields = match arguments {
            [Value::Record(fields)] => fields,
            [_] => return Err(invalid("sha256_file argument must be a record")),
            _ => return Err(invalid("sha256_file expects exactly one argument")),
        };
        let expected = BTreeSet::from(["chunk_bytes", "path", "root"]);
        let actual = fields.keys().map(String::as_str).collect::<BTreeSet<_>>();
        if actual != expected {
            return Err(invalid("sha256_file fields do not match the V1 contract"));
        }
        let root = PathBuf::from(string_field(fields, "root")?);
        if !root.is_absolute() {
            return Err(invalid("root must be an absolute directory"));
        }
        let relative_path = confined_relative_path(string_field(fields, "path")?)?.to_path_buf();
        let chunk_bytes = match fields.get("chunk_bytes") {
            Some(Value::I64(value)) if (1..=MAX_CHUNK_BYTES).contains(value) => *value,
            Some(Value::I64(_)) => {
                return Err(invalid(format!(
                    "chunk_bytes must be between 1 and {MAX_CHUNK_BYTES}"
                )));
            }
            Some(_) => return Err(invalid("chunk_bytes must be i64")),
            None => return Err(invalid("missing field: chunk_bytes")),
        };
        Ok(Self {
            root,
            relative_path,
            chunk_bytes: usize::try_from(chunk_bytes)
                .map_err(|_| invalid("chunk_bytes is invalid"))?,
        })
    }

    fn confine(&self) -> Result<PathBuf, OwnedError> {
        let root = self
            .root
            .canonicalize()
            .map_err(|_| invalid("root must be an existing directory"))?;
        if !root.is_dir() {
            return Err(invalid("root must be an existing directory"));
        }
        let target = root.join(&self.relative_path);
        let canonical = target
            .canonicalize()
            .map_err(|_| invalid("path must name an existing file beneath the root"))?;
        if !canonical.starts_with(&root) {
            return Err(invalid("path escapes the configured root"));
        }
        Ok(canonical)
    }
}

/// Incremental SHA-256 over a byte stream.
///
/// Wraps `sha2` for the digest extension's byte and file surfaces. Other
/// extensions use `sha2` directly so linking their libraries does not pull in
/// this extension's exported C ABI entry point.
#[derive(Default)]
pub struct Sha256Stream {
    hasher: Sha256,
    bytes: u64,
}

impl Sha256Stream {
    /// Starts an empty stream.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one chunk.
    pub fn update(&mut self, chunk: &[u8]) {
        self.bytes = self
            .bytes
            .saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
        self.hasher.update(chunk);
    }

    /// Returns the bytes consumed so far.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Consumes the stream and renders the digest as lowercase hexadecimal.
    #[must_use]
    pub fn finish(self) -> String {
        let mut text = String::with_capacity(32 * 2);
        for byte in self.hasher.finalize() {
            let _ = write!(text, "{byte:02x}");
        }
        text
    }
}

/// Streams `reader` in bounded chunks, returning the byte count and the
/// lowercase hexadecimal digest. Shared by the bytes and file surfaces.
///
/// # Errors
///
/// Returns an extension error when the reader fails.
pub fn hash_reader(mut reader: impl Read, chunk_bytes: usize) -> Result<(u64, String), OwnedError> {
    let mut buffer = vec![0_u8; chunk_bytes.max(1)];
    let mut stream = Sha256Stream::new();
    loop {
        let size = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(size) => size,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(OwnedError::extension(format!("read failed: {error}")));
            }
        };
        stream.update(&buffer[..size]);
    }
    Ok((stream.bytes(), stream.finish()))
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

fn string_field<'a>(
    fields: &'a BTreeMap<String, Value>,
    name: &str,
) -> Result<&'a str, OwnedError> {
    match fields.get(name) {
        Some(Value::String(value)) => Ok(value),
        Some(_) => Err(invalid(format!("{name} must be a string"))),
        None => Err(invalid(format!("missing field: {name}"))),
    }
}

fn invalid(message: impl Into<String>) -> OwnedError {
    OwnedError::invalid_argument(message)
}
