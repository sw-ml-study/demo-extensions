//! Generic headless native-3D primitives for MLPL applications.

use std::cell::RefCell;
use std::collections::BTreeMap;

use mlpl_extension_sdk::{DenseArray, HandleRegistry, NativeHandle, OwnedError, Value};
use mlpl_native3d_scene::{BoxLimits, BoxScene, Camera, Projection, Viewport};

const EXTENSION_ID: u64 = 0x4E_33_44_01;
const VIEWER_TYPE: u64 = 1;
const MAX_VIEWERS: usize = 64;
const MAX_VERTICES: usize = 1_000_000;
const MAX_LINES: usize = 2_000_000;
const MAX_BOXES: usize = 100_000;

thread_local! {
    static VIEWERS: RefCell<HandleRegistry> = const {
        RefCell::new(HandleRegistry::with_limits(EXTENSION_ID, MAX_VIEWERS, u32::MAX))
    };
}

#[derive(Debug)]
struct Viewer {
    width: u32,
    height: u32,
    scene: Option<LineScene>,
    boxes: Option<BoxScene>,
    camera: Camera,
    selected_id: Option<u64>,
    frame: u64,
    rotation_y: f64,
}

#[derive(Debug)]
struct LineScene {
    positions: DenseArray,
    edges: DenseArray,
    colors: DenseArray,
    thicknesses: DenseArray,
    ids: DenseArray,
}

const METADATA: &str = r#"
[[functions]]
name = "create_viewer"
documentation = "Create one headless viewer resource with a logical drawable size."
returns = "native<Viewer>"
[[functions.arguments]]
name = "width"
type = "f64"
[[functions.arguments]]
name = "height"
type = "f64"

[[functions]]
name = "set_lines"
documentation = "Replace a viewer's generic line scene using bulk arrays."
returns = "record"
[[functions.arguments]]
name = "viewer"
type = "native<Viewer>"
[[functions.arguments]]
name = "positions"
type = "array<f64>[N,3]"
[[functions.arguments]]
name = "edges"
type = "array<f64>[M,2]"
[[functions.arguments]]
name = "colors"
type = "array<f64>[M,4]"
[[functions.arguments]]
name = "thicknesses"
type = "array<f64>[M]"
[[functions.arguments]]
name = "ids"
type = "array<f64>[M]"

[[functions]]
name = "viewer_state"
documentation = "Return deterministic headless viewer state."
returns = "record"
[[functions.arguments]]
name = "viewer"
type = "native<Viewer>"

[[functions]]
name = "set_boxes"
documentation = "Replace a viewer's generic filled-box scene using parallel bulk arrays."
returns = "record"
[[functions.arguments]]
name = "viewer"
type = "native<Viewer>"
[[functions.arguments]]
name = "centers"
type = "array<f64>[N,3]"
[[functions.arguments]]
name = "sizes"
type = "array<f64>[N,3]"
[[functions.arguments]]
name = "colors"
type = "array<f64>[N,4]"
[[functions.arguments]]
name = "ids"
type = "array<f64>[N]"

[[functions]]
name = "set_view"
documentation = "Set a generic perspective or orthographic camera. Scale means fov radians or vertical world span."
returns = "record"
[[functions.arguments]]
name = "viewer"
type = "native<Viewer>"
[[functions.arguments]]
name = "projection"
type = "string"
[[functions.arguments]]
name = "target"
type = "array<f64>[3]"
[[functions.arguments]]
name = "yaw"
type = "f64"
[[functions.arguments]]
name = "pitch"
type = "f64"
[[functions.arguments]]
name = "distance"
type = "f64"
[[functions.arguments]]
name = "scale"
type = "f64"
[[functions.arguments]]
name = "near"
type = "f64"

[[functions]]
name = "pick_box"
documentation = "Pick the nearest stable box ID at a physical-pixel coordinate."
returns = "record"
[[functions.arguments]]
name = "viewer"
type = "native<Viewer>"
[[functions.arguments]]
name = "x"
type = "f64"
[[functions.arguments]]
name = "y"
type = "f64"
[[functions.arguments]]
name = "rotation_y"
type = "f64"

