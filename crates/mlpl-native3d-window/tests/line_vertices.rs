use mlpl_native3d_scene::{Camera, LineScene, Viewport};
use mlpl_native3d_window::{line_vertices, text_vertices, text_vertices_colored};

fn line_scene() -> LineScene {
    LineScene::parse(
        r#"{"schema":"sw-ml-study.native3d.line-scene","version":1,"positions":{"shape":[2,3],"values":[-1,0,0,1,0,0]},"edges":{"shape":[1,2],"values":[0,1]},"controls":{"rotation_speed":0.75,"line_color":[0.25,0.5,1,1],"line_thickness":4}}"#,
    )
    .unwrap()
}

#[test]
fn expands_planned_lines_to_gpu_triangles_without_scene_semantics() {
    let viewport = Viewport::new(200, 100).unwrap();
    let lines = line_scene()
        .plan_lines(Camera::default(), viewport, 0.0)
        .unwrap();
    let vertices = line_vertices(&lines, viewport);

    assert_eq!(vertices.len(), 6);
    for (actual, expected) in vertices[0].color.into_iter().zip([0.25, 0.5, 1.0, 1.0]) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }
    assert!(vertices.iter().all(|vertex| {
        vertex.position[0].is_finite()
            && vertex.position[1].is_finite()
            && (-1.0..=1.0).contains(&vertex.position[0])
            && (-1.0..=1.0).contains(&vertex.position[1])
    }));
    let vertical_span = vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::NEG_INFINITY, f32::max)
        - vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
    assert!((vertical_span - 0.08).abs() < 0.001);
}

#[test]
fn empty_line_plan_produces_no_gpu_work() {
    assert!(line_vertices(&[], Viewport::new(32, 32).unwrap()).is_empty());
}

#[test]
fn help_text_expands_to_visible_gpu_quads() {
    let viewport = Viewport::new(800, 600).unwrap();
    let vertices = text_vertices("W/S WIDTH", viewport);
    assert!(!vertices.is_empty());
    assert!(vertices.iter().all(|vertex| {
        (-1.0..=1.0).contains(&vertex.position[0]) && (-1.0..=1.0).contains(&vertex.position[1])
    }));
}

#[test]
fn colored_status_text_preserves_requested_accent() {
    let color = [1.0, 0.9, 0.15, 1.0];
    let vertices = text_vertices_colored(
        "SELECTED: tensor",
        Viewport::new(800, 600).unwrap(),
        [14.0, 180.0],
        color,
    );
    assert!(!vertices.is_empty());
    assert!(vertices.iter().all(|vertex| {
        vertex
            .color
            .into_iter()
            .zip(color)
            .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
    }));
}

#[test]
fn model_metadata_digits_and_path_punctuation_are_visible() {
    let viewport = Viewport::new(800, 600).unwrap();
    for character in "0123456789._:%=,".chars() {
        assert!(
            !text_vertices(&character.to_string(), viewport).is_empty(),
            "metadata glyph {character:?} must produce visible geometry"
        );
    }
}
