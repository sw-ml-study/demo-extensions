//! Safe validation, loading, registration, and lifecycle management.
//!
//! Dynamic loading retains the library for as long as registered callables can
//! be invoked and copies every foreign result before returning to callers.

mod error;
#[allow(unsafe_code)]
mod foreign;
#[allow(unsafe_code)]
mod registry;
mod value;

pub use error::{CallError, LoadError};
pub use registry::Registry;
pub use value::Value;