[[functions]]
name = "set_selection"
documentation = "Set or clear the generic highlighted stable box ID."
returns = "record"
[[functions.arguments]]
name = "viewer"
type = "native<Viewer>"
[[functions.arguments]]
name = "id"
type = "f64|nil"

[[functions]]
name = "viewer_size"
documentation = "Return the logical drawable width and height."
returns = "record"
[[functions.arguments]]
name = "viewer"
type = "native<Viewer>"

[[functions]]
name = "render"
documentation = "Record explicit rotation state and advance one headless frame."
returns = "record"
[[functions.arguments]]
name = "viewer"
type = "native<Viewer>"
[[functions.arguments]]
name = "rotation_y"
type = "f64"

[[functions]]
name = "close"
documentation = "Close and invalidate one viewer resource."
returns = "bool"
[[functions.arguments]]
name = "viewer"
type = "native<Viewer>"

[[types]]
name = "Viewer"
documentation = "Opaque generational handle to a generic native3d viewer."
"#;

fn create_viewer(arguments: &[Value]) -> Result<Value, OwnedError> {
    let width = dimension(argument(arguments, 0, "width")?, "width")?;
    let height = dimension(argument(arguments, 1, "height")?, "height")?;
    VIEWERS.with_borrow_mut(|viewers| {
        viewers
            .insert(
                VIEWER_TYPE,
                Viewer {
                    width,
                    height,
                    scene: None,
                    boxes: None,
                    camera: Camera::default(),
                    selected_id: None,
                    frame: 0,
                    rotation_y: 0.0,
                },
            )
            .map(Value::Handle)
            .map_err(handle_error)
    })
}

fn set_lines(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    let scene = LineScene::from_arguments(arguments)?;
    let vertices = scene.vertices();
    let lines = scene.lines();
    VIEWERS.with_borrow_mut(|viewers| {
        viewers
            .get_mut::<Viewer>(handle, VIEWER_TYPE)
            .map_err(handle_error)?
            .scene = Some(scene);
        Ok(scene_record(vertices, lines))
    })
}

fn set_boxes(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    let centers = array_argument(arguments, 1, "centers")?;
    let sizes = array_argument(arguments, 2, "sizes")?;
    let colors = array_argument(arguments, 3, "colors")?;
    let ids = array_argument(arguments, 4, "ids")?;
    let count = matrix(centers, 3, MAX_BOXES, "centers")?;
    matrix_exact(sizes, count, 3, "sizes")?;
    matrix_exact(colors, count, 4, "colors")?;
    vector_exact(ids, count, "ids")?;
    let centers = f32_values(centers, "centers")?;
    let sizes = f32_values(sizes, "sizes")?;
    let colors = f32_values(colors, "colors")?
        .chunks_exact(4)
        .map(|rgba| [rgba[0], rgba[1], rgba[2], rgba[3]])
        .collect();
    let ids = ids
        .view()
        .as_f64()
        .map_err(array_error)?
        .iter()
        .map(|value| exact_id(*value, "ids"))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = BoxLimits::new(MAX_BOXES, MAX_BOXES * 48)
        .map_err(|error| OwnedError::extension(format!("invalid box limits: {error:?}")))?;
    let scene = BoxScene::from_parallel_arrays(centers, sizes, colors, ids, limits)
        .map_err(|error| OwnedError::invalid_argument(format!("invalid box scene: {error:?}")))?;
    VIEWERS.with_borrow_mut(|viewers| {
        let viewer = viewers
            .get_mut::<Viewer>(handle, VIEWER_TYPE)
            .map_err(handle_error)?;
        viewer.boxes = Some(scene);
        viewer.selected_id = None;
        Ok(record([("boxes", number(&count))]))
    })
}

