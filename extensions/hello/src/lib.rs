//! Small external extension proving the safe public authoring path.

use mlpl_extension_sdk::{OwnedError, Value};

const METADATA: &str = r#"
[[functions]]
name = "answer"
documentation = "Return the canonical extension answer."
returns = "i64"

[[functions]]
name = "fail"
documentation = "Return a contained extension failure."
returns = "i64"

[[functions]]
name = "panic"
documentation = "Demonstrate containment of an extension panic."
returns = "i64"

[[functions]]
name = "sum_positions"
documentation = "Sum one dense N by 3 f32 position array."
returns = "f64"

[[functions.arguments]]
name = "positions"
type = "array<f32>[N,3]"
"#;

#[expect(
    clippy::unnecessary_wraps,
    reason = "all exported handlers share the fallible SDK signature"
)]
fn answer(_arguments: &[Value]) -> Result<Value, OwnedError> {
    Ok(Value::I64(42))
}

fn fail(_arguments: &[Value]) -> Result<Value, OwnedError> {
    Err(OwnedError::extension("hello requested failure"))
}

fn panic_call(_arguments: &[Value]) -> Result<Value, OwnedError> {
    panic!("contained hello panic")
}

fn sum_positions(arguments: &[Value]) -> Result<Value, OwnedError> {
    let Some(Value::Array(array)) = arguments.first() else {
        return Err(OwnedError::invalid_argument(
            "positions must be a dense array",
        ));
    };
    if array.view().shape().len() != 2 || array.view().shape()[1] != 3 {
        return Err(OwnedError::invalid_argument(
            "positions must have shape [N,3]",
        ));
    }
    let values = array
        .view()
        .as_f32()
        .map_err(|_| OwnedError::invalid_argument("positions must have dtype f32"))?;
    Ok(Value::F64(
        values.iter().map(|value| f64::from(*value)).sum(),
    ))
}

mlpl_extension_sdk::export_extension! {
    module: generated_export,
    entry: sw_mlpl_extension_v1,
    name: "_hello",
    version: "0.1.0",
    metadata: crate::METADATA,
    functions: [
        (answer_trampoline, "answer", 0, crate::answer),
        (fail_trampoline, "fail", 0, crate::fail),
        (panic_trampoline, "panic", 0, crate::panic_call),
        (sum_positions_trampoline, "sum_positions", 1, crate::sum_positions),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
