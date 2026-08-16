use mlpl_array::{DenseArray, Shape};
use mlpl_eval::Value;
use mlpl_native3d_window::live::{RetainedScene, parse_scene_command, parse_scene_patch_command};

fn array(shape: &[usize], values: &[f64]) -> Value {
    Value::Array(DenseArray::new(Shape::new(shape.to_vec()), values.to_vec()).unwrap())
}

fn scalar(value: f64) -> Value {
    Value::Array(DenseArray::from_scalar(value))
}

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record {
        fields: fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    }
}

fn initial_scene() -> Value {
    record([
        ("op", Value::Str("set_scene".into())),
        ("positions", array(&[2, 3], &[0., 0., 0., 1., 0., 0.])),
        ("edges", array(&[1, 2], &[0., 1.])),
        ("colors", array(&[1, 4], &[1., 1., 1., 1.])),
        ("thicknesses", array(&[1], &[1.])),
        ("ids", array(&[1], &[10.])),
        ("rotation_y_speed", scalar(0.)),
        ("revision", scalar(4.)),
        ("help", Value::Str("patch test".into())),
    ])
}

fn patch(base: f64, target: f64, ids: &[f64], removes: &[f64]) -> Value {
    let count = ids.len();
    let starts: Vec<_> = (0..count).flat_map(|_| [2., 0., 0.]).collect();
    let ends: Vec<_> = (0..count).flat_map(|_| [3., 0., 0.]).collect();
    let colors: Vec<_> = (0..count).flat_map(|_| [0., 1., 0., 1.]).collect();
    record([
        ("op", Value::Str("patch_scene".into())),
        ("base_revision", scalar(base)),
        ("target_revision", scalar(target)),
        ("ids", array(&[count], ids)),
        ("starts", array(&[count, 3], &starts)),
        ("ends", array(&[count, 3], &ends)),
        ("colors", array(&[count, 4], &colors)),
        ("thicknesses", array(&[count], &vec![2.; count])),
        ("remove_ids", array(&[removes.len()], removes)),
    ])
}

#[test]
fn applies_id_addressed_add_update_remove_atomically() {
    let initial = parse_scene_command(initial_scene()).unwrap();
    let mut retained = RetainedScene::from_scene_command(&initial).unwrap();
    let command = parse_scene_patch_command(patch(4., 5., &[20.], &[10.])).unwrap();
    retained.apply(&command).unwrap();
    assert_eq!(retained.revision(), 5);
    assert_eq!(retained.scene().edges().len(), 1);
    let update = parse_scene_patch_command(patch(5., 6., &[20.], &[])).unwrap();
    retained.apply(&update).unwrap();
    assert_eq!(retained.revision(), 6, "an existing ID is updated in place");

    let before = retained.clone();
    let stale = parse_scene_patch_command(patch(4., 7., &[30.], &[])).unwrap();
    assert!(retained.apply(&stale).is_err());
    assert_eq!(retained, before, "stale patches change no retained state");
}

#[test]
fn rejects_duplicate_conflicting_and_unbounded_patch_descriptors() {
    assert!(parse_scene_patch_command(patch(4., 5., &[20., 20.], &[])).is_err());
    assert!(parse_scene_patch_command(patch(4., 5., &[20.], &[20.])).is_err());
    assert!(parse_scene_patch_command(patch(5., 5., &[], &[])).is_err());
    let oversized_ids: Vec<_> = (0_u32..100_001).map(f64::from).collect();
    let oversized = record([
        ("op", Value::Str("patch_scene".into())),
        ("base_revision", scalar(5.)),
        ("target_revision", scalar(6.)),
        ("ids", array(&[oversized_ids.len()], &oversized_ids)),
        ("remove_ids", array(&[0], &[])),
    ]);
    assert!(parse_scene_patch_command(oversized).is_err());
}
