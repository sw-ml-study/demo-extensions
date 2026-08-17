use std::collections::{BTreeMap, BTreeSet};

use crate::interaction::{InputEvent, Modifiers, PointerButton, PointerButtons};
use mlpl_array::DenseArray;
use mlpl_eval::Value;
use mlpl_native3d_scene::{Camera, LineScene};

/// Runs a parked-main MLPL applet with one explicit canonical filesystem root.
///
/// This downstream adapter preserves the host's opt-in containment policy until
/// sw-MLPL exposes a configured variant of `run_applet_with_host` directly.
///
/// # Errors
///
/// Returns an error when the root cannot be canonicalized, is not a directory,
/// the worker cannot start, or MLPL evaluation fails.
pub fn run_applet_with_host_root<H>(
    source: &str,
    root: &std::path::Path,
    host: H,
) -> Result<Value, mlpl_eval::EvalError>
where
    H: FnOnce(std::sync::mpsc::Receiver<Value>, std::sync::mpsc::Sender<Value>),
{
    let root = root.canonicalize().map_err(|error| {
        mlpl_eval::EvalError::Unsupported(format!("invalid applet filesystem root: {error}"))
    })?;
    if !root.is_dir() {
        return Err(mlpl_eval::EvalError::Unsupported(
            "applet filesystem root is not a directory".into(),
        ));
    }
    let (command_tx, command_rx) = std::sync::mpsc::channel();
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let (result_tx, result_rx) = std::sync::mpsc::channel();
    let source = source.to_owned();
    let worker = std::thread::Builder::new()
        .name("mlpl-native3d-applet".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let mut environment = mlpl_eval::Environment::new();
            environment.ui_host_thread = true;
            environment.fs_root = Some(root);
            let handle = environment.register_port(command_tx, event_rx);
            environment.ext_handles.insert("port".into(), handle);
            let _ = result_tx.send(mlpl_eval::eval_source_value(&source, &mut environment));
        })
        .map_err(|error| {
            mlpl_eval::EvalError::Unsupported(format!("cannot start rooted applet worker: {error}"))
        })?;
    host(command_rx, event_tx);
    let _ = worker.join();
    result_rx.recv().unwrap_or_else(|_| {
        Err(mlpl_eval::EvalError::Unsupported(
            "rooted applet worker died".into(),
        ))
    })
}

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
const ATLAS_SCAN: &str = include_str!("../../../lib/model-atlas/bounded_scan.mlpl");
const ATLAS_INTERCHANGE: &str = include_str!("../../../lib/model-atlas/interchange.mlpl");
const ATLAS_FIXTURE: &str = include_str!("../../../fixtures/model-atlas/tensor_city_derived.mlpl");
const ATLAS_DETAIL_FIXTURE: &str =
    include_str!("../../../fixtures/model-atlas/detail_derived.mlpl");
const ATLAS_ARCHITECTURE: &str = include_str!("../../../demos/model-atlas/architecture.mlpl");
const ATLAS_MODEL: &str = include_str!("../../../demos/model-atlas/model.mlpl");
const ATLAS_SCENE: &str = include_str!("../../../demos/model-atlas/scene.mlpl");
const ATLAS_CONTROLS: &str = include_str!("../../../demos/model-atlas/controls.mlpl");
const ATLAS_APPLET: &str = include_str!("../../../demos/model-atlas/live-applet.mlpl");
const FILE_ATLAS_HEADER: &str =
    include_str!("../../../../demo-ml-utils/src/formats/safetensors_header.mlpl");
const FILE_ATLAS_CATALOG: &str =
    include_str!("../../../../demo-ml-utils/src/formats/safetensors_catalog.mlpl");