fn set_view(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    let Value::String(projection) = argument(arguments, 1, "projection")? else {
        return Err(OwnedError::invalid_argument("projection must be a string"));
    };
    let target = array_argument(arguments, 2, "target")?;
    vector_exact(target, 3, "target")?;
    let target = f32_values(target, "target")?;
    let target = [target[0], target[1], target[2]];
    let yaw = finite_f32(argument(arguments, 3, "yaw")?, "yaw")?;
    let pitch = finite_f32(argument(arguments, 4, "pitch")?, "pitch")?;
    let distance = finite_f32(argument(arguments, 5, "distance")?, "distance")?;
    let scale = finite_f32(argument(arguments, 6, "scale")?, "scale")?;
    let near = finite_f32(argument(arguments, 7, "near")?, "near")?;
    let camera = match projection.as_str() {
        "perspective" => Camera::orbit(target, yaw, pitch, distance, scale, near),
        "orthographic" => Camera::orthographic(target, yaw, pitch, distance, scale, near),
        _ => return Err(OwnedError::invalid_argument("unsupported projection")),
    }
    .map_err(|error| OwnedError::invalid_argument(format!("invalid camera: {error:?}")))?;
    VIEWERS.with_borrow_mut(|viewers| {
        viewers
            .get_mut::<Viewer>(handle, VIEWER_TYPE)
            .map_err(handle_error)?
            .camera = camera;
        Ok(record([
            ("projection", Value::String(projection.clone())),
            ("scale", Value::F64(f64::from(scale))),
        ]))
    })
}

fn pick_box(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    let x = finite_f32(argument(arguments, 1, "x")?, "x")?;
    let y = finite_f32(argument(arguments, 2, "y")?, "y")?;
    let rotation = finite_f32(argument(arguments, 3, "rotation_y")?, "rotation_y")?;
    VIEWERS.with_borrow(|viewers| {
        let viewer = viewers
            .get::<Viewer>(handle, VIEWER_TYPE)
            .map_err(handle_error)?;
        let scene = viewer
            .boxes
            .as_ref()
            .ok_or_else(|| OwnedError::invalid_argument("viewer requires set_boxes before pick"))?;
        let viewport = Viewport::new(viewer.width, viewer.height)
            .map_err(|error| OwnedError::extension(format!("invalid viewport: {error:?}")))?;
        let ray = viewer
            .camera
            .pick_ray(viewport, [x, y])
            .map_err(|error| OwnedError::invalid_argument(format!("invalid pick: {error:?}")))?;
        let hit = scene
            .pick(ray, rotation)
            .map_err(|error| OwnedError::invalid_argument(format!("invalid pick: {error:?}")))?;
        Ok(hit.map_or_else(
            || {
                record([
                    ("hit", Value::Bool(false)),
                    ("id", Value::Nil),
                    ("distance", Value::Nil),
                ])
            },
            |hit| {
                record([
                    ("hit", Value::Bool(true)),
                    ("id", number(&hit.id())),
                    ("distance", Value::F64(f64::from(hit.distance()))),
                ])
            },
        ))
    })
}

fn set_selection(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    let selected_id = match argument(arguments, 1, "id")? {
        Value::Nil => None,
        value => Some(exact_id(finite_number(value, "id")?, "id")?),
    };
    VIEWERS.with_borrow_mut(|viewers| {
        let viewer = viewers
            .get_mut::<Viewer>(handle, VIEWER_TYPE)
            .map_err(handle_error)?;
        let scene = viewer.boxes.as_ref().ok_or_else(|| {
            OwnedError::invalid_argument("viewer requires set_boxes before selection")
        })?;
        scene.validate_selection(selected_id).map_err(|error| {
            OwnedError::invalid_argument(format!("invalid selection: {error:?}"))
        })?;
        viewer.selected_id = selected_id;
        Ok(record([(
            "selected_id",
            selected_id.map_or(Value::Nil, |id| number(&id)),
        )]))
    })
}

