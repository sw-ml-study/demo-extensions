//! Hugging Face byte-level BPE tokenizer capability for MLPL extensions.
//!
//! This step delivers file parsing, validation, and the `validate` diagnostic.
//! Encoding, decoding, and typed handles follow in later steps.

mod byte_level;
mod file_source;
mod handles;
mod tokenizer_file;

pub use byte_level::ByteLevelTokenizer;
pub use file_source::validate_value;
pub use handles::{
    close_value, decode_value, encode_value, info_value, load_path_value, load_value,
    token_to_id_value,
};
pub use tokenizer_file::{CONTROL_TOKENS, TokenizerFile};

const METADATA: &str = r#"
[[functions]]
name = "validate"
documentation = "Validate a tokenizer.json and report its vocabulary summary."
returns = "record"
[[functions.arguments]]
name = "request"
type = "record"

[[functions]]
name = "load"
documentation = "Load one tokenizer.json beneath an explicit root."
returns = "native<Tokenizer>"
[[functions.arguments]]
name = "request"
type = "record"

[[functions]]
name = "load_path"
documentation = "Load one tokenizer.json named by an absolute path."
returns = "native<Tokenizer>"
[[functions.arguments]]
name = "path"
type = "string"

[[functions]]
name = "encode"
documentation = "Encode text into an array of token ids."
returns = "array"
[[functions.arguments]]
name = "tokenizer"
type = "native<Tokenizer>"
[[functions.arguments]]
name = "text"
type = "string"

[[functions]]
name = "decode"
documentation = "Decode token ids back into text, keeping control tokens visible."
returns = "string"
[[functions.arguments]]
name = "tokenizer"
type = "native<Tokenizer>"
[[functions.arguments]]
name = "ids"
type = "array"

[[functions]]
name = "token_to_id"
documentation = "Report one token's id."
returns = "i64"
[[functions.arguments]]
name = "tokenizer"
type = "native<Tokenizer>"
[[functions.arguments]]
name = "token"
type = "string"

[[functions]]
name = "info"
documentation = "Report vocabulary size, control-token ids, and the pattern."
returns = "record"
[[functions.arguments]]
name = "tokenizer"
type = "native<Tokenizer>"

[[functions]]
name = "close"
documentation = "Release one tokenizer and invalidate its handle."
returns = "bool"
[[functions.arguments]]
name = "tokenizer"
type = "native<Tokenizer>"
"#;

mlpl_extension_sdk::export_extension! {
    module: generated_export,
    entry: sw_mlpl_extension_v1,
    name: "_hftok",
    version: "0.1.0",
    metadata: crate::METADATA,
    functions: [
        (validate_trampoline, "validate", 1, crate::validate_value),
        (load_trampoline, "load", 1, crate::load_value),
        (load_path_trampoline, "load_path", 1, crate::load_path_value),
        (encode_trampoline, "encode", 2, crate::encode_value),
        (decode_trampoline, "decode", 2, crate::decode_value),
        (token_to_id_trampoline, "token_to_id", 2, crate::token_to_id_value),
        (info_trampoline, "info", 1, crate::info_value),
        (close_trampoline, "close", 1, crate::close_value),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
