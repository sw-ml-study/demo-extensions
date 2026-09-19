//! The handle-based public surface: `load`, `encode`, `decode`,
//! `token_to_id`, `info`, and `close`. Tokenizers are extension-owned resources reached through typed
//! generational handles, never raw pointers.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mlpl_extension_sdk::{
    DenseArray, HandleError, HandleRegistry, NativeHandle, OwnedError, Value,
};

use crate::byte_level::ByteLevelTokenizer;
use crate::file_source::{LoadRequest, read_confined};
use crate::tokenizer_file::TokenizerFile;

/// Identifies this extension's handles, so another extension's handle is
/// rejected rather than dereferenced.
const EXTENSION_ID: u64 = 0x48_46_54_4f_4b_5f_5f_31;
/// Identifies the tokenizer resource type within this extension.
const TOKENIZER_TYPE_ID: u64 = 0x01;
const MAX_TOKENIZERS: usize = 64;
const GENERATION_LIMIT: u32 = u32::MAX;

thread_local! {
    static TOKENIZERS: RefCell<HandleRegistry> = const {
        RefCell::new(HandleRegistry::with_limits(
            EXTENSION_ID,
            MAX_TOKENIZERS,
            GENERATION_LIMIT,
        ))
    };
}

/// Loads a tokenizer from a confined path and returns an opaque handle.
///
/// # Errors
///
/// Returns an invalid-argument error for a malformed record or an unconfined
/// path, and an extension error when the file cannot be read or describes an
/// unsupported tokenizer.
pub fn load_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let request = LoadRequest::parse(arguments)?;
    insert(&request.read()?)
}

/// Loads a tokenizer named by one absolute path, confining it to the
/// directory that contains it.
///
/// This is the one-argument spelling the work order asks for. The root is not
/// ambient: it is the file's own parent directory, chosen here and stated in
/// `docs/hftok-extension.md`, so the authority stays visible.
///
/// # Errors
///
/// Returns an invalid-argument error unless the argument is one absolute path
/// to an existing regular file, and an extension error when the file cannot be
/// read or describes an unsupported tokenizer.
pub fn load_path_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let path = match arguments {
        [Value::String(path)] => PathBuf::from(path),
        [_] => return Err(invalid("load path must be a string")),
        _ => return Err(invalid("load expects exactly one argument")),
    };
    if !path.is_absolute() {
        return Err(invalid("load path must be absolute"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| invalid("load path has no parent directory"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid("load path has no file name"))?;
    insert(&read_confined(parent, Path::new(name))?)
}

fn insert(source: &str) -> Result<Value, OwnedError> {
    let file = TokenizerFile::parse(source).map_err(OwnedError::extension)?;
    let tokenizer = ByteLevelTokenizer::new(&file).map_err(OwnedError::extension)?;
    TOKENIZERS.with_borrow_mut(|registry| {
        registry
            .insert(TOKENIZER_TYPE_ID, tokenizer)
            .map(Value::Handle)
            .map_err(handle_error)
    })
}

/// Encodes text into an integer array of token ids.
///
/// # Errors
///
/// Returns an invalid-argument error for a bad handle or non-string text, and
/// an extension error when encoding fails.
pub fn encode_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let (handle, text) = match arguments {
        [Value::Handle(handle), Value::String(text)] => (*handle, text),
        [Value::Handle(_), _] => return Err(invalid("encode text must be a string")),
        [_, _] => return Err(invalid("encode expects a tokenizer handle")),
        _ => return Err(invalid("encode expects exactly two arguments")),
    };
    with_tokenizer(handle, |tokenizer| {
        let ids = tokenizer.encode(text).map_err(OwnedError::extension)?;
        let length = ids.len();
        DenseArray::from_i64(vec![length], ids)
            .map(Value::Array)
            .map_err(|error| OwnedError::extension(format!("cannot build id array: {error:?}")))
    })
}

/// Decodes an integer array of ids back into text.
///
/// # Errors
///
/// Returns an invalid-argument error for a bad handle or a non-integer array,
/// and an extension error when an id is unknown or the bytes are not UTF-8.
pub fn decode_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let (handle, array) = match arguments {
        [Value::Handle(handle), Value::Array(array)] => (*handle, array),
        [Value::Handle(_), _] => return Err(invalid("decode ids must be an integer array")),
        [_, _] => return Err(invalid("decode expects a tokenizer handle")),
        _ => return Err(invalid("decode expects exactly two arguments")),
    };
    let view = array.view();
    if view.shape().len() != 1 {
        return Err(invalid("decode ids must be a one-dimensional array"));
    }
    // MLPL's numeric arrays are f64, so ids that this extension produced as
    // i64 arrive back as f64 after a round trip through the interpreter. Both
    // are accepted; a non-integral value is refused rather than truncated.
    let ids = match (view.as_i64(), view.as_f64()) {
        (Ok(values), _) => values.to_vec(),
        (_, Ok(values)) => values
            .iter()
            .map(|value| {
                if value.fract() == 0.0 && value.is_finite() {
                    #[allow(clippy::cast_possible_truncation)]
                    Ok(*value as i64)
                } else {
                    Err(invalid(format!("decode id {value} is not a whole number")))
                }
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(invalid("decode ids must be an integer array")),
    };
    with_tokenizer(handle, |tokenizer| {
        tokenizer
            .decode(&ids)
            .map(Value::String)
            .map_err(OwnedError::extension)
    })
}

/// Reports one token's id.
///
/// # Errors
///
/// Returns an invalid-argument error for a bad handle or non-string token, and
/// an extension error when the token is not in the vocabulary.
pub fn token_to_id_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let (handle, token) = match arguments {
        [Value::Handle(handle), Value::String(token)] => (*handle, token),
        [Value::Handle(_), _] => return Err(invalid("token must be a string")),
        [_, _] => return Err(invalid("token_to_id expects a tokenizer handle")),
        _ => return Err(invalid("token_to_id expects exactly two arguments")),
    };
    with_tokenizer(handle, |tokenizer| {
        tokenizer
            .token_to_id(token)
            .map(Value::I64)
            .ok_or_else(|| OwnedError::extension(format!("token {token} is not in the vocabulary")))
    })
}

/// Reports the vocabulary summary: size, control-token ids, and the file's own
/// pre-tokenization pattern.
///
/// # Errors
///
/// Returns an invalid-argument error for a bad handle.
pub fn info_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments, "info")?;
    with_tokenizer(handle, |tokenizer| {
        let control = tokenizer.control_ids();
        let mut fields = BTreeMap::from([
            (
                "vocabulary_size".to_owned(),
                Value::I64(tokenizer.vocabulary_size()),
            ),
            (
                "pattern".to_owned(),
                Value::String(tokenizer.pattern_source.clone()),
            ),
        ]);
        for (name, id) in CONTROL_FIELD_NAMES.iter().zip(control) {
            fields.insert((*name).to_owned(), Value::I64(id));
        }
        Ok(Value::Record(fields))
    })
}