fn viewer_state(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    VIEWERS.with_borrow(|viewers| {
        let viewer = viewers
            .get::<Viewer>(handle, VIEWER_TYPE)
            .map_err(handle_error)?;
        let (vertices, lines) = viewer
            .scene
            .as_ref()
            .map_or((0, 0), |scene| (scene.vertices(), scene.lines()));
        let projection = match viewer.camera.projection() {
            Projection::Perspective => "perspective",
            Projection::Orthographic { .. } => "orthographic",
        };
        Ok(record([
            ("vertices", number(&vertices)),
            ("lines", number(&lines)),
            ("frame", number(&viewer.frame)),
            ("rotation_y", Value::F64(viewer.rotation_y)),
            ("configured", Value::Bool(viewer.scene.is_some())),
            (
                "boxes",
                number(&viewer.boxes.as_ref().map_or(0, BoxScene::len)),
            ),
            ("projection", Value::String(projection.into())),
            (
                "selected_id",
                viewer.selected_id.map_or(Value::Nil, |id| number(&id)),
            ),
        ]))
    })
}

fn viewer_size(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    VIEWERS.with_borrow(|viewers| {
        let viewer = viewers
            .get::<Viewer>(handle, VIEWER_TYPE)
            .map_err(handle_error)?;
        Ok(record([
            ("width", number(&viewer.width)),
            ("height", number(&viewer.height)),
        ]))
    })
}

fn render(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    let rotation = finite_number(argument(arguments, 1, "rotation_y")?, "rotation_y")?;
    VIEWERS.with_borrow_mut(|viewers| {
        let viewer = viewers
            .get_mut::<Viewer>(handle, VIEWER_TYPE)
            .map_err(handle_error)?;
        if viewer.scene.is_none() && viewer.boxes.is_none() {
            return Err(OwnedError::invalid_argument(
                "viewer requires set_lines or set_boxes before render",
            ));
        }
        viewer.frame = viewer
            .frame
            .checked_add(1)
            .ok_or_else(|| OwnedError::extension("frame counter exhausted"))?;
        viewer.rotation_y = rotation;
        Ok(record([
            ("frame", number(&viewer.frame)),
            ("rotation_y", Value::F64(rotation)),
        ]))
    })
}

fn close(arguments: &[Value]) -> Result<Value, OwnedError> {
    let handle = handle_argument(arguments)?;
    VIEWERS.with_borrow_mut(|viewers| {
        viewers
            .remove::<Viewer>(handle, VIEWER_TYPE)
            .map(|_| Value::Bool(true))
            .map_err(handle_error)
    })
}

impl LineScene {
    fn from_arguments(arguments: &[Value]) -> Result<Self, OwnedError> {
        let positions = array_argument(arguments, 1, "positions")?.clone();
        let edges = array_argument(arguments, 2, "edges")?.clone();
        let colors = array_argument(arguments, 3, "colors")?.clone();
        let thicknesses = array_argument(arguments, 4, "thicknesses")?.clone();
        let ids = array_argument(arguments, 5, "ids")?.clone();

        let vertices = matrix(&positions, 3, MAX_VERTICES, "positions")?;
        let lines = matrix(&edges, 2, MAX_LINES, "edges")?;
        matrix_exact(&colors, lines, 4, "colors")?;
        vector_exact(&thicknesses, lines, "thicknesses")?;
        vector_exact(&ids, lines, "ids")?;

        validate_finite(&positions, "positions")?;
        validate_indices(&edges, vertices)?;
        validate_colors(&colors)?;
        validate_positive(&thicknesses, "thicknesses")?;
        validate_ids(&ids)?;

        Ok(Self {
            positions,
            edges,
            colors,
            thicknesses,
            ids,
        })
    }

    fn vertices(&self) -> usize {
        self.positions.view().shape()[0]
    }

    fn lines(&self) -> usize {
        debug_assert_eq!(self.colors.view().shape()[0], self.edges.view().shape()[0]);
        debug_assert_eq!(
            self.thicknesses.view().shape()[0],
            self.edges.view().shape()[0]
        );
        debug_assert_eq!(self.ids.view().shape()[0], self.edges.view().shape()[0]);
        self.edges.view().shape()[0]
    }
}

