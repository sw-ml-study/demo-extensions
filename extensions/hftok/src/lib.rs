//! Hugging Face byte-level BPE tokenizer capability for MLPL extensions.
//!
//! This step delivers file parsing, validation, and the `validate` diagnostic.
//! Encoding, decoding, and typed handles follow in later steps.

mod byte_level;
mod file_source;
mod tokenizer_file;

pub use byte_level::ByteLevelTokenizer;
pub use file_source::validate_value;
pub use tokenizer_file::{CONTROL_TOKENS, TokenizerFile};

const METADATA: &str = r#"
[[functions]]
name = "validate"
documentation = "Validate a tokenizer.json and report its vocabulary summary."
returns = "record"
[[functions.arguments]]
name = "request"
type = "record"
"#;

mlpl_extension_sdk::export_extension! {
    module: generated_export,
    entry: sw_mlpl_extension_v1,
    name: "_hftok",
    version: "0.1.0",
    metadata: crate::METADATA,
    functions: [
        (validate_trampoline, "validate", 1, crate::validate_value),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
