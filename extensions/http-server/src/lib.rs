//! Callback-free bounded HTTP server capability for MLPL extensions.

mod server;

pub use server::{
    close_value, listen_value, local_address_value, next_request_value, respond_value,
};

const METADATA: &str = r#"
[[functions]]
name = "listen"
documentation = "Open one bounded loopback-only HTTP listener."
returns = "native<HttpServer>"
[[functions.arguments]]
name = "config"
type = "record"
[[functions.arguments]]
name = "middleware_toml"
type = "string"

[[functions]]
name = "local_address"
documentation = "Return the bound listener address for diagnostics."
returns = "string"
[[functions.arguments]]
name = "server"
type = "native<HttpServer>"

[[functions]]
name = "next_request"
documentation = "Poll one bounded request or return nil on timeout."
returns = "record|nil"
[[functions.arguments]]
name = "server"
type = "native<HttpServer>"
[[functions.arguments]]
name = "timeout_ms"
type = "i64"

[[functions]]
name = "respond"
documentation = "Send one response and consume its pending request ID."
returns = "bool"
[[functions.arguments]]
name = "server"
type = "native<HttpServer>"
[[functions.arguments]]
name = "request_id"
type = "i64"
[[functions.arguments]]
name = "response"
type = "record"

[[functions]]
name = "close"
documentation = "Close a listener and invalidate all pending requests."
returns = "bool"
[[functions.arguments]]
name = "server"
type = "native<HttpServer>"

[[types]]
name = "HttpServer"
documentation = "Opaque loopback listener with bounded pending request state."
"#;

mlpl_extension_sdk::export_extension! {
    module: generated_export,
    entry: sw_mlpl_extension_v1,
    name: "_web",
    version: "0.1.0",
    metadata: crate::METADATA,
    functions: [
        (listen_trampoline, "listen", 2, crate::listen_value),
        (local_address_trampoline, "local_address", 1, crate::local_address_value),
        (next_request_trampoline, "next_request", 2, crate::next_request_value),
        (respond_trampoline, "respond", 3, crate::respond_value),
        (close_trampoline, "close", 1, crate::close_value),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