fn matrix(
    array: &DenseArray,
    columns: usize,
    maximum: usize,
    name: &str,
) -> Result<usize, OwnedError> {
    let shape = array.view().shape();
    if shape.len() != 2 || shape[1] != columns || shape[0] == 0 || shape[0] > maximum {
        return Err(OwnedError::invalid_argument(format!(
            "{name} must have shape [N,{columns}] with bounded nonzero N"
        )));
    }
    array.view().as_f64().map_err(array_error)?;
    Ok(shape[0])
}

fn matrix_exact(
    array: &DenseArray,
    rows: usize,
    columns: usize,
    name: &str,
) -> Result<(), OwnedError> {
    if array.view().shape() != [rows, columns] {
        return Err(OwnedError::invalid_argument(format!(
            "{name} must have shape [{rows},{columns}]"
        )));
    }
    array.view().as_f64().map_err(array_error)?;
    Ok(())
}

fn vector_exact(array: &DenseArray, length: usize, name: &str) -> Result<(), OwnedError> {
    if array.view().shape() != [length] {
        return Err(OwnedError::invalid_argument(format!(
            "{name} must have shape [{length}]"
        )));
    }
    array.view().as_f64().map_err(array_error)?;
    Ok(())
}

fn validate_finite(array: &DenseArray, name: &str) -> Result<(), OwnedError> {
    if array
        .view()
        .as_f64()
        .map_err(array_error)?
        .iter()
        .all(|value| value.is_finite())
    {
        Ok(())
    } else {
        Err(OwnedError::invalid_argument(format!(
            "{name} values must be finite"
        )))
    }
}

fn validate_indices(array: &DenseArray, vertices: usize) -> Result<(), OwnedError> {
    let vertex_limit = number_as_f64(&vertices);
    if array
        .view()
        .as_f64()
        .map_err(array_error)?
        .iter()
        .all(|value| {
            value.is_finite() && *value >= 0.0 && value.fract() == 0.0 && *value < vertex_limit
        })
    {
        Ok(())
    } else {
        Err(OwnedError::invalid_argument(
            "edges contain an invalid vertex index",
        ))
    }
}

fn validate_colors(array: &DenseArray) -> Result<(), OwnedError> {
    if array
        .view()
        .as_f64()
        .map_err(array_error)?
        .iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
    {
        Ok(())
    } else {
        Err(OwnedError::invalid_argument(
            "colors must be finite RGBA values in 0..=1",
        ))
    }
}

fn validate_positive(array: &DenseArray, name: &str) -> Result<(), OwnedError> {
    if array
        .view()
        .as_f64()
        .map_err(array_error)?
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
    {
        Ok(())
    } else {
        Err(OwnedError::invalid_argument(format!(
            "{name} values must be finite and positive"
        )))
    }
}

fn validate_ids(array: &DenseArray) -> Result<(), OwnedError> {
    if array
        .view()
        .as_f64()
        .map_err(array_error)?
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0 && value.fract() == 0.0)
    {
        Ok(())
    } else {
        Err(OwnedError::invalid_argument(
            "ids must be nonnegative integers",
        ))
    }
}

fn argument<'a>(arguments: &'a [Value], index: usize, name: &str) -> Result<&'a Value, OwnedError> {
    arguments
        .get(index)
        .ok_or_else(|| OwnedError::invalid_argument(format!("missing {name}")))
}

fn array_argument<'a>(
    arguments: &'a [Value],
    index: usize,
    name: &str,
) -> Result<&'a DenseArray, OwnedError> {
    match argument(arguments, index, name)? {
        Value::Array(array) => Ok(array),
        _ => Err(OwnedError::invalid_argument(format!(
            "{name} must be a dense array"
        ))),
    }
}

fn handle_argument(arguments: &[Value]) -> Result<NativeHandle, OwnedError> {
    match argument(arguments, 0, "viewer")? {
        Value::Handle(handle) => Ok(*handle),
        _ => Err(OwnedError::invalid_argument(
            "viewer must be a native handle",
        )),
    }
}

