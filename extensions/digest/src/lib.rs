//! Generic content-digest capability for MLPL extensions.

mod sha256;

pub use sha256::{
    DEFAULT_CHUNK_BYTES, Sha256Stream, hash_reader, sha256_bytes_value, sha256_file_value,
};

const METADATA: &str = r#"
[[functions]]
name = "sha256_bytes"
documentation = "Hash one in-memory byte string as lowercase hexadecimal."
returns = "string"
[[functions.arguments]]
name = "bytes"
type = "bytes"

[[functions]]
name = "sha256_file"
documentation = "Stream an existing confined file and return its digest and size."
returns = "record"
[[functions.arguments]]
name = "request"
type = "record"
"#;

mlpl_extension_sdk::export_extension! {
    module: generated_export,
    entry: sw_mlpl_extension_v1,
    name: "_digest",
    version: "0.1.0",
    metadata: crate::METADATA,
    functions: [
        (sha256_bytes_trampoline, "sha256_bytes", 1, crate::sha256_bytes_value),
        (sha256_file_trampoline, "sha256_file", 1, crate::sha256_file_value),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
