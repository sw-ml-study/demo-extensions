#![allow(unsafe_code)]

use mlpl_extension_loader::{CallError, Registry, Value};
use mlpl_extension_sdk::DenseArray;

fn registry() -> Registry {
    unsafe { Registry::load_static(mlpl_extension_native3d::static_entry) }.unwrap()
}

fn viewer(registry: &Registry) -> Value {
    registry
        .call(
            "_native3d.create_viewer",
            &[Value::F64(640.0), Value::F64(480.0)],
        )
        .unwrap()
}

fn array(shape: &[usize], values: &[f64]) -> Value {
    Value::Array(DenseArray::from_f64(shape.to_vec(), values.to_vec()).unwrap())
}

#[test]
fn host_deactivation_rejects_all_later_provider_calls() {
    let mut registry = registry();
    assert!(
        registry
            .call(
                "_native3d.create_viewer",
                &[Value::F64(10.0), Value::F64(10.0)]
            )
            .is_ok()
    );
    registry.deactivate();
    assert!(matches!(
        registry.call("_native3d.create_viewer", &[Value::F64(10.0), Value::F64(10.0)]),
        Err(CallError::Inactive(name)) if name == "_native3d"
    ));
}

#[test]
fn bulk_scene_validation_rejects_each_mismatched_parallel_array() {
    let registry = registry();
    let viewer = viewer(&registry);
    let valid = [
        array(&[2, 3], &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]),
        array(&[1, 2], &[0.0, 1.0]),
        array(&[1, 4], &[1.0, 0.0, 0.0, 1.0]),
        array(&[1], &[2.0]),
        array(&[1], &[7.0]),
    ];

    assert!(
        registry
            .call("_native3d.render", &[viewer.clone(), Value::F64(0.0)])
            .is_err()
    );
    for (index, invalid) in [
        array(&[1, 2], &[0.0, 0.0]),
        array(&[1, 2], &[0.0, 2.0]),
        array(&[1, 4], &[2.0, 0.0, 0.0, 1.0]),
        array(&[1], &[0.0]),
        array(&[1], &[0.5]),
    ]
    .into_iter()
    .enumerate()
    {
        let mut arguments = vec![viewer.clone()];
        arguments.extend(valid.iter().cloned());
        arguments[index + 1] = invalid;
        assert!(registry.call("_native3d.set_lines", &arguments).is_err());
    }
}
