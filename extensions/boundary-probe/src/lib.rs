//! External provider used to prove rich values through the real sw-MLPL host.

use std::cell::RefCell;
use std::collections::BTreeMap;

use mlpl_extension_sdk::{HandleRegistry, NativeHandle, OwnedError, Value};

const EXTENSION_ID: u64 = 0xB0_0D_A4_7A;
const FOREIGN_EXTENSION_ID: u64 = 0xF0_2E_16_A1;
const VIEWER_TYPE: u64 = 1;

thread_local! {
    static HANDLES: RefCell<HandleRegistry> = const {
        RefCell::new(HandleRegistry::with_limits(EXTENSION_ID, 64, u32::MAX))
    };
}

const METADATA: &str = r#"
[[functions]]
name = "echo_array"
documentation = "Return one dense array after safe boundary copying."
returns = "array<f64>[...]"
[[functions.arguments]]
name = "value"
type = "array<f64>[...]"

[[functions]]
name = "make_handle"
documentation = "Mint one opaque viewer probe handle."
returns = "native<ViewerProbe>"

[[functions]]
name = "handle_value"
documentation = "Read the scalar stored behind a valid viewer probe handle."
returns = "f64"
[[functions.arguments]]
name = "viewer"
type = "native<ViewerProbe>"

[[functions]]
name = "close_handle"
documentation = "Close and invalidate one viewer probe handle."
returns = "bool"
[[functions.arguments]]
name = "viewer"
type = "native<ViewerProbe>"

[[functions]]
name = "stale_handle"
documentation = "Return a deliberately invalidated handle for acceptance tests."
returns = "native<ViewerProbe>"

[[functions]]
name = "foreign_handle"
documentation = "Return a deliberately foreign handle for acceptance tests."
returns = "native<ViewerProbe>"

[[functions]]
name = "event_batch"
documentation = "Return deterministic nested structured event records."
returns = "record"

[[types]]
name = "ViewerProbe"
documentation = "Opaque acceptance-only resource used to prove handle transport."
"#;

fn echo_array(arguments: &[Value]) -> Result<Value, OwnedError> {
    match arguments.first() {
        Some(Value::Array(value)) => Ok(Value::Array(value.clone())),
        _ => Err(OwnedError::invalid_argument("value must be a dense array")),
    }
}

fn make_handle(_arguments: &[Value]) -> Result<Value, OwnedError> {
    HANDLES.with_borrow_mut(|registry| {
        registry
            .insert(VIEWER_TYPE, 42.0_f64)
            .map(Value::Handle)
            .map_err(handle_error)
    })
}

fn handle_value(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    HANDLES.with_borrow(|registry| {
        registry
            .get::<f64>(handle, VIEWER_TYPE)
            .copied()
            .map(Value::F64)
            .map_err(handle_error)
    })
}

fn close_handle(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    HANDLES.with_borrow_mut(|registry| {
        registry
            .remove::<f64>(handle, VIEWER_TYPE)
            .map(|_| Value::Bool(true))
            .map_err(handle_error)
    })
}

fn stale_handle(_arguments: &[Value]) -> Result<Value, OwnedError> {
    HANDLES.with_borrow_mut(|registry| {
        let handle = registry
            .insert(VIEWER_TYPE, 0.0_f64)
            .map_err(handle_error)?;
        registry
            .remove::<f64>(handle, VIEWER_TYPE)
            .map_err(handle_error)?;
        Ok(Value::Handle(handle))
    })
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "all exported handlers share the fallible SDK signature"
)]
fn foreign_handle(_arguments: &[Value]) -> Result<Value, OwnedError> {
    Ok(Value::Handle(NativeHandle::from_parts(
        FOREIGN_EXTENSION_ID,
        VIEWER_TYPE,
        0,
        1,
    )))
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "all exported handlers share the fallible SDK signature"
)]
fn event_batch(_arguments: &[Value]) -> Result<Value, OwnedError> {
    Ok(record([
        ("count", Value::F64(2.0)),
        (
            "e0",
            record([
                ("kind", Value::String("key".into())),
                ("x", Value::F64(5.0)),
            ]),
        ),
        (
            "e1",
            record([
                ("kind", Value::String("resize".into())),
                ("x", Value::F64(7.0)),
            ]),
        ),
    ]))
}

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn handle_argument(arguments: &[Value]) -> Result<NativeHandle, OwnedError> {
    match arguments.first() {
        Some(Value::Handle(handle)) => Ok(*handle),
        _ => Err(OwnedError::invalid_argument(
            "viewer must be a native handle",
        )),
    }
}

fn handle_error(error: mlpl_extension_sdk::HandleError) -> OwnedError {
    OwnedError::extension(format!("invalid viewer handle: {error:?}"))
}

mlpl_extension_sdk::export_extension! {
    module: generated_export,
    entry: sw_mlpl_extension_v1,
    name: "_boundary",
    version: "0.1.0",
    metadata: crate::METADATA,
    functions: [
        (echo_array_trampoline, "echo_array", 1, crate::echo_array),
        (make_handle_trampoline, "make_handle", 0, crate::make_handle),
        (handle_value_trampoline, "handle_value", 1, crate::handle_value),
        (close_handle_trampoline, "close_handle", 1, crate::close_handle),
        (stale_handle_trampoline, "stale_handle", 0, crate::stale_handle),
        (foreign_handle_trampoline, "foreign_handle", 0, crate::foreign_handle),
        (event_batch_trampoline, "event_batch", 0, crate::event_batch),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
