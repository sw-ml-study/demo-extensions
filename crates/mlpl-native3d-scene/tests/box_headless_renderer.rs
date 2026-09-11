use mlpl_native3d_scene::{BoxLimits, BoxScene, Camera, Viewport};

#[test]
fn depth_order_and_headless_raster_are_deterministic() {
    let scene = BoxScene::from_parallel_arrays(
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        vec![1.0; 6],
        vec![[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]],
        vec![10, 20],
        BoxLimits::new(8, 4096).unwrap(),
    )
    .unwrap();
    let camera = Camera::default();
    let viewport = Viewport::new(96, 64).unwrap();
    let plan = scene.plan_box_triangles(camera, viewport, 0.0).unwrap();
    assert!(
        plan.triangles()
            .windows(2)
            .all(|pair| pair[0].depth() >= pair[1].depth())
    );

    let first = scene.render_boxes_headless(camera, viewport, 0.0).unwrap();
    let second = scene.render_boxes_headless(camera, viewport, 0.0).unwrap();
    assert_eq!(first, second);
    let center = (32 * 96 + 48) * 4;
    assert_eq!(&first.rgba()[center..center + 4], &[0, 255, 0, 255]);
    assert!(
        first
            .rgba()
            .chunks_exact(4)
            .any(|pixel| pixel != [8, 10, 16, 255])
    );
}

#[test]
fn orthographic_physical_view_preserves_size_across_depth() {
    let scene = BoxScene::from_parallel_arrays(
        vec![-1.0, 0.0, 0.0, 1.0, 0.0, -2.0],
        vec![1.0; 6],
        vec![[0.2, 0.4, 0.6, 1.0]; 2],
        vec![10, 20],
        BoxLimits::new(2, 4096).unwrap(),
    )
    .unwrap();
    let camera = Camera::orthographic([0.0; 3], 0.0, 0.0, 6.0, 4.0, 0.1).unwrap();
    let plan = scene
        .plan_box_triangles(camera, Viewport::new(200, 100).unwrap(), 0.0)
        .unwrap();
    let width = |id| {
        let xs = plan
            .triangles()
            .iter()
            .filter(|triangle| triangle.id() == id)
            .flat_map(|triangle| triangle.vertices().map(|point| point[0]));
        let (minimum, maximum) = xs.fold((f32::INFINITY, f32::NEG_INFINITY), |bounds, x| {
            (bounds.0.min(x), bounds.1.max(x))
        });
        maximum - minimum
    };
    assert!((width(10) - width(20)).abs() < 0.000_01);
}
