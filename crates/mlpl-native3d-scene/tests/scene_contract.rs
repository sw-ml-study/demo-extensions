use mlpl_native3d_scene::{LineScene, SceneError};

const VALID: &str = r#"{
  "schema":"sw-ml-study.native3d.line-scene",
  "version":1,
  "positions":{"shape":[2,3],"values":[-1.0,-2.0,-3.0,1.0,2.0,3.0]},
  "edges":{"shape":[1,2],"values":[0,1]},
  "controls":{"rotation_speed":0.75,"line_color":[0.2,0.8,1.0,1.0],"line_thickness":2.5}
}"#;

#[test]
fn parses_generic_line_scene_and_serializes_canonically() {
    let scene = LineScene::parse(VALID).unwrap();
    assert_eq!(scene.positions().shape(), [2, 3]);
    assert_eq!(scene.edges(), [[0, 1]]);
    assert!((scene.controls().rotation_speed() - 0.75).abs() < f32::EPSILON);
    for (actual, expected) in scene
        .controls()
        .line_color()
        .into_iter()
        .zip([0.2, 0.8, 1.0, 1.0])
    {
        assert!((actual - expected).abs() < f32::EPSILON);
    }
    assert!((scene.controls().line_thickness() - 2.5).abs() < f32::EPSILON);
    assert_eq!(
        scene.to_json(),
        r#"{"schema":"sw-ml-study.native3d.line-scene","version":1,"positions":{"shape":[2,3],"values":[-1.0,-2.0,-3.0,1.0,2.0,3.0]},"edges":{"shape":[1,2],"values":[0,1]},"controls":{"rotation_speed":0.75,"line_color":[0.2,0.8,1.0,1.0],"line_thickness":2.5}}"#
    );
    assert_eq!(LineScene::parse(&scene.to_json()).unwrap(), scene);
}

#[test]
fn rejects_malformed_shapes_indices_and_controls() {
    assert_eq!(LineScene::parse("{"), Err(SceneError::Malformed));
    assert_eq!(
        LineScene::parse(&VALID.replace("[2,3]", "[2,2]")),
        Err(SceneError::PositionShape)
    );
    assert_eq!(
        LineScene::parse(&VALID.replace("[0,1]", "[0,2]")),
        Err(SceneError::EdgeIndex { edge: 0, vertex: 2 })
    );
    assert_eq!(
        LineScene::parse(&VALID.replace("\"line_thickness\":2.5", "\"line_thickness\":0.0")),
        Err(SceneError::LineThickness)
    );
    assert_eq!(
        LineScene::parse(&VALID.replace("\"rotation_speed\":0.75", "\"rotation_speed\":11.0")),
        Err(SceneError::RotationSpeed)
    );
    assert_eq!(
        LineScene::parse(&VALID.replace("0.2,0.8,1.0,1.0", "1.2,0.8,1.0,1.0")),
        Err(SceneError::LineColor)
    );
}