const FILE_ATLAS_MENU: &str = include_str!("../../../demos/model-atlas-file/menu.mlpl");
const FILE_ATLAS_MODEL: &str = include_str!("../../../demos/model-atlas-file/model.mlpl");
const FILE_ATLAS_SCENE: &str = include_str!("../../../demos/model-atlas-file/scene.mlpl");
const FILE_ATLAS_APPLET: &str = include_str!("../../../demos/model-atlas-file/live-applet.mlpl");
const DISK_USAGE_MODEL: &str = include_str!("../../../demos/disk-usage/model.mlpl");
const DISK_USAGE_SCENE: &str = include_str!("../../../demos/disk-usage/scene.mlpl");
const DISK_USAGE_APPLET: &str = include_str!("../../../demos/disk-usage/live-applet.mlpl");
const AUDIO_MODEL: &str = include_str!("../../../demos/audio-spectrum/model.mlpl");
const AUDIO_SCENE: &str = include_str!("../../../demos/audio-spectrum/scene.mlpl");
const AUDIO_APPLET: &str = include_str!("../../../demos/audio-spectrum/live-applet.mlpl");

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

#[must_use]
pub fn model_atlas_applet_source() -> String {
    format!(
        "{CAMERA_SOURCE}\n{GEOMETRY_SOURCE}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        without_includes(ATLAS_SCAN),
        without_includes(ATLAS_INTERCHANGE),
        without_includes(ATLAS_FIXTURE),
        without_includes(ATLAS_DETAIL_FIXTURE),
        without_includes(ATLAS_ARCHITECTURE),
        without_includes(ATLAS_MODEL),
        without_includes(ATLAS_SCENE),
        without_includes(ATLAS_CONTROLS),
        without_includes(ATLAS_APPLET)
    )
}

#[must_use]
pub fn model_atlas_file_applet_source() -> String {
    format!(
        "{CAMERA_SOURCE}\n{GEOMETRY_SOURCE}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        without_includes(FILE_ATLAS_HEADER),
        without_includes(FILE_ATLAS_CATALOG),
        without_includes(ATLAS_SCAN),
        without_includes(ATLAS_INTERCHANGE),
        without_includes(ATLAS_FIXTURE),
        without_includes(ATLAS_DETAIL_FIXTURE),
        without_includes(ATLAS_ARCHITECTURE),
        without_includes(ATLAS_MODEL),
        without_includes(ATLAS_SCENE),
        without_includes(FILE_ATLAS_MENU),
        without_includes(FILE_ATLAS_MODEL),
        without_includes(FILE_ATLAS_SCENE),
        without_includes(FILE_ATLAS_APPLET)
    )
}

#[must_use]
pub fn disk_usage_applet_source(snapshot: &crate::disk_usage::DiskUsageSnapshot) -> String {
    format!(
        "{}\n{CAMERA_SOURCE}\n{GEOMETRY_SOURCE}\n{}\n{}\n{}\n{}\n{}",
        snapshot.to_mlpl_binding(),
        without_includes(ATLAS_MODEL),
        without_includes(ATLAS_SCENE),
        without_includes(DISK_USAGE_MODEL),
        without_includes(DISK_USAGE_SCENE),
        without_includes(DISK_USAGE_APPLET)
    )
}

