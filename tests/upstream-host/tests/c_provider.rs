//! Proves sw-MLPL registers this repository's actual static C descriptor.

#![allow(unsafe_code)]

use mlpl_eval::{Environment, Value};
use mlpl_extension_cabi::{ExtensionDescriptorV1, register_c_extension};

#[test]
fn downstream_c_descriptor_dispatches_through_sw_mlpl() {
    let downstream = mlpl_extension_hello::static_entry();
    // SAFETY: sw-MLPL publishes a byte-for-byte identical V1 descriptor and
    // the downstream static provider remains resident for the process.
    unsafe { register_c_extension(downstream.cast::<ExtensionDescriptorV1>()) }.unwrap();

    let mut environment = Environment::new();
    assert!((scalar(&mut environment, "_hello:answer()") - 42.0).abs() < f64::EPSILON);
    assert!(scalar(&mut environment, "is_ok(_hello:fail())").abs() < f64::EPSILON);
}

fn scalar(environment: &mut Environment, source: &str) -> f64 {
    let tokens = mlpl_parser::lex(source).unwrap();
    let statements = mlpl_parser::parse(&tokens).unwrap();
    match mlpl_eval::eval_program_value(&statements, environment).unwrap() {
        Value::Array(array) => array.data()[0],
        other => panic!("expected scalar from {source}, received {other:?}"),
    }
}
