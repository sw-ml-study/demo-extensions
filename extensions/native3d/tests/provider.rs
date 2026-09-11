#![allow(unsafe_code)]

use std::path::PathBuf;

use mlpl_extension_loader::{CallError, ProviderKind, Registry, Value};
use mlpl_extension_sdk::DenseArray;

fn registry() -> Registry {
    unsafe { Registry::load_static(mlpl_extension_native3d::static_entry) }.unwrap()
}

fn dynamic_registry() -> Registry {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target/debug/deps");
    path.push(format!(
        "{}mlpl_extension_native3d{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    Registry::load(path).unwrap()
}

#[test]
fn dynamic_and_static_providers_publish_the_same_box_api() {
    let dynamic = dynamic_registry();
    let static_provider = registry();
    assert_eq!(dynamic.provider_kind(), ProviderKind::Dynamic);
    assert_eq!(static_provider.provider_kind(), ProviderKind::Static);
    assert_eq!(dynamic.function_names(), static_provider.function_names());
    assert!(dynamic.function_names().contains(&"_native3d.set_boxes"));
    assert!(dynamic.function_names().contains(&"_native3d.set_view"));
    assert!(dynamic.function_names().contains(&"_native3d.pick_box"));
    assert!(
        dynamic
            .function_names()
            .contains(&"_native3d.set_selection")
    );
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

#[test]
fn bulk_boxes_orthographic_pick_and_selection_share_one_typed_viewer() {
    let registry = registry();
    let viewer = viewer(&registry);
    registry
        .call(
            "_native3d.set_boxes",
            &[
                viewer.clone(),
                array(&[2, 3], &[-1.0, 0.0, 0.0, 1.0, 0.0, -2.0]),
                array(&[2, 3], &[1.0; 6]),
                array(&[2, 4], &[0.2, 0.4, 0.6, 1.0, 0.6, 0.4, 0.2, 1.0]),
                array(&[2], &[10.0, 20.0]),
            ],
        )
        .unwrap();
    registry
        .call(
            "_native3d.set_view",
            &[
                viewer.clone(),
                Value::String("orthographic".into()),
                array(&[3], &[0.0, 0.0, 0.0]),
                Value::F64(0.0),
                Value::F64(0.0),
                Value::F64(6.0),
                Value::F64(4.0),
                Value::F64(0.1),
            ],
        )
        .unwrap();
    let hit = registry
        .call(
            "_native3d.pick_box",
            &[
                viewer.clone(),
                Value::F64(230.0),
                Value::F64(240.0),
                Value::F64(0.0),
            ],
        )
        .unwrap();
    let Value::Record(hit) = hit else {
        panic!("pick must return a record")
    };
    assert_eq!(hit.get("hit"), Some(&Value::Bool(true)));
    assert_eq!(hit.get("id"), Some(&Value::F64(10.0)));
    registry
        .call(
            "_native3d.set_selection",
            &[viewer.clone(), Value::F64(10.0)],
        )
        .unwrap();
    let Value::Record(state) = registry.call("_native3d.viewer_state", &[viewer]).unwrap() else {
        panic!("state must be a record")
    };
    assert_eq!(state.get("boxes"), Some(&Value::F64(2.0)));
    assert_eq!(
        state.get("projection"),
        Some(&Value::String("orthographic".into()))
    );
    assert_eq!(state.get("selected_id"), Some(&Value::F64(10.0)));
}

#[test]
fn box_api_rejects_bad_shapes_unknown_ids_and_wrong_handles() {
    let registry = registry();
    let viewer = viewer(&registry);
    assert!(
        registry
            .call("_native3d.set_boxes", std::slice::from_ref(&viewer))
            .is_err()
    );
    assert!(
        registry
            .call(
                "_native3d.set_boxes",
                &[
                    viewer.clone(),
                    array(&[1, 2], &[0.0, 0.0]),
                    array(&[1, 3], &[1.0; 3]),
                    array(&[1, 4], &[1.0; 4]),
                    array(&[1], &[7.0]),
                ],
            )
            .is_err()
    );
    assert!(
        registry
            .call("_native3d.set_selection", &[viewer, Value::F64(99.0)])
            .is_err()
    );
}