#[must_use]
pub fn audio_spectrum_applet_source(paths: &[String]) -> String {
    let files = paths
        .iter()
        .map(|path| format!("\"{}\"", path.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "audio_files=[{files}];\n{CAMERA_SOURCE}\n{GEOMETRY_SOURCE}\n{}\n{}\n{}\n{}\n{}",
        without_includes(ATLAS_MODEL),
        without_includes(ATLAS_SCENE),
        without_includes(AUDIO_MODEL),
        without_includes(AUDIO_SCENE),
        without_includes(AUDIO_APPLET)
    )
}

#[derive(Debug)]
pub struct SceneCommand {
    pub scene: LineScene,
    pub objects: BTreeMap<u64, LineObject>,
    pub camera: Camera,
    pub rotation_speed: f32,
    pub revision: u64,
    pub help: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineObject {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 4],
    pub thickness: f32,
}

#[derive(Clone, Debug)]
pub struct ScenePatchCommand {
    pub base_revision: u64,
    pub target_revision: u64,
    pub upserts: BTreeMap<u64, LineObject>,
    pub remove_ids: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetainedScene {
    scene: LineScene,
    objects: BTreeMap<u64, LineObject>,
    revision: u64,
    rotation_speed: f32,
}

impl RetainedScene {
    /// Creates retained ID-addressed state from a validated complete scene.
    ///
    /// # Errors
    ///
    /// Returns an error if the validated command cannot initialize retained state.
    pub fn from_scene_command(command: &SceneCommand) -> Result<Self, String> {
        Ok(Self {
            scene: command.scene.clone(),
            objects: command.objects.clone(),
            revision: command.revision,
            rotation_speed: command.rotation_speed,
        })
    }

    /// Applies a complete patch transaction or leaves the scene unchanged.
    ///
    /// # Errors
    ///
    /// Rejects stale revisions, unknown removals, or a candidate scene that
    /// violates generic line-scene bounds.
    pub fn apply(&mut self, patch: &ScenePatchCommand) -> Result<(), String> {
        if patch.base_revision != self.revision || patch.target_revision <= patch.base_revision {
            return Err("scene patch revision mismatch".into());
        }
        let mut next = self.objects.clone();
        for id in &patch.remove_ids {
            if next.remove(id).is_none() {
                return Err("scene patch removes an unknown id".into());
            }
        }
        for (id, line) in &patch.upserts {
            next.insert(*id, line.clone());
        }
        let scene = scene_from_objects(&next, self.rotation_speed)?;
        self.objects = next;
        self.scene = scene;
        self.revision = patch.target_revision;
        Ok(())
    }

    /// Advances the shared visual revision for a geometry-preserving view update.
    ///
    /// # Errors
    ///
    /// Rejects a view revision older than the retained scene revision.
    pub fn apply_view_revision(&mut self, revision: u64) -> Result<(), String> {
        if revision < self.revision {
            return Err("view revision is stale".into());
        }
        self.revision = revision;
        Ok(())
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub const fn scene(&self) -> &LineScene {
        &self.scene
    }
}

#[derive(Debug)]
pub struct ViewCommand {
    pub camera: Camera,
    pub revision: u64,
    pub help: String,
    pub status: String,
}

#[derive(Debug)]
pub enum LiveCommand {
    Scene(SceneCommand),
    Patch(ScenePatchCommand),
    View(ViewCommand),
    FrameAck(u64),
    AudioOpen(String),
    AudioPlay(bool),
    AudioSeek(f64),
    AudioAck,
}

#[must_use]
pub fn audio_chunk_event(chunk: &crate::audio::PcmChunk) -> Value {
    record([
        ("kind", Value::Str("audio_chunk".into())),
        (
            "left",
            Value::Array(DenseArray::from_vec(chunk.left.clone())),
        ),
        (
            "right",
            Value::Array(DenseArray::from_vec(chunk.right.clone())),
        ),
        ("sample_rate_hz", scalar(f64::from(chunk.sample_rate_hz))),
        (
            "start_frame",
            scalar(f64::from(
                u32::try_from(chunk.start_frame).unwrap_or(u32::MAX),
            )),
        ),
    ])
}

#[must_use]
pub fn audio_error_event(message: impl Into<String>) -> Value {
    record([
        ("kind", Value::Str("audio_error".into())),
        ("message", Value::Str(message.into())),
    ])
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

#[must_use]
pub fn resync_event() -> Value {
    record([("kind", Value::Str("resync".into()))])
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
        "patch_scene" => parse_scene_patch_fields(&fields).map(LiveCommand::Patch),
        "set_view" => parse_view_fields(&fields).map(LiveCommand::View),
        "frame_ack" => parse_revision(&fields).map(LiveCommand::FrameAck),
        "audio_open" => string_field(&fields, "path")
            .map(ToOwned::to_owned)
            .map(LiveCommand::AudioOpen),
        "audio_play" => scalar_field(&fields, "playing")
            .and_then(|value| match value {
                0.0 => Ok(false),
                1.0 => Ok(true),
                _ => Err("audio playing must be zero or one".into()),
            })
            .map(LiveCommand::AudioPlay),
        "audio_seek" => scalar_field(&fields, "seconds").map(LiveCommand::AudioSeek),
        "audio_ack" => Ok(LiveCommand::AudioAck),
        _ => Err("unsupported live command".into()),
    }
}

/// Decodes one bounded, versioned ID-addressed line patch.
///
/// # Errors
///
/// Rejects malformed records, invalid arrays or styles, duplicate/conflicting
/// IDs, non-advancing revisions, and descriptors above the operation bound.
pub fn parse_scene_patch_command(value: Value) -> Result<ScenePatchCommand, String> {
    let fields = record_fields(value, "command")?;
    if string_field(&fields, "op")? != "patch_scene" {
        return Err("unsupported live command".into());
    }
    parse_scene_patch_fields(&fields)
}

fn parse_scene_patch_fields(fields: &BTreeMap<String, Value>) -> Result<ScenePatchCommand, String> {
    const MAX_PATCH_LINES: usize = 100_000;
    let base_revision = parse_named_revision(fields, "base_revision")?;
    let target_revision = parse_named_revision(fields, "target_revision")?;
    if target_revision <= base_revision {
        return Err("target_revision must advance".into());
    }
    let ids_raw = vector_field(fields, "ids")?;
    let remove_raw = vector_field(fields, "remove_ids")?;
    if ids_raw.len().saturating_add(remove_raw.len()) > MAX_PATCH_LINES {
        return Err("scene patch exceeds line budget".into());
    }
    let starts = array_field(fields, "starts", &[ids_raw.len(), 3])?;
    let ends = array_field(fields, "ends", &[ids_raw.len(), 3])?;
    let colors = array_field(fields, "colors", &[ids_raw.len(), 4])?;
    let thicknesses = array_field(fields, "thicknesses", &[ids_raw.len()])?;
    let ids = parse_unique_ids(&ids_raw, "id")?;
    let remove_ids = parse_unique_ids(&remove_raw, "remove id")?;
    if ids.iter().any(|id| remove_ids.contains(id)) {
        return Err("scene patch cannot upsert and remove the same id".into());
    }
    let mut upserts = BTreeMap::new();
    for (row, id) in ids.iter().copied().enumerate() {
        let start = triple(&starts[row * 3..row * 3 + 3], "start")?;
        let end = triple(&ends[row * 3..row * 3 + 3], "end")?;
        let color = color4(&colors[row * 4..row * 4 + 4])?;
        let thickness = to_f32(thicknesses[row], "thickness")?;
        let line = LineObject {
            start,
            end,
            color,
            thickness,
        };
        validate_line_object(&line)?;
        upserts.insert(id, line);
    }
    Ok(ScenePatchCommand {
        base_revision,
        target_revision,
        upserts,
        remove_ids,
    })
}

fn parse_scene_fields(fields: &BTreeMap<String, Value>) -> Result<SceneCommand, String> {
    let (positions, _position_shape) = matrix_field(fields, "positions", 3)?;
    let (raw_edges, edge_shape) = matrix_field(fields, "edges", 2)?;
    let edge_count = edge_shape[0];
    let colors = array_field(fields, "colors", &[edge_count, 4])?;
    let thicknesses = array_field(fields, "thicknesses", &[edge_count])?;
    let ids = array_field(fields, "ids", &[edge_count])?;
    let parsed_ids = parse_unique_ids(&ids, "id")?;
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
    let positions = positions
        .into_iter()
        .map(|value| to_f32(value, "position"))
        .collect::<Result<Vec<_>, _>>()?;
    let objects = parsed_ids
        .into_iter()
        .enumerate()
        .map(|(row, id)| {
            let edge = &edges[row * 2..row * 2 + 2];
            (
                id,
                LineObject {
                    start: [
                        positions[edge[0] * 3],
                        positions[edge[0] * 3 + 1],
                        positions[edge[0] * 3 + 2],
                    ],
                    end: [
                        positions[edge[1] * 3],
                        positions[edge[1] * 3 + 1],
                        positions[edge[1] * 3 + 2],
                    ],
                    color: colors[row],
                    thickness: thicknesses[row],
                },
            )
        })
        .collect();
    let scene =
        LineScene::from_parallel_arrays(positions, edges, rotation_speed, colors, thicknesses)
            .map_err(|error| format!("invalid live scene: {error:?}"))?;
    Ok(SceneCommand {
        scene,
        objects,
        camera: view.camera,
        rotation_speed,
        revision: view.revision,
        help: view.help,
        status: view.status,
    })
}

fn parse_view_fields(fields: &BTreeMap<String, Value>) -> Result<ViewCommand, String> {
    let revision = parse_revision(fields)?;
    let help = string_field(fields, "help")?.to_owned();
    let status = match fields.get("status") {
        None => String::new(),
        Some(Value::Str(value)) => value.clone(),
        Some(_) => return Err("status must be a string".into()),
    };
    let camera = match fields.get("camera") {
        None => Camera::default(),
        Some(Value::Record { fields }) => parse_camera(fields)?,
        Some(_) => return Err("camera must be a record".into()),
    };
    Ok(ViewCommand {
        camera,
        revision,
        help,
        status,
    })
}

fn parse_revision(fields: &BTreeMap<String, Value>) -> Result<u64, String> {
    parse_named_revision(fields, "revision")
}

fn parse_named_revision(fields: &BTreeMap<String, Value>, name: &str) -> Result<u64, String> {
    u64::try_from(numeric_index(scalar_field(fields, name)?, name)?)
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

fn parse_unique_ids(values: &[f64], name: &str) -> Result<Vec<u64>, String> {
    let mut seen = BTreeSet::new();
    let mut ids = Vec::with_capacity(values.len());
    for value in values {
        let id = u64::try_from(numeric_index(*value, name)?)
            .map_err(|_| format!("{name} is out of range"))?;
        if !seen.insert(id) {
            return Err(format!("duplicate {name}"));
        }
        ids.push(id);
    }
    Ok(ids)
}

fn vector_field(fields: &BTreeMap<String, Value>, name: &str) -> Result<Vec<f64>, String> {
    let Value::Array(array) = fields.get(name).ok_or_else(|| format!("missing {name}"))? else {
        return Err(format!("{name} must be an array"));
    };
    if array.shape().dims().len() != 1 {
        return Err(format!("{name} has the wrong shape"));
    }
    Ok(array.data().to_vec())
}

fn triple(values: &[f64], name: &str) -> Result<[f32; 3], String> {
    Ok([
        to_f32(values[0], name)?,
        to_f32(values[1], name)?,
        to_f32(values[2], name)?,
    ])
}

fn color4(values: &[f64]) -> Result<[f32; 4], String> {
    Ok([
        to_f32(values[0], "color")?,
        to_f32(values[1], "color")?,
        to_f32(values[2], "color")?,
        to_f32(values[3], "color")?,
    ])
}

fn validate_line_object(line: &LineObject) -> Result<(), String> {
    LineScene::from_parallel_arrays(
        line.start.into_iter().chain(line.end).collect(),
        vec![0, 1],
        0.0,
        vec![line.color],
        vec![line.thickness],
    )
    .map(|_| ())
    .map_err(|error| format!("invalid patched line: {error:?}"))
}

fn scene_from_objects(
    objects: &BTreeMap<u64, LineObject>,
    rotation_speed: f32,
) -> Result<LineScene, String> {
    if objects.is_empty() {
        return Err("retained scene cannot be empty".into());
    }
    let mut positions = Vec::with_capacity(objects.len() * 6);
    let mut edges = Vec::with_capacity(objects.len() * 2);
    let mut colors = Vec::with_capacity(objects.len());
    let mut thicknesses = Vec::with_capacity(objects.len());
    for (index, line) in objects.values().enumerate() {
        positions.extend(line.start);
        positions.extend(line.end);
        edges.extend([index * 2, index * 2 + 1]);
        colors.push(line.color);
        thicknesses.push(line.thickness);
    }
    LineScene::from_parallel_arrays(positions, edges, rotation_speed, colors, thicknesses)
        .map_err(|error| format!("invalid retained scene: {error:?}"))
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
