use std::collections::BTreeMap;

use mlpl_array::{DenseArray, Shape};
use mlpl_eval::Value;
use mlpl_native3d_scene::Camera;
use mlpl_native3d_window::live::parse_scene_command;

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
            .map(|(key, value)| (key.into(), value))
            .collect(),
    }
}

fn command(camera: Option<Value>) -> Value {
    let positions = vec![0.0; 24];
    let edges: Vec<f64> = (0..12)
        .flat_map(|index| [0.0, f64::from(index % 8)])
        .collect();
    let colors: Vec<f64> = (0..12).flat_map(|_| [1.0; 4]).collect();
    let mut fields = BTreeMap::from([
        ("op".into(), Value::Str("set_scene".into())),
        ("positions".into(), array(&[8, 3], &positions)),
        ("edges".into(), array(&[12, 2], &edges)),
        ("colors".into(), array(&[12, 4], &colors)),
        ("thicknesses".into(), array(&[12], &[1.0; 12])),
        (
            "ids".into(),
            array(&[12], &(0..12).map(f64::from).collect::<Vec<_>>()),
        ),
        ("rotation_y_speed".into(), scalar(0.0)),
        ("revision".into(), scalar(0.0)),
        ("help".into(), Value::Str(String::new())),
    ]);
    if let Some(camera) = camera {
        fields.insert("camera".into(), camera);
    }
    Value::Record { fields }
}

fn camera(target: &[f64]) -> Value {
    record([
        ("target", array(&[target.len()], target)),
        ("yaw", scalar(0.5)),
        ("pitch", scalar(-0.25)),
        ("distance", scalar(8.0)),
        ("fov", scalar(1.1)),
        ("near", scalar(0.2)),
    ])
}

#[test]
fn parses_mlpl_owned_orbit_camera() {
    let parsed = parse_scene_command(command(Some(camera(&[1.0, 2.0, 3.0])))).unwrap();
    assert!(
        parsed
            .camera
            .target()
            .into_iter()
            .zip([1.0, 2.0, 3.0])
            .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
    );
    assert!((parsed.camera.yaw() - 0.5).abs() < f32::EPSILON);
    assert!((parsed.camera.distance() - 8.0).abs() < f32::EPSILON);
}

#[test]
fn rejects_malformed_mlpl_camera_and_defaults_when_absent() {
    assert!(parse_scene_command(command(Some(camera(&[0.0, 0.0])))).is_err());
    assert_eq!(
        parse_scene_command(command(None)).unwrap().camera,
        Camera::default()
    );
}
