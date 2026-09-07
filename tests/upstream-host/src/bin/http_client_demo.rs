//! Minimal host for the MLPL HTTP-client demo and static extension provider.

#![allow(unsafe_code)]

use std::process::ExitCode;

use mlpl_eval::Environment;
use mlpl_extension_cabi::{ExtensionDescriptorV1, register_c_extension};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("http-client-demo: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let descriptor = mlpl_extension_http_client::static_entry();
    // SAFETY: the linked static descriptor and its code remain resident until
    // this process exits.
    unsafe { register_c_extension(descriptor.cast::<ExtensionDescriptorV1>()) }?;

    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../demos/http-client/get.mlpl"),
    )
    .map_err(|error| format!("cannot read demo source: {error}"))?;
    let tokens = mlpl_parser::lex(&source).map_err(|error| error.to_string())?;
    let statements = mlpl_parser::parse(&tokens).map_err(|error| error.to_string())?;
    let mut environment = Environment::new();
    environment.cli_args = std::env::args().skip(1).collect();
    mlpl_eval::eval_program_value(&statements, &mut environment)
        .map_err(|error| error.to_string())?;
    Ok(())
}
