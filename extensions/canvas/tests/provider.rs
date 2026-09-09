#![allow(unsafe_code)]

use mlpl_extension_canvas::{CanvasError, CanvasScene};
use mlpl_extension_loader::Registry;
use mlpl_extension_sdk::DenseArray;

#[test]
fn validates_bounded_polyline_shapes_styles_and_values() {
    let points = DenseArray::from_f64(vec![3, 2], vec![-1.0, 0.0, 0.0, 1.0, 1.0, 0.0]).unwrap();
    let colors =
        DenseArray::from_f64(vec![2, 4], vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.5, 1.0, 1.0]).unwrap();
    let thicknesses = DenseArray::from_f64(vec![2], vec![2.0, 3.0]).unwrap();
    let scene = CanvasScene::new("MLPL", 800.0, 600.0, &points, &colors, &thicknesses).unwrap();
    assert_eq!(scene.point_count(), 3);
    assert_eq!(scene.line_count(), 2);

    let bad_colors = DenseArray::from_f64(vec![1, 4], vec![1.0, 0.0, 0.0, 1.0]).unwrap();
    assert_eq!(
        CanvasScene::new("MLPL", 800.0, 600.0, &points, &bad_colors, &thicknesses),
        Err(CanvasError::ColorShape)
    );
    let bad_points = DenseArray::from_f64(vec![1, 2], vec![0.0, 0.0]).unwrap();
    assert_eq!(
        CanvasScene::new("MLPL", 800.0, 600.0, &bad_points, &colors, &thicknesses),
        Err(CanvasError::PointCount)
    );
}

#[test]
fn descriptor_registers_the_generic_blocking_canvas_call() {
    let registry = unsafe { Registry::load_static(mlpl_extension_canvas::static_entry) }.unwrap();
    let help = registry.help("_canvas.show").unwrap();
    assert!(help.contains("blocking native canvas"));
}
