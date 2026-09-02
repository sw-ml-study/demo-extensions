use mlpl_array::{DenseArray, Shape};
use mlpl_eval::Value;
use mlpl_native3d_scene::{Camera, Viewport};
use mlpl_native3d_window::live::{
    LiveCommand, RetainedPointScene, parse_live_command, parse_point_patch_command,
    parse_point_scene_command, point_selection_event,
};

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

fn initial_points() -> Value {
    record([
        ("op", Value::Str("set_points".into())),
        ("positions", array(&[2, 3], &[0., 0., 0., 0., 0., 0.])),
        ("sizes", array(&[2], &[12., 12.])),
        ("colors", array(&[2, 4], &[1., 0., 0., 1., 0., 1., 0., 1.])),
        ("opacities", array(&[2], &[1., 0.5])),
        ("ids", array(&[2], &[9., 2.])),
        ("revision", scalar(4.)),
        ("help", Value::Str("point patch test".into())),
    ])
}

fn patch(base: f64, target: f64, ids: &[f64], removes: &[f64]) -> Value {
    let count = ids.len();
    let positions: Vec<_> = (0..count).flat_map(|_| [1., 0., 0.]).collect();
    let colors: Vec<_> = (0..count).flat_map(|_| [0., 0.5, 1., 1.]).collect();
    record([
        ("op", Value::Str("patch_points".into())),
        ("base_revision", scalar(base)),
        ("target_revision", scalar(target)),
        ("ids", array(&[count], ids)),
        ("positions", array(&[count, 3], &positions)),
        ("sizes", array(&[count], &vec![8.; count])),
        ("colors", array(&[count, 4], &colors)),
        ("opacities", array(&[count], &vec![0.75; count])),
        ("remove_ids", array(&[removes.len()], removes)),
    ])
}

#[test]
fn applies_point_upserts_and_removals_atomically() {
    assert!(matches!(
        parse_live_command(initial_points()).unwrap(),
        LiveCommand::PointScene(_)
    ));
    assert!(matches!(
        parse_live_command(patch(4., 5., &[20.], &[9.])).unwrap(),
        LiveCommand::PointPatch(_)
    ));
    let initial = parse_point_scene_command(initial_points()).unwrap();
    let mut retained = RetainedPointScene::from_command(&initial).unwrap();
    retained
        .apply(&parse_point_patch_command(patch(4., 5., &[20.], &[9.])).unwrap())
        .unwrap();
    assert_eq!(retained.revision(), 5);
    assert_eq!(retained.scene().ids(), [2, 20]);
    retained.apply_view(6).unwrap();
    assert_eq!(retained.revision(), 6);
    assert!(retained.apply_view(5).is_err());

    let before = retained.clone();
    let stale = parse_point_patch_command(patch(4., 7., &[30.], &[])).unwrap();
    assert!(retained.apply(&stale).is_err());
    assert_eq!(retained, before);
    let unknown = parse_point_patch_command(patch(6., 7., &[], &[999.])).unwrap();
    assert!(retained.apply(&unknown).is_err());
    assert_eq!(retained, before);
}

#[test]
fn rejects_conflicting_and_unbounded_point_patches_without_partial_state() {
    assert!(parse_point_patch_command(patch(4., 5., &[20., 20.], &[])).is_err());
    assert!(parse_point_patch_command(patch(4., 5., &[20.], &[20.])).is_err());
    assert!(parse_point_patch_command(patch(5., 5., &[], &[])).is_err());
    assert!(parse_point_patch_command(patch(4., 5., &[9_007_199_254_740_992.0], &[])).is_err());

    let initial = parse_point_scene_command(initial_points()).unwrap();
    let mut retained = RetainedPointScene::from_command(&initial).unwrap();
    let before = retained.clone();
    let oversized_ids: Vec<_> = (10_u32..100_010).map(f64::from).collect();
    let oversized = parse_point_patch_command(patch(4., 5., &oversized_ids, &[])).unwrap();
    assert!(retained.apply(&oversized).is_err());
    assert_eq!(retained, before);
}

#[test]
fn selection_event_uses_topmost_stable_id_and_reports_no_hit() {
    let initial = parse_point_scene_command(initial_points()).unwrap();
    let viewport = Viewport::new(64, 64).unwrap();
    let hit = point_selection_event(
        &initial.scene,
        Camera::default(),
        viewport,
        0.0,
        [32.0, 32.0],
        initial.revision,
    )
    .unwrap();
    let Value::Record { fields } = hit else {
        panic!("selection must be an owned record")
    };
    assert_eq!(
        fields.get("kind"),
        Some(&Value::Str("point_selection".into()))
    );
    assert!(matches!(fields.get("hit"), Some(Value::Array(value)) if value.data() == [1.0]));
    assert_eq!(fields.get("id"), Some(&Value::Str("2".into())));
    assert_eq!(fields.get("revision"), Some(&Value::Str("4".into())));

    let Value::Record { fields } = point_selection_event(
        &initial.scene,
        Camera::default(),
        viewport,
        0.0,
        [0.0, 0.0],
        initial.revision,
    )
    .unwrap() else {
        panic!("selection must be an owned record")
    };
    assert!(matches!(fields.get("hit"), Some(Value::Array(value)) if value.data() == [0.0]));
    assert_eq!(fields.get("id"), Some(&Value::Str(String::new())));
}
