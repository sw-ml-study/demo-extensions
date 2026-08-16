use std::collections::BTreeMap;

use crate::interaction::{InputEvent, Modifiers, PointerButton, PointerButtons};
use mlpl_array::DenseArray;
use mlpl_eval::Value;
use mlpl_native3d_scene::{Camera, LineScene};

const SCENE_SOURCE: &str = include_str!("../../../demos/wireframe-cube/scene.mlpl");
const CAMERA_SOURCE: &str = include_str!("../../../lib/native3d/camera.mlpl");
const GEOMETRY_SOURCE: &str = include_str!("../../../lib/native3d/geometry.mlpl");
const CONTROLS_SOURCE: &str = include_str!("../../../demos/wireframe-cube/controls.mlpl");
const APPLET_BODY: &str = include_str!("../../../demos/wireframe-cube/live-applet.mlpl");
const TTT_MODEL: &str = include_str!("../../../demos/tic-tac-toe/model.mlpl");
const TTT_SCENE: &str = include_str!("../../../demos/tic-tac-toe/scene.mlpl");
const TTT_CONTROLS: &str = include_str!("../../../demos/tic-tac-toe/controls.mlpl");
const TTT_APPLET: &str = include_str!("../../../demos/tic-tac-toe/live-applet.mlpl");
const LIFE_MODEL: &str = include_str!("../../../demos/life-plane/model.mlpl");
const LIFE_CONTROLS: &str = include_str!("../../../demos/life-plane/controls.mlpl");
const LIFE_SCENE: &str = include_str!("../../../demos/life-plane/scene.mlpl");
const LIFE_APPLET: &str = include_str!("../../../demos/life-plane/live-applet.mlpl");
const LIFE_TORUS_MODEL: &str = include_str!("../../../demos/life-torus/model.mlpl");
const LIFE_TORUS_CONTROLS: &str = include_str!("../../../demos/life-torus/controls.mlpl");
const LIFE_TORUS_SCENE: &str = include_str!("../../../demos/life-torus/scene.mlpl");
const LIFE_TORUS_APPLET: &str = include_str!("../../../demos/life-torus/live-applet.mlpl");

#[must_use]
pub fn applet_source() -> String {
    format!(
        "{SCENE_SOURCE}\n{CAMERA_SOURCE}\n{}\n{}",
        without_includes(CONTROLS_SOURCE),
        without_includes(APPLET_BODY)
    )
}

#[must_use]
pub fn tic_tac_toe_applet_source() -> String {
    format!(
        "{CAMERA_SOURCE}\n{GEOMETRY_SOURCE}\n{}\n{}\n{}\n{}",
        without_includes(TTT_MODEL),
        without_includes(TTT_SCENE),
        without_includes(TTT_CONTROLS),
        without_includes(TTT_APPLET)
    )
}

#[must_use]
pub fn life_applet_source() -> String {
    format!(
        "{CAMERA_SOURCE}\n{GEOMETRY_SOURCE}\n{}\n{}\n{}\n{}",
        without_includes(LIFE_MODEL),
        without_includes(LIFE_CONTROLS),
        without_includes(LIFE_SCENE),
        without_includes(LIFE_APPLET)
    )
}

#[must_use]
pub fn life_torus_applet_source() -> String {
    format!(
        "{CAMERA_SOURCE}\n{GEOMETRY_SOURCE}\n{}\n{}\n{}\n{}\n{}\n{}",
        without_includes(LIFE_MODEL),
        without_includes(LIFE_TORUS_MODEL),
        without_includes(LIFE_CONTROLS),
        without_includes(LIFE_TORUS_CONTROLS),
        without_includes(LIFE_TORUS_SCENE),
        without_includes(LIFE_TORUS_APPLET)
    )
}

#[derive(Debug)]
pub struct SceneCommand {
    pub scene: LineScene,
    pub camera: Camera,
    pub rotation_speed: f32,
    pub revision: u64,
    pub help: String,
}

#[derive(Debug)]
pub struct ViewCommand {
    pub camera: Camera,
    pub revision: u64,
    pub help: String,
}

