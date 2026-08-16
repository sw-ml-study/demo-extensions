use mlpl_native3d_scene::{Camera, LineScene, OrbitCamera, Ray3, Viewport};

#[test]
fn orbit_camera_validates_bounds_and_builds_center_pick_ray() {
    let camera = OrbitCamera::new([0.0, 0.0, 0.0], 0.0, 0.0, 5.0, 1.0, 0.1).unwrap();
    let ray = camera
        .pick_ray(Viewport::new(800, 600).unwrap(), [400.0, 300.0])
        .unwrap();
    assert!((ray.origin()[2] - 5.0).abs() < 0.0001);
    assert!(ray.direction()[0].abs() < 0.0001);
    assert!(ray.direction()[1].abs() < 0.0001);
    assert!((ray.direction()[2] + 1.0).abs() < 0.0001);

    assert!(OrbitCamera::new([0.0; 3], 0.0, 1.57, 5.0, 1.0, 0.1).is_err());
    assert!(OrbitCamera::new([0.0; 3], 0.0, 0.0, 0.0, 1.0, 0.1).is_err());
    assert!(
        camera
            .pick_ray(Viewport::new(800, 600).unwrap(), [-1.0, 4.0])
            .is_err()
    );
}

#[test]
fn renderer_camera_orbits_and_pans_without_application_semantics() {
    let scene = LineScene::from_arrays(
        vec![-1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        vec![0, 1],
        0.0,
        [1.0; 4],
        1.0,
    )
    .unwrap();
    let viewport = Viewport::new(800, 600).unwrap();
    let front = scene
        .plan_lines(
            Camera::orbit([0.0; 3], 0.0, 0.0, 5.0, 1.0, 0.1).unwrap(),
            viewport,
            0.0,
        )
        .unwrap();
    let orbited = scene
        .plan_lines(
            Camera::orbit([0.5, 0.25, 0.0], 0.6, 0.3, 7.0, 1.0, 0.1).unwrap(),
            viewport,
            0.0,
        )
        .unwrap();
    let start_delta: f32 = front[0]
        .start()
        .into_iter()
        .zip(orbited[0].start())
        .map(|(a, b)| (a - b).abs())
        .sum();
    let end_delta: f32 = front[0]
        .end()
        .into_iter()
        .zip(orbited[0].end())
        .map(|(a, b)| (a - b).abs())
        .sum();
    assert!(start_delta > 0.01);
    assert!(end_delta > 0.01);
}

#[test]
fn pick_ray_intersects_plane_in_front_and_rejects_parallel_or_behind() {
    let camera = OrbitCamera::new([0.0, 0.0, 0.0], 0.0, 0.0, 5.0, 1.0, 0.1).unwrap();
    let ray = camera
        .pick_ray(Viewport::new(800, 600).unwrap(), [400.0, 300.0])
        .unwrap();
    let hit = ray.intersect_plane([0.0; 3], [0.0, 0.0, 1.0]).unwrap();
    assert!(hit.into_iter().all(|value| value.abs() < 0.0001));

    let parallel = Ray3::new([0.0; 3], [1.0, 0.0, 0.0]).unwrap();
    assert!(
        parallel
            .intersect_plane([0.0, 1.0, 0.0], [0.0, 1.0, 0.0])
            .is_none()
    );
    assert!(
        ray.intersect_plane([0.0, 0.0, 6.0], [0.0, 0.0, 1.0])
            .is_none()
    );
}
