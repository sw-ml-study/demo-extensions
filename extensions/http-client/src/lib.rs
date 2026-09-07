//! Bounded synchronous HTTP capability for MLPL extensions.

mod client;
mod middleware;

pub use client::request_value;
pub use middleware::middleware_plan_value;

const METADATA: &str = r#"
[[functions]]
name = "request"
documentation = "Perform one bounded synchronous HTTP or HTTPS request."
returns = "record"
[[functions.arguments]]
name = "request"
type = "record"

[[functions]]
name = "middleware_plan"
documentation = "Validate V1 middleware TOML and return its fixed execution plan."
returns = "record"
[[functions.arguments]]
name = "config_toml"
type = "string"
"#;

mlpl_extension_sdk::export_extension! {
    module: generated_export,
    entry: sw_mlpl_extension_v1,
    name: "_http",
    version: "0.1.0",
    metadata: crate::METADATA,
    functions: [
        (request_trampoline, "request", 1, crate::request_value),
        (middleware_plan_trampoline, "middleware_plan", 1, crate::middleware_plan_value),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
