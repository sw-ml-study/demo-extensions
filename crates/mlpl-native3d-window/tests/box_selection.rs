use mlpl_eval::Value;
use mlpl_native3d_scene::{BoxLimits, BoxScene, Camera, Viewport};
use mlpl_native3d_window::live::box_selection_event;

#[test]
fn selection_event_preserves_exact_id_and_no_hit() {
    let scene = BoxScene::from_parallel_arrays(
        vec![0.0, 0.0, 0.0],
        vec![2.0; 3],
        vec![[0.2, 0.4, 0.6, 1.0]],
        vec![9_007_199_254_740_993],
        BoxLimits::new(1, 48).unwrap(),
    )
    .unwrap();
    let hit = box_selection_event(
        &scene,
        Camera::default(),
        Viewport::new(100, 100).unwrap(),
        0.0,
        [50.0, 50.0],
        7,
    )
    .unwrap();
    let Value::Record { fields } = hit else {
        panic!("record expected")
    };
    assert_eq!(
        fields.get("kind"),
        Some(&Value::Str("box_selection".into()))
    );
    assert_eq!(
        fields.get("id"),
        Some(&Value::Str("9007199254740993".into()))
    );
    assert_eq!(fields.get("revision"), Some(&Value::Str("7".into())));

    let miss = box_selection_event(
        &scene,
        Camera::default(),
        Viewport::new(100, 100).unwrap(),
        0.0,
        [0.0, 0.0],
        7,
    )
    .unwrap();
    let Value::Record { fields } = miss else {
        panic!("record expected")
    };
    assert_eq!(fields.get("id"), Some(&Value::Str(String::new())));
}
