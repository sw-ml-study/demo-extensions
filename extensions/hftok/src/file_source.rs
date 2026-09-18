//! Reading a `tokenizer.json` from a confined path and reporting its summary.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use mlpl_extension_sdk::{OwnedError, Value};

use crate::tokenizer_file::{CONTROL_TOKENS, TokenizerFile};

/// Largest tokenizer file this extension will read (64 MiB). Real
/// vocabularies are a few megabytes; the cap bounds a hostile input.
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Validates one `tokenizer.json` beneath an explicit root and reports its
/// vocabulary size, control-token ids, and pre-tokenization pattern.
///
/// # Errors
///
/// Returns an invalid-argument error for a malformed record or an unconfined
/// path, and an extension error when the file cannot be read or describes a
/// tokenizer this extension cannot encode correctly.
pub fn validate_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let request = LoadRequest::parse(arguments)?;
    let file = request.read()?;
    let parsed = TokenizerFile::parse(&file).map_err(OwnedError::extension)?;
    let control = parsed.control_ids();
    let mut fields = BTreeMap::from([
        (
            "vocabulary_size".to_owned(),
            Value::I64(parsed.vocabulary_size()),
        ),
        (
            "merge_count".to_owned(),
            Value::I64(i64::try_from(parsed.merges.len()).unwrap_or(i64::MAX)),
        ),
        (
            "added_token_count".to_owned(),
            Value::I64(i64::try_from(parsed.added_tokens.len()).unwrap_or(i64::MAX)),
        ),
        ("pattern".to_owned(), Value::String(parsed.pattern.clone())),
    ]);
    for (name, id) in control_field_names().iter().zip(control) {
        fields.insert((*name).to_owned(), Value::I64(id));
    }
    Ok(Value::Record(fields))
}

/// Field names reporting each control token's id, in `CONTROL_TOKENS` order.
fn control_field_names() -> [&'static str; CONTROL_TOKENS.len()] {
    [
        "end_of_text_id",
        "turn_start_id",
        "turn_end_id",
        "think_start_id",
        "think_end_id",
    ]
}

pub(crate) struct LoadRequest {
    root: PathBuf,
    relative_path: PathBuf,
}

impl LoadRequest {
    pub(crate) fn parse(arguments: &[Value]) -> Result<Self, OwnedError> {
        let fields = match arguments {
            [Value::Record(fields)] => fields,
            [_] => return Err(invalid("load argument must be a record")),
            _ => return Err(invalid("load expects exactly one argument")),
        };
        let expected = ["path", "root"];
        let actual = fields.keys().map(String::as_str).collect::<Vec<_>>();
        if actual != expected {
            return Err(invalid("load fields do not match the V1 contract"));
        }
        let root = PathBuf::from(string_field(fields, "root")?);
        if !root.is_absolute() {
            return Err(invalid("root must be an absolute directory"));
        }
        let relative_path = confined_relative_path(string_field(fields, "path")?)?.to_path_buf();
        Ok(Self {
            root,
            relative_path,
        })
    }

    pub(crate) fn confine(&self) -> Result<PathBuf, OwnedError> {
        let root = self
            .root
            .canonicalize()
            .map_err(|_| invalid("root must be an existing directory"))?;
        if !root.is_dir() {
            return Err(invalid("root must be an existing directory"));
        }
        let canonical = root
            .join(&self.relative_path)
            .canonicalize()
            .map_err(|_| invalid("path must name an existing file beneath the root"))?;
        if !canonical.starts_with(&root) {
            return Err(invalid("path escapes the configured root"));
        }
        if !canonical.is_file() {
            return Err(invalid("path does not name a regular file"));
        }
        Ok(canonical)
    }

    pub(crate) fn read(&self) -> Result<String, OwnedError> {
        let path = self.confine()?;
        let metadata = fs::metadata(&path).map_err(|error| {
            OwnedError::extension(format!("cannot read file metadata: {error}"))
        })?;
        if metadata.len() > MAX_FILE_BYTES {
            return Err(OwnedError::extension(format!(
                "tokenizer.json exceeds the {MAX_FILE_BYTES} byte limit"
            )));
        }
        fs::read_to_string(&path)
            .map_err(|error| OwnedError::extension(format!("cannot read tokenizer.json: {error}")))
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
