use mlpl_native3d_scene::{Camera, LineScene, RenderError, Viewport};

fn scene(positions: &str, shape: &str, edges: &str, edge_shape: &str) -> LineScene {
    LineScene::parse(&format!(
        r#"{{"schema":"sw-ml-study.native3d.line-scene","version":1,"positions":{{"shape":{shape},"values":{positions}}},"edges":{{"shape":{edge_shape},"values":{edges}}},"controls":{{"rotation_speed":0.75,"line_color":[0.25,0.5,1.0,1.0],"line_thickness":3.0}}}}"#
    ))
    .unwrap()
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[test]
fn plans_generic_rotated_lines_with_aspect_color_and_thickness() {
    let line = scene("[-1.0,0.0,0.0,1.0,0.0,0.0]", "[2,3]", "[0,1]", "[1,2]");
    let square = line
        .plan_lines(Camera::default(), Viewport::new(200, 200).unwrap(), 0.0)
        .unwrap();
    let wide = line
        .plan_lines(Camera::default(), Viewport::new(400, 200).unwrap(), 0.0)
        .unwrap();

    assert_eq!(square.len(), 1);
    for (actual, expected) in square[0].color().into_iter().zip([0.25, 0.5, 1.0, 1.0]) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }
    assert!((square[0].thickness() - 3.0).abs() < f32::EPSILON);
    assert!((wide[0].length() - square[0].length()).abs() < 0.001);
    assert!((wide[0].start()[0] - square[0].start()[0] - 100.0).abs() < 0.001);

    let rotated = line
        .plan_lines(Camera::default(), Viewport::new(200, 200).unwrap(), 0.5)
        .unwrap();
    assert!((square[0].start()[0] - rotated[0].start()[0]).abs() > 0.01);
    assert!((square[0].end()[0] - rotated[0].end()[0]).abs() > 0.01);
}

#[test]
fn clips_near_plane_and_rejects_invalid_render_bounds() {
    let crossing = scene("[0.0,0.0,4.5,1.0,0.0,0.0]", "[2,3]", "[0,1]", "[1,2]");
    let planned = crossing
        .plan_lines(Camera::default(), Viewport::new(64, 64).unwrap(), 0.0)
        .unwrap();
    assert_eq!(planned.len(), 1, "near-plane crossing is clipped");
    assert!(planned[0].start().into_iter().all(f32::is_finite));

    assert_eq!(Viewport::new(0, 10), Err(RenderError::InvalidViewport));
    assert_eq!(Viewport::new(20_000, 10), Err(RenderError::InvalidViewport));
    assert_eq!(
        Camera::perspective(0.05, 1.0, 0.1),
        Err(RenderError::InvalidCamera)
    );

    let degenerate = scene("[0.0,0.0,0.0]", "[1,3]", "[0,0]", "[1,2]");
    assert!(
        degenerate
            .plan_lines(Camera::default(), Viewport::new(64, 64).unwrap(), 0.0)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        degenerate.plan_lines(Camera::default(), Viewport::new(64, 64).unwrap(), f32::NAN),
        Err(RenderError::NonFiniteRotation)
    );
}

#[test]
fn raster_output_is_reproducible_and_reflects_geometry_and_controls() {
    let short = scene("[-0.5,0.0,0.0,0.5,0.0,0.0]", "[2,3]", "[0,1]", "[1,2]");
    let long = scene("[-2.0,0.0,0.0,2.0,0.0,0.0]", "[2,3]", "[0,1]", "[1,2]");
    let viewport = Viewport::new(96, 64).unwrap();

    let first = short
        .render_headless(Camera::default(), viewport, 0.25)
        .unwrap();
    let second = short
        .render_headless(Camera::default(), viewport, 0.25)
        .unwrap();
    let longer = long
        .render_headless(Camera::default(), viewport, 0.25)
        .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.dimensions(), [96, 64]);
    assert_eq!(&first.rgba()[0..4], &[8, 10, 16, 255]);
    assert_ne!(first.rgba(), longer.rgba());
    assert_eq!(first.ppm_bytes(), second.ppm_bytes());
    assert_eq!(fnv1a(&first.ppm_bytes()), 78_031_761_721_831_318);
}
