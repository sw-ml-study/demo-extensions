//! Exercises the headless native3d provider through the real interpreter.

#![allow(unsafe_code)]

use mlpl_eval::{Environment, Value};
use mlpl_extension_cabi::{ExtensionDescriptorV1, register_c_extension};

static REGISTER: std::sync::Once = std::sync::Once::new();

fn environment() -> Environment {
    REGISTER.call_once(|| {
        let descriptor = mlpl_extension_native3d::static_entry();
        unsafe { register_c_extension(descriptor.cast::<ExtensionDescriptorV1>()) }.unwrap();
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

const LINES: &str = r"
positions = [[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]]
edges = [[0,1],[1,2],[2,3],[3,0]]
colors = [[1,0,0,1],[0,1,0,1],[0,0,1,1],[1,1,1,1]]
thicknesses = [2,2,2,2]
ids = [10,11,12,13]
";

#[test]
fn lifecycle_bulk_scene_and_render_state_dispatch_through_interpreter() {
    let mut environment = environment();
    let source = format!(
        "viewer = _native3d:create_viewer(640,480)\n{LINES}\n_native3d:set_lines(viewer,positions,edges,colors,thicknesses,ids)\nframe = _native3d:render(viewer,0.5)\nsize = _native3d:viewer_size(viewer)\nstate = _native3d:viewer_state(viewer)\nsize.width + size.height + state.vertices + state.lines + frame.frame"
    );
    assert!((scalar(&mut environment, &source) - 1129.0).abs() < f64::EPSILON);
}

#[test]
fn malformed_arrays_closed_and_stale_viewers_fail_cleanly() {
    let mut environment = environment();
    let malformed = format!(
        "viewer = _native3d:create_viewer(640,480)\n{LINES}\nis_ok(_native3d:set_lines(viewer,[[0,0]],edges,colors,thicknesses,ids))"
    );
    assert!(scalar(&mut environment, &malformed).abs() < f64::EPSILON);
    assert!(scalar(
        &mut environment,
        "viewer = _native3d:create_viewer(20,20)\n_native3d:close(viewer)\nis_ok(_native3d:viewer_state(viewer))",
    )
    .abs()
        < f64::EPSILON);
    assert!(scalar(
        &mut environment,
        "old = _native3d:create_viewer(20,20)\n_native3d:close(old)\nreplacement = _native3d:create_viewer(20,20)\nis_ok(_native3d:viewer_state(old))",
    )
    .abs()
        < f64::EPSILON);
}
