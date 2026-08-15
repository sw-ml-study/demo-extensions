//! Proves this repository's provider uses the real sw-MLPL data boundary.

#![allow(unsafe_code)]

use mlpl_eval::{Environment, Value};
use mlpl_extension_cabi::{ExtensionDescriptorV1, register_c_extension};

static REGISTER: std::sync::Once = std::sync::Once::new();

fn environment() -> Environment {
    REGISTER.call_once(|| {
        let downstream = mlpl_extension_boundary_probe::static_entry();
        unsafe { register_c_extension(downstream.cast::<ExtensionDescriptorV1>()) }.unwrap();
    });
    Environment::new()
}

fn evaluate(environment: &mut Environment, source: &str) -> Value {
    let tokens = mlpl_parser::lex(source).unwrap();
    let statements = mlpl_parser::parse(&tokens).unwrap();
    mlpl_eval::eval_program_value(&statements, environment).unwrap()
}

fn scalar(environment: &mut Environment, source: &str) -> f64 {
    match evaluate(environment, source) {
        Value::Array(array) => array.data()[0],
        other => panic!("expected scalar from {source}, received {other:?}"),
    }
}

#[test]
fn dense_arrays_cross_in_and_out_through_the_real_interpreter() {
    let mut environment = environment();
    match evaluate(&mut environment, "_boundary:echo_array([[1,2,3],[4,5,6]])") {
        Value::Array(array) => {
            assert_eq!(array.shape().dims(), &[2, 3]);
            assert_eq!(array.data(), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        }
        other => panic!("expected returned dense array, received {other:?}"),
    }
    assert!(
        scalar(
            &mut environment,
            "is_ok(_boundary:echo_array(\"not an array\"))",
        )
        .abs()
            < f64::EPSILON
    );
}

#[test]
fn native_handles_survive_variables_and_fail_cleanly_when_invalid() {
    let mut environment = environment();
    assert!(
        (scalar(
            &mut environment,
            "viewer = _boundary:make_handle()\n_boundary:handle_value(viewer)",
        ) - 42.0)
            .abs()
            < f64::EPSILON
    );
    assert!(scalar(
        &mut environment,
        "viewer = _boundary:make_handle()\n_boundary:close_handle(viewer)\nis_ok(_boundary:handle_value(viewer))",
    )
    .abs()
        < f64::EPSILON);
    assert!(
        scalar(
            &mut environment,
            "is_ok(_boundary:handle_value(_boundary:stale_handle()))",
        )
        .abs()
            < f64::EPSILON
    );
    assert!(
        scalar(
            &mut environment,
            "is_ok(_boundary:handle_value(_boundary:foreign_handle()))",
        )
        .abs()
            < f64::EPSILON
    );
    assert!(scalar(&mut environment, "is_ok(_boundary:handle_value(42))",).abs() < f64::EPSILON);
}

#[test]
fn nested_records_return_as_field_addressable_mlpl_values() {
    let mut environment = environment();
    assert!(
        (scalar(
            &mut environment,
            "events = _boundary:event_batch()\nevents.e0.x + events.e1.x + events.count",
        ) - 14.0)
            .abs()
            < f64::EPSILON
    );
}
