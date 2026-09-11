use mlpl_native3d_scene::{BoxLimits, BoxScene, BoxSceneError, Camera, Ray3, Viewport};

fn scene(ids: Vec<u64>) -> BoxScene {
    BoxScene::from_parallel_arrays(
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 2.0],
        vec![2.0; 6],
        vec![[0.2, 0.4, 0.6, 1.0]; 2],
        ids,
        BoxLimits::new(2, 96).unwrap(),
    )
    .unwrap()
}

#[test]
fn ray_pick_returns_nearest_box_and_stable_distance() {
    let hit = scene(vec![10, 20])
        .pick(Ray3::new([0.0, 0.0, 5.0], [0.0, 0.0, -1.0]).unwrap(), 0.0)
        .unwrap()
        .unwrap();
    assert_eq!(hit.id(), 20);
    assert!((hit.distance() - 2.0).abs() < f32::EPSILON);
}

#[test]
fn equal_distance_ties_choose_lowest_stable_id() {
    let scene = BoxScene::from_parallel_arrays(
        vec![-1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        vec![2.0; 6],
        vec![[1.0; 4]; 2],
        vec![9, 3],
        BoxLimits::new(2, 96).unwrap(),
    )
    .unwrap();
    let hit = scene
        .pick(Ray3::new([0.0, 0.0, 5.0], [0.0, 0.0, -1.0]).unwrap(), 0.0)
        .unwrap()
        .unwrap();
    assert_eq!(hit.id(), 3);
}

#[test]
fn camera_pick_rotation_and_missing_selection_fail_cleanly() {
    let scene = scene(vec![10, 20]);
    let ray = Camera::default()
        .pick_ray(Viewport::new(100, 100).unwrap(), [50.0, 50.0])
        .unwrap();
    assert!(
        scene
            .pick(ray, std::f32::consts::FRAC_PI_2)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        scene.validate_selection(Some(99)),
        Err(BoxSceneError::UnknownId(99))
    );
    assert_eq!(scene.validate_selection(Some(10)), Ok(()));
    assert_eq!(scene.validate_selection(None), Ok(()));
}
