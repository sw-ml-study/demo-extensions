//! Generic headless native-3D primitives for MLPL applications.

use std::cell::RefCell;
use std::collections::BTreeMap;

use mlpl_extension_sdk::{DenseArray, HandleRegistry, NativeHandle, OwnedError, Value};

const EXTENSION_ID: u64 = 0x4E_33_44_01;
const VIEWER_TYPE: u64 = 1;
const MAX_VIEWERS: usize = 64;
const MAX_VERTICES: usize = 1_000_000;
const MAX_LINES: usize = 2_000_000;

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
        Ok(record([
            ("vertices", number(&vertices)),
            ("lines", number(&lines)),
            ("frame", number(&viewer.frame)),
            ("rotation_y", Value::F64(viewer.rotation_y)),
            ("configured", Value::Bool(viewer.scene.is_some())),
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
        if viewer.scene.is_none() {
            return Err(OwnedError::invalid_argument(
                "viewer requires set_lines before render",
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
    version: "0.1.0",
    metadata: crate::METADATA,
    functions: [
        (create_viewer_trampoline, "create_viewer", 2, crate::create_viewer),
        (set_lines_trampoline, "set_lines", 6, crate::set_lines),
        (viewer_state_trampoline, "viewer_state", 1, crate::viewer_state),
        (viewer_size_trampoline, "viewer_size", 1, crate::viewer_size),
        (render_trampoline, "render", 2, crate::render),
        (close_trampoline, "close", 1, crate::close),
    ]
}

pub use sw_mlpl_extension_v1 as static_entry;