/// Releases one tokenizer, after which its handle is stale.
///
/// # Errors
///
/// Returns an invalid-argument error for a bad or already-released handle.
pub fn close_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments, "close")?;
    TOKENIZERS.with_borrow_mut(|registry| {
        registry
            .remove::<ByteLevelTokenizer>(handle, TOKENIZER_TYPE_ID)
            .map(|_| Value::Bool(true))
            .map_err(handle_error)
    })
}

/// Field names reporting each control token's id, in `CONTROL_TOKENS` order.
pub(crate) const CONTROL_FIELD_NAMES: [&str; 5] = [
    "end_of_text_id",
    "turn_start_id",
    "turn_end_id",
    "think_start_id",
    "think_end_id",
];

fn with_tokenizer<T>(
    handle: NativeHandle,
    action: impl FnOnce(&ByteLevelTokenizer) -> Result<T, OwnedError>,
) -> Result<T, OwnedError> {
    TOKENIZERS.with_borrow(|registry| {
        let tokenizer = registry
            .get::<ByteLevelTokenizer>(handle, TOKENIZER_TYPE_ID)
            .map_err(handle_error)?;
        action(tokenizer)
    })
}

fn handle_argument(arguments: &[Value], name: &str) -> Result<NativeHandle, OwnedError> {
    match arguments {
        [Value::Handle(handle)] => Ok(*handle),
        [_] => Err(invalid(format!("{name} expects a tokenizer handle"))),
        _ => Err(invalid(format!("{name} expects exactly one argument"))),
    }
}

fn handle_error(error: HandleError) -> OwnedError {
    OwnedError::invalid_argument(match error {
        HandleError::Inactive => "tokenizer registry is inactive",
        HandleError::Exhausted => "tokenizer capacity is exhausted",
        HandleError::WrongExtension => "handle belongs to another extension",
        HandleError::WrongType => "handle has the wrong resource type",
        HandleError::Stale => "tokenizer handle is stale",
    })
}

fn invalid(message: impl Into<String>) -> OwnedError {
    OwnedError::invalid_argument(message)
}
