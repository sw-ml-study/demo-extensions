//! Blocking native canvas extension over validated MLPL bulk arrays.

use std::collections::BTreeMap;

use mlpl_extension_sdk::{DenseArray, OwnedError, Value};

mod window;

const MAX_POINTS: usize = 100_000;
const MAX_TITLE_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasScene {
    title: String,
    width: u32,
    height: u32,
    points: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    thicknesses: Vec<f32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasError {
    Title,
    Dimension,
    PointShape,
    PointCount,
    PointValue,
    ColorShape,
    ColorValue,
    ThicknessShape,
    ThicknessValue,
    ArrayType,
}

impl CanvasScene {
    /// Validates and copies one generic two-dimensional polyline.
    ///
    /// # Errors
    ///
    /// Rejects invalid dimensions, titles, shapes, dtypes, bounds, and values.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "validated finite normalized f64 inputs are deliberately copied to GPU f32 storage"
    )]
    pub fn new(
        title: &str,
        width: f64,
        height: f64,
        points: &DenseArray,
        colors: &DenseArray,
        thicknesses: &DenseArray,
    ) -> Result<Self, CanvasError> {
        if title.is_empty() || title.len() > MAX_TITLE_BYTES || title.chars().any(char::is_control)
        {
            return Err(CanvasError::Title);
        }
        let width = dimension(width)?;
        let height = dimension(height)?;
        let point_view = points.view();
        let point_shape = point_view.shape();
        if point_shape.len() != 2 || point_shape[1] != 2 {
            return Err(CanvasError::PointShape);
        }
        let count = point_shape[0];
        if !(2..=MAX_POINTS).contains(&count) {
            return Err(CanvasError::PointCount);
        }
        let line_count = count - 1;
        if colors.view().shape() != [line_count, 4] {
            return Err(CanvasError::ColorShape);
        }
        if thicknesses.view().shape() != [line_count] {
            return Err(CanvasError::ThicknessShape);
        }
        let point_values = point_view.as_f64().map_err(|_| CanvasError::ArrayType)?;
        let color_values = colors.view().as_f64().map_err(|_| CanvasError::ArrayType)?;
        let thickness_values = thicknesses
            .view()
            .as_f64()
            .map_err(|_| CanvasError::ArrayType)?;
        if point_values
            .iter()
            .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
        {
            return Err(CanvasError::PointValue);
        }
        if color_values
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(CanvasError::ColorValue);
        }
        if thickness_values
            .iter()
            .any(|value| !value.is_finite() || !(0.25..=64.0).contains(value))
        {
            return Err(CanvasError::ThicknessValue);
        }
        Ok(Self {
            title: title.to_owned(),
            width,
            height,
            points: point_values
                .chunks_exact(2)
                .map(|p| [p[0] as f32, p[1] as f32])
                .collect(),
            colors: color_values
                .chunks_exact(4)
                .map(|c| [c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32])
                .collect(),
            thicknesses: thickness_values.iter().map(|value| *value as f32).collect(),
        })
    }

    #[must_use]
    pub fn point_count(&self) -> usize {
        self.points.len()
    }
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.colors.len()
    }
}

fn dimension(value: f64) -> Result<u32, CanvasError> {
    if value.fract() == 0.0 && (64.0..=8192.0).contains(&value) {
        value
            .to_string()
            .parse()
            .map_err(|_| CanvasError::Dimension)
    } else {
        Err(CanvasError::Dimension)
    }
}

fn argument<'a>(arguments: &'a [Value], index: usize, name: &str) -> Result<&'a Value, OwnedError> {
    arguments
        .get(index)
        .ok_or_else(|| OwnedError::invalid_argument(format!("missing {name}")))
}

fn number(value: &Value, name: &str) -> Result<f64, OwnedError> {
    match value {
        Value::F64(value) => Ok(*value),
        Value::I64(value) => value
            .to_string()
            .parse()
            .map_err(|_| OwnedError::invalid_argument(format!("{name} is outside numeric bounds"))),
        _ => Err(OwnedError::invalid_argument(format!(
            "{name} must be numeric"
        ))),
    }
}

fn scene_argument(arguments: &[Value]) -> Result<CanvasScene, OwnedError> {
    let Value::String(title) = argument(arguments, 0, "title")? else {
        return Err(OwnedError::invalid_argument("title must be a string"));
    };
    let Value::Array(points) = argument(arguments, 3, "points")? else {
        return Err(OwnedError::invalid_argument("points must be an array"));
    };
    let Value::Array(colors) = argument(arguments, 4, "colors")? else {
        return Err(OwnedError::invalid_argument("colors must be an array"));
    };
    let Value::Array(thicknesses) = argument(arguments, 5, "thicknesses")? else {
        return Err(OwnedError::invalid_argument("thicknesses must be an array"));
    };
    CanvasScene::new(
        title,
        number(argument(arguments, 1, "width")?, "width")?,
        number(argument(arguments, 2, "height")?, "height")?,
        points,
        colors,
        thicknesses,
    )
    .map_err(|error| OwnedError::invalid_argument(format!("invalid canvas scene: {error:?}")))
}

fn summary(scene: &CanvasScene, closed: bool) -> Value {
    Value::Record(BTreeMap::from([
        (
            "points".into(),
            Value::F64(
                scene
                    .point_count()
                    .to_string()
                    .parse()
                    .expect("bounded point count converts to f64"),
            ),
        ),
        (
            "lines".into(),
            Value::F64(
                scene
                    .line_count()
                    .to_string()
                    .parse()
                    .expect("bounded line count converts to f64"),
            ),
        ),
        ("closed".into(), Value::Bool(closed)),
    ]))
}

fn validate(arguments: &[Value]) -> Result<Value, OwnedError> {
    let scene = scene_argument(arguments)?;
    Ok(summary(&scene, false))
}

fn show(arguments: &[Value]) -> Result<Value, OwnedError> {
    let scene = scene_argument(arguments)?;
    let result = summary(&scene, true);
    window::show(scene).map_err(OwnedError::extension)?;
    Ok(result)
}

const METADATA: &str = r#"
[[functions]]
name = "validate"
documentation = "Validate and summarize a bounded MLPL polyline without opening a window."
returns = "record"
[[functions.arguments]]
name = "title"
type = "string"
[[functions.arguments]]
name = "width"
type = "f64"
[[functions.arguments]]
name = "height"
type = "f64"
[[functions.arguments]]
name = "points"
type = "array<f64>[N,2]"
[[functions.arguments]]
name = "colors"
type = "array<f64>[N-1,4]"
[[functions.arguments]]
name = "thicknesses"
type = "array<f64>[N-1]"

[[functions]]
name = "show"
documentation = "Open one blocking native canvas for a validated MLPL polyline and return after close."
returns = "record"
[[functions.arguments]]
name = "title"
type = "string"
[[functions.arguments]]
name = "width"
type = "f64"
[[functions.arguments]]
name = "height"
type = "f64"
[[functions.arguments]]
name = "points"
type = "array<f64>[N,2]"
[[functions.arguments]]
name = "colors"
type = "array<f64>[N-1,4]"
[[functions.arguments]]
name = "thicknesses"
type = "array<f64>[N-1]"
"#;

mlpl_extension_sdk::export_extension! {
    module: generated_export,
    entry: sw_mlpl_extension_v1,
    name: "_canvas",
    version: "0.1.0",
    metadata: crate::METADATA,
    functions: [
        (validate_trampoline, "validate", 6, crate::validate),
        (show_trampoline, "show", 6, crate::show),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
