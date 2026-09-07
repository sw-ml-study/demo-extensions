//! Confined `SQLite` capability for MLPL extensions.

mod database;

pub use database::{
    begin_value, close_value, commit_value, execute_value, open_value, query_value, rollback_value,
};

const METADATA: &str = r#"
[[functions]]
name = "open"
documentation = "Open one SQLite database beneath an explicit sandbox root."
returns = "native<SqliteConnection>"
[[functions.arguments]]
name = "config"
type = "record"

[[functions]]
name = "execute"
documentation = "Execute one parameterized non-query SQL statement."
returns = "record"
[[functions.arguments]]
name = "connection"
type = "native<SqliteConnection>"
[[functions.arguments]]
name = "sql"
type = "string"
[[functions.arguments]]
name = "params"
type = "record"

[[functions]]
name = "query"
documentation = "Run one parameterized query and return a bounded result."
returns = "record"
[[functions.arguments]]
name = "connection"
type = "native<SqliteConnection>"
[[functions.arguments]]
name = "sql"
type = "string"
[[functions.arguments]]
name = "params"
type = "record"

[[functions]]
name = "begin"
documentation = "Begin one explicit transaction."
returns = "bool"
[[functions.arguments]]
name = "connection"
type = "native<SqliteConnection>"

[[functions]]
name = "commit"
documentation = "Commit the active explicit transaction."
returns = "bool"
[[functions.arguments]]
name = "connection"
type = "native<SqliteConnection>"

[[functions]]
name = "rollback"
documentation = "Roll back the active explicit transaction."
returns = "bool"
[[functions.arguments]]
name = "connection"
type = "native<SqliteConnection>"

[[functions]]
name = "close"
documentation = "Roll back if needed, close, and invalidate a connection."
returns = "bool"
[[functions.arguments]]
name = "connection"
type = "native<SqliteConnection>"

[[types]]
name = "SqliteConnection"
documentation = "Opaque confined SQLite connection with fixed query budgets."
"#;

mlpl_extension_sdk::export_extension! {
    module: generated_export,
    entry: sw_mlpl_extension_v1,
    name: "_sqlite",
    version: "0.1.0",
    metadata: crate::METADATA,
    functions: [
        (open_trampoline, "open", 1, crate::open_value),
        (execute_trampoline, "execute", 3, crate::execute_value),
        (query_trampoline, "query", 3, crate::query_value),
        (begin_trampoline, "begin", 1, crate::begin_value),
        (commit_trampoline, "commit", 1, crate::commit_value),
        (rollback_trampoline, "rollback", 1, crate::rollback_value),
        (close_trampoline, "close", 1, crate::close_value),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