fn finite_number(value: &Value, name: &str) -> Result<f64, OwnedError> {
    let number = match value {
        Value::F64(value) => *value,
        Value::I64(value) => number_as_f64(value),
        _ => {
            return Err(OwnedError::invalid_argument(format!(
                "{name} must be numeric"
            )));
        }
    };
    if number.is_finite() {
        Ok(number)
    } else {
        Err(OwnedError::invalid_argument(format!(
            "{name} must be finite"
        )))
    }
}

fn finite_f32(value: &Value, name: &str) -> Result<f32, OwnedError> {
    let value = finite_number(value, name)?;
    if value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return Err(OwnedError::invalid_argument(format!(
            "{name} is outside f32 range"
        )));
    }
    value
        .to_string()
        .parse::<f32>()
        .map_err(|_| OwnedError::invalid_argument(format!("{name} is outside f32 range")))
}

fn f32_values(array: &DenseArray, name: &str) -> Result<Vec<f32>, OwnedError> {
    array
        .view()
        .as_f64()
        .map_err(array_error)?
        .iter()
        .map(|value| finite_f32(&Value::F64(*value), name))
        .collect()
}

fn exact_id(value: f64, name: &str) -> Result<u64, OwnedError> {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    if value.is_finite() && (0.0..=MAX_SAFE_INTEGER).contains(&value) && value.fract() == 0.0 {
        value.to_string().parse::<u64>().map_err(|_| {
            OwnedError::invalid_argument(format!(
                "{name} must contain exact nonnegative integer IDs"
            ))
        })
    } else {
        Err(OwnedError::invalid_argument(format!(
            "{name} must contain exact nonnegative integer IDs"
        )))
    }
}

fn dimension(value: &Value, name: &str) -> Result<u32, OwnedError> {
    let number = finite_number(value, name)?;
    if number.fract() == 0.0 && (1.0..=16384.0).contains(&number) {
        number
            .to_string()
            .parse::<u32>()
            .map_err(|_| OwnedError::invalid_argument(format!("{name} is out of range")))
    } else {
        Err(OwnedError::invalid_argument(format!(
            "{name} must be an integer in 1..=16384"
        )))
    }
}

fn number(value: &impl ToString) -> Value {
    Value::F64(number_as_f64(value))
}

fn number_as_f64(value: &impl ToString) -> f64 {
    value
        .to_string()
        .parse::<f64>()
        .expect("bounded counters always convert to f64")
}

fn scene_record(vertices: usize, lines: usize) -> Value {
    record([("vertices", number(&vertices)), ("lines", number(&lines))])
}

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err transfers the conversion error into this formatter"
)]
fn array_error(error: mlpl_extension_sdk::ArrayError) -> OwnedError {
    OwnedError::invalid_argument(format!("invalid dense array: {error:?}"))
}

fn handle_error(error: mlpl_extension_sdk::HandleError) -> OwnedError {
    OwnedError::extension(format!("invalid viewer handle: {error:?}"))
}

mlpl_extension_sdk::export_extension! {
    module: generated_export,
    entry: sw_mlpl_extension_v1,
    name: "_native3d",
    version: "0.2.0",
    metadata: crate::METADATA,
    functions: [
        (create_viewer_trampoline, "create_viewer", 2, crate::create_viewer),
        (set_lines_trampoline, "set_lines", 6, crate::set_lines),
        (set_boxes_trampoline, "set_boxes", 5, crate::set_boxes),
        (set_view_trampoline, "set_view", 8, crate::set_view),
        (pick_box_trampoline, "pick_box", 4, crate::pick_box),
        (set_selection_trampoline, "set_selection", 2, crate::set_selection),
        (viewer_state_trampoline, "viewer_state", 1, crate::viewer_state),
        (viewer_size_trampoline, "viewer_size", 1, crate::viewer_size),
        (render_trampoline, "render", 2, crate::render),
        (close_trampoline, "close", 1, crate::close),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
