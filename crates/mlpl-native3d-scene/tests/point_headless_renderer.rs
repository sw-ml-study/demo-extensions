use mlpl_native3d_scene::{Camera, PointLimits, PointScene, RenderError, Viewport};

fn scene(
    positions: Vec<f32>,
    sizes: Vec<f32>,
    colors: Vec<[f32; 4]>,
    opacities: Vec<f32>,
    ids: Vec<u64>,
) -> PointScene {
    let count = ids.len();
    PointScene::from_parallel_arrays(
        positions,
        sizes,
        colors,
        opacities,
        ids,
        PointLimits::new(count, count * 44).unwrap(),
    )
    .unwrap()
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[test]
fn projects_culls_and_orders_points_far_to_near_with_stable_ids() {
    let cloud = scene(
        vec![
            0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 100.0, 0.0, 0.0, 0.0, 0.0, 5.0,
        ],
        vec![10.0; 4],
        vec![[1.0, 0.0, 0.0, 1.0]; 4],
        vec![1.0; 4],
        vec![40, 20, 30, 10],
    );
    let plan = cloud
        .plan_points(Camera::default(), Viewport::new(100, 100).unwrap(), 0.0)
        .unwrap();

    assert_eq!(
        plan.points().len(),
        2,
        "offscreen and behind-camera points are culled"
    );
    assert_eq!(plan.points()[0].id(), 40, "far point is painted first");
    assert_eq!(plan.points()[1].id(), 20, "near point is painted last");
    assert!(plan.points()[0].depth() > plan.points()[1].depth());
    assert_eq!(plan.pick([50.0, 50.0]).unwrap().id(), 20);
}

#[test]
fn equal_depth_overlap_uses_lowest_stable_id_as_the_topmost_pick() {
    let scene = scene(
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![12.0, 12.0],
        vec![[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]],
        vec![1.0, 1.0],
        vec![9, 2],
    );
    let plan = scene
        .plan_points(Camera::default(), Viewport::new(64, 64).unwrap(), 0.0)
        .unwrap();
    assert_eq!(
        plan.points()
            .iter()
            .map(|point| point.id())
            .collect::<Vec<_>>(),
        [9, 2]
    );
    assert_eq!(plan.pick([32.0, 32.0]).unwrap().id(), 2);
    assert!(plan.pick([0.0, 0.0]).is_none());
}

#[test]
fn point_raster_is_bounded_repeatable_and_attribute_sensitive() {
    let cloud = scene(
        vec![-0.5, 0.0, 0.0, 0.5, 0.0, 0.0],
        vec![8.0, 14.0],
        vec![[1.0, 0.2, 0.1, 1.0], [0.1, 0.5, 1.0, 1.0]],
        vec![1.0, 0.5],
        vec![1, 2],
    );
    let viewport = Viewport::new(96, 64).unwrap();
    let first = cloud
        .render_points_headless(Camera::default(), viewport, 0.25)
        .unwrap();
    let second = cloud
        .render_points_headless(Camera::default(), viewport, 0.25)
        .unwrap();
    let changed = scene(
        vec![-0.5, 0.0, 0.0, 0.5, 0.0, 0.0],
        vec![8.0, 6.0],
        vec![[1.0, 0.2, 0.1, 1.0], [0.1, 1.0, 0.2, 1.0]],
        vec![1.0, 1.0],
        vec![1, 2],
    )
    .render_points_headless(Camera::default(), viewport, 0.25)
    .unwrap();
    assert_eq!(first, second);
    assert_ne!(first, changed);
    assert_eq!(first.dimensions(), [96, 64]);
    assert_eq!(&first.rgba()[0..4], &[8, 10, 16, 255]);
    assert_eq!(fnv1a(&first.ppm_bytes()), 2_572_401_580_842_708_506);
    assert_eq!(
        cloud.render_points_headless(Camera::default(), viewport, f32::NAN),
        Err(RenderError::NonFiniteRotation)
    );
}
