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
