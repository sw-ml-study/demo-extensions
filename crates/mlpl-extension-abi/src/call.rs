use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Debug, Eq, PartialEq)]
pub enum HostCallError<E> {
    Extension(E),
    Panicked,
}

/// Runs one extension operation without allowing a Rust panic to escape.
///
/// # Errors
///
/// Returns `Extension` when the operation returns its normal error and
/// `Panicked` when unwinding begins inside the operation.
pub fn catch_extension_call<T, E>(
    call: impl FnOnce() -> Result<T, E>,
) -> Result<T, HostCallError<E>> {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(HostCallError::Extension(error)),
        Err(_) => Err(HostCallError::Panicked),
    }
}