#[derive(Debug)]
pub enum LiveCommand {
    Scene(SceneCommand),
    View(ViewCommand),
    FrameAck(u64),
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

/// Encodes one validated, renderer-neutral input event as an owned MLPL record.
#[must_use]
pub fn input_event(event: InputEvent) -> Value {
    match event {
        InputEvent::PointerMove {
            position,
            buttons,
            modifiers,
        } => record(
            [
                ("kind", Value::Str("pointer_move".into())),
                ("x", scalar(position[0])),
                ("y", scalar(position[1])),
                (
                    "left",
                    scalar(bool_number(buttons.contains(PointerButtons::LEFT))),
                ),
            ]
            .into_iter()
            .chain(modifier_fields(modifiers)),
        ),
        InputEvent::PointerButton {
            button,
            pressed,
            position,
            modifiers,
        } => record(
            [
                (
                    "kind",
                    Value::Str(
                        if pressed {
                            "pointer_down"
                        } else {
                            "pointer_up"
                        }
                        .into(),
                    ),
                ),
                ("button", Value::Str(button_name(button).into())),
                ("x", scalar(position[0])),
                ("y", scalar(position[1])),
            ]
            .into_iter()
            .chain(modifier_fields(modifiers)),
        ),
        InputEvent::Wheel {
            delta,
            position,
            modifiers,
        } => record(
            [
                ("kind", Value::Str("wheel".into())),
                ("dx", scalar(delta[0])),
                ("dy", scalar(delta[1])),
                ("x", scalar(position[0])),
                ("y", scalar(position[1])),
            ]
            .into_iter()
            .chain(modifier_fields(modifiers)),
        ),
        InputEvent::Frame {
            delta_ms,
            elapsed_ms,
        } => record([
            ("kind", Value::Str("frame".into())),
            ("delta_ms", scalar(delta_ms)),
            ("elapsed_ms", scalar(elapsed_ms)),
        ]),
    }
}

fn modifier_fields(modifiers: Modifiers) -> [(&'static str, Value); 4] {
    [
        (
            "shift",
            scalar(bool_number(modifiers.contains(Modifiers::SHIFT))),
        ),
        (
            "control",
            scalar(bool_number(modifiers.contains(Modifiers::CONTROL))),
        ),
        (
            "alt",
            scalar(bool_number(modifiers.contains(Modifiers::ALT))),
        ),
        (
            "meta",
            scalar(bool_number(modifiers.contains(Modifiers::META))),
        ),
    ]
}

const fn button_name(button: PointerButton) -> &'static str {
    match button {
        PointerButton::Left => "left",
        PointerButton::Middle => "middle",
        PointerButton::Right => "right",
    }
}

const fn bool_number(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
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
    parse_scene_fields(&fields)
}

/// Decodes a camera/help-only update that retains the current geometry.
///
/// # Errors
///
/// Rejects malformed records, unsupported operations, revisions, or cameras.
pub fn parse_view_command(value: Value) -> Result<ViewCommand, String> {
    let fields = record_fields(value, "command")?;
    if string_field(&fields, "op")? != "set_view" {
        return Err("unsupported live command".into());
    }
    parse_view_fields(&fields)
}

/// Decodes an MLPL acknowledgement for one consumed frame event.
///
/// # Errors
///
/// Rejects malformed records, unsupported operations, or revisions.
pub fn parse_frame_ack(value: Value) -> Result<u64, String> {
    let fields = record_fields(value, "command")?;
    if string_field(&fields, "op")? != "frame_ack" {
        return Err("unsupported live command".into());
    }
    parse_revision(&fields)
}

/// Decodes either a complete scene replacement or a retained-scene view diff.
///
/// # Errors
///
/// Rejects malformed or unsupported live commands.
pub fn parse_live_command(value: Value) -> Result<LiveCommand, String> {
    let fields = record_fields(value, "command")?;
    match string_field(&fields, "op")? {
        "set_scene" => parse_scene_fields(&fields).map(LiveCommand::Scene),
        "set_view" => parse_view_fields(&fields).map(LiveCommand::View),
        "frame_ack" => parse_revision(&fields).map(LiveCommand::FrameAck),
        _ => Err("unsupported live command".into()),
    }
}

fn parse_scene_fields(fields: &BTreeMap<String, Value>) -> Result<SceneCommand, String> {
    let (positions, _position_shape) = matrix_field(fields, "positions", 3)?;
    let (raw_edges, edge_shape) = matrix_field(fields, "edges", 2)?;
    let edge_count = edge_shape[0];
    let colors = array_field(fields, "colors", &[edge_count, 4])?;
    let thicknesses = array_field(fields, "thicknesses", &[edge_count])?;
    let ids = array_field(fields, "ids", &[edge_count])?;
    validate_parallel_styles(&colors, &thicknesses, &ids)?;
    let edges = raw_edges
        .iter()
        .map(|value| numeric_index(*value, "edge"))
        .collect::<Result<Vec<_>, _>>()?;
    let rotation_speed = to_f32(
        scalar_field(fields, "rotation_y_speed")?,
        "rotation_y_speed",
    )?;
    let view = parse_view_fields(fields)?;
    let colors = colors
        .chunks_exact(4)
        .map(|color| {
            Ok([
                to_f32(color[0], "color")?,
                to_f32(color[1], "color")?,
                to_f32(color[2], "color")?,
                to_f32(color[3], "color")?,
            ])
        })
        .collect::<Result<Vec<_>, String>>()?;
    let thicknesses = thicknesses
        .into_iter()
        .map(|value| to_f32(value, "thickness"))
        .collect::<Result<Vec<_>, _>>()?;
    let scene = LineScene::from_parallel_arrays(
        positions
            .into_iter()
            .map(|value| to_f32(value, "position"))
            .collect::<Result<Vec<_>, _>>()?,
        edges,
        rotation_speed,
        colors,
        thicknesses,
    )
    .map_err(|error| format!("invalid live scene: {error:?}"))?;
    Ok(SceneCommand {
        scene,
        camera: view.camera,
        rotation_speed,
        revision: view.revision,
        help: view.help,
    })
}

fn parse_view_fields(fields: &BTreeMap<String, Value>) -> Result<ViewCommand, String> {
    let revision = parse_revision(fields)?;
    let help = string_field(fields, "help")?.to_owned();
    let camera = match fields.get("camera") {
        None => Camera::default(),
        Some(Value::Record { fields }) => parse_camera(fields)?,
        Some(_) => return Err("camera must be a record".into()),
    };
    Ok(ViewCommand {
        camera,
        revision,
        help,
    })
}

fn parse_revision(fields: &BTreeMap<String, Value>) -> Result<u64, String> {
    u64::try_from(numeric_index(
        scalar_field(fields, "revision")?,
        "revision",
    )?)
    .map_err(|_| "revision is out of range".to_owned())
}

fn parse_camera(fields: &BTreeMap<String, Value>) -> Result<Camera, String> {
    let target = array_field(fields, "target", &[3])?
        .into_iter()
        .map(|value| to_f32(value, "camera target"))
        .collect::<Result<Vec<_>, _>>()?;
    Camera::orbit(
        [target[0], target[1], target[2]],
        to_f32(scalar_field(fields, "yaw")?, "camera yaw")?,
        to_f32(scalar_field(fields, "pitch")?, "camera pitch")?,
        to_f32(scalar_field(fields, "distance")?, "camera distance")?,
        to_f32(scalar_field(fields, "fov")?, "camera fov")?,
        to_f32(scalar_field(fields, "near")?, "camera near")?,
    )
    .map_err(|_| "camera values are outside supported bounds".into())
}

fn validate_parallel_styles(
    _colors: &[f64],
    _thicknesses: &[f64],
    ids: &[f64],
) -> Result<(), String> {
    for (index, id) in ids.iter().copied().enumerate() {
        if numeric_index(id, "id")? != index {
            return Err("line ids must be stable and contiguous".into());
        }
    }
    Ok(())
}

fn matrix_field(
    fields: &BTreeMap<String, Value>,
    name: &str,
    columns: usize,
) -> Result<(Vec<f64>, [usize; 2]), String> {
    let Value::Array(array) = fields.get(name).ok_or_else(|| format!("missing {name}"))? else {
        return Err(format!("{name} must be an array"));
    };
    let dims = array.shape().dims();
    if dims.len() != 2 || dims[0] == 0 || dims[1] != columns {
        return Err(format!("{name} has the wrong shape"));
    }
    Ok((array.data().to_vec(), [dims[0], dims[1]]))
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
