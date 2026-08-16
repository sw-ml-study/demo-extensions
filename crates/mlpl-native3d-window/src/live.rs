use std::collections::BTreeMap;

use mlpl_array::DenseArray;
use mlpl_eval::Value;
use mlpl_native3d_scene::LineScene;

const SCENE_SOURCE: &str = include_str!("../../../demos/wireframe-cube/scene.mlpl");
const CONTROLS_SOURCE: &str = include_str!("../../../demos/wireframe-cube/controls.mlpl");
const APPLET_BODY: &str = include_str!("../../../demos/wireframe-cube/live-applet.mlpl");

#[must_use]
pub fn applet_source() -> String {
    format!(
        "{SCENE_SOURCE}\n{}\n{}",
        without_includes(CONTROLS_SOURCE),
        without_includes(APPLET_BODY)
    )
}

#[derive(Debug)]
pub struct SceneCommand {
    pub scene: LineScene,
    pub rotation_speed: f32,
    pub revision: u64,
    pub help: String,
}

#[must_use]
pub fn key_event(key: &str) -> Value {
    record([
        ("kind", Value::Str("key".into())),
        ("key", Value::Str(key.into())),
    ])
}

#[must_use]
pub fn resize_event(width: u32, height: u32) -> Value {
    record([
        ("kind", Value::Str("resize".into())),
        ("width", scalar(f64::from(width))),
        ("height", scalar(f64::from(height))),
    ])
}

#[must_use]
pub fn close_event() -> Value {
    record([("kind", Value::Str("close".into()))])
}

/// Decodes and validates one MLPL-owned generic scene command.
///
/// # Errors
///
/// Rejects malformed records, wrong array shapes, nonuniform styles in the
/// current renderer slice, unstable IDs, and invalid scene values.
pub fn parse_scene_command(value: Value) -> Result<SceneCommand, String> {
    let fields = record_fields(value, "command")?;
    if string_field(&fields, "op")? != "set_scene" {
        return Err("unsupported live command".into());
    }
    let positions = array_field(&fields, "positions", &[8, 3])?;
    let raw_edges = array_field(&fields, "edges", &[12, 2])?;
    let colors = array_field(&fields, "colors", &[12, 4])?;
    let thicknesses = array_field(&fields, "thicknesses", &[12])?;
    let ids = array_field(&fields, "ids", &[12])?;
    validate_parallel_styles(&colors, &thicknesses, &ids)?;
    let edges = raw_edges
        .iter()
        .map(|value| numeric_index(*value, "edge"))
        .collect::<Result<Vec<_>, _>>()?;
    let rotation_speed = to_f32(
        scalar_field(&fields, "rotation_y_speed")?,
        "rotation_y_speed",
    )?;
    let revision = u64::try_from(numeric_index(
        scalar_field(&fields, "revision")?,
        "revision",
    )?)
    .map_err(|_| "revision is out of range".to_owned())?;
    let help = string_field(&fields, "help")?.to_owned();
    let line_color = [
        to_f32(colors[0], "color")?,
        to_f32(colors[1], "color")?,
        to_f32(colors[2], "color")?,
        to_f32(colors[3], "color")?,
    ];
    let scene = LineScene::from_arrays(
        positions
            .into_iter()
            .map(|value| to_f32(value, "position"))
            .collect::<Result<Vec<_>, _>>()?,
        edges,
        rotation_speed,
        line_color,
        to_f32(thicknesses[0], "thickness")?,
    )
    .map_err(|error| format!("invalid live scene: {error:?}"))?;
    Ok(SceneCommand {
        scene,
        rotation_speed,
        revision,
        help,
    })
}

fn validate_parallel_styles(
    colors: &[f64],
    thicknesses: &[f64],
    ids: &[f64],
) -> Result<(), String> {
    let first_color = &colors[..4];
    if !colors.chunks_exact(4).all(|color| color == first_color) {
        return Err("current line renderer requires one uniform color".into());
    }
    if !thicknesses
        .iter()
        .all(|value| value.to_bits() == thicknesses[0].to_bits())
    {
        return Err("current line renderer requires one uniform thickness".into());
    }
    for (index, id) in ids.iter().copied().enumerate() {
        if numeric_index(id, "id")? != index {
            return Err("line ids must be stable and contiguous".into());
        }
    }
    Ok(())
}

fn array_field(
    fields: &BTreeMap<String, Value>,
    name: &str,
    expected: &[usize],
) -> Result<Vec<f64>, String> {
    let Value::Array(array) = fields.get(name).ok_or_else(|| format!("missing {name}"))? else {
        return Err(format!("{name} must be an array"));
    };
    if array.shape().dims() != expected {
        return Err(format!("{name} has the wrong shape"));
    }
    Ok(array.data().to_vec())
}

fn scalar_field(fields: &BTreeMap<String, Value>, name: &str) -> Result<f64, String> {
    let Value::Array(array) = fields.get(name).ok_or_else(|| format!("missing {name}"))? else {
        return Err(format!("{name} must be scalar"));
    };
    if array.rank() != 0 || !array.data()[0].is_finite() {
        return Err(format!("{name} must be a finite scalar"));
    }
    Ok(array.data()[0])
}

fn string_field<'a>(fields: &'a BTreeMap<String, Value>, name: &str) -> Result<&'a str, String> {
    match fields.get(name) {
        Some(Value::Str(value)) => Ok(value),
        _ => Err(format!("{name} must be a string")),
    }
}

fn record_fields(value: Value, name: &str) -> Result<BTreeMap<String, Value>, String> {
    match value {
        Value::Record { fields } => Ok(fields),
        _ => Err(format!("{name} must be a record")),
    }
}

fn numeric_index(value: f64, name: &str) -> Result<usize, String> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
        return Err(format!("{name} must be a nonnegative integer"));
    }
    value
        .to_string()
        .parse()
        .map_err(|_| format!("{name} is out of range"))
}

fn to_f32(value: f64, name: &str) -> Result<f32, String> {
    if !value.is_finite() {
        return Err(format!("{name} must be finite"));
    }
    value
        .to_string()
        .parse::<f32>()
        .map_err(|_| format!("{name} is outside f32 range"))
}

fn scalar(value: f64) -> Value {
    Value::Array(DenseArray::from_scalar(value))
}

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record {
        fields: fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    }
}

fn without_includes(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("include \""))
        .collect::<Vec<_>>()
        .join("\n")
}
