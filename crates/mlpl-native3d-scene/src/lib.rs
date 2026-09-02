//! Renderer-neutral line and point scene parsing, validation, and planning.

use serde::{Deserialize, Serialize};

mod interaction;
mod point;
mod renderer;

pub use interaction::{InteractionError, OrbitCamera, Ray3};
pub use point::{PlannedPoint, PointLimits, PointScene, PointSceneError, PointUploadPlan};
pub use renderer::{Camera, HeadlessImage, PlannedLine, RenderError, Viewport};

const SCHEMA: &str = "sw-ml-study.native3d.line-scene";
const MAX_VERTICES: usize = 1_000_000;
const MAX_EDGES: usize = 2_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineScene {
    schema: String,
    version: u32,
    positions: NumericArray,
    edges: IndexArray,
    controls: SceneControls,
    #[serde(skip)]
    line_styles: Option<Vec<([f32; 4], f32)>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NumericArray {
    shape: [usize; 2],
    values: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneControls {
    rotation_speed: f32,
    line_color: [f32; 4],
    line_thickness: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct IndexArray {
    shape: [usize; 2],
    values: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SceneError {
    Malformed,
    UnsupportedSchema,
    UnsupportedVersion(u32),
    PositionShape,
    PositionValue,
    EdgeShape,
    EdgeIndex { edge: usize, vertex: usize },
    RotationSpeed,
    LineColor,
    LineThickness,
}

impl LineScene {
    /// Builds one validated generic line scene from owned row-major arrays.
    ///
    /// # Errors
    ///
    /// Applies the same shape, bounds, finite-value, index, and style checks as
    /// JSON parsing.
    pub fn from_arrays(
        positions: Vec<f32>,
        edges: Vec<usize>,
        rotation_speed: f32,
        line_color: [f32; 4],
        line_thickness: f32,
    ) -> Result<Self, SceneError> {
        if positions.len().checked_rem(3) != Some(0) || edges.len().checked_rem(2) != Some(0) {
            return Err(SceneError::Malformed);
        }
        let scene = Self {
            schema: SCHEMA.to_owned(),
            version: 1,
            positions: NumericArray {
                shape: [positions.len() / 3, 3],
                values: positions,
            },
            edges: IndexArray {
                shape: [edges.len() / 2, 2],
                values: edges,
            },
            controls: SceneControls {
                rotation_speed,
                line_color,
                line_thickness,
            },
            line_styles: None,
        };
        scene.validate()?;
        Ok(scene)
    }

    /// Builds a line scene with one validated color and thickness per edge.
    ///
    /// # Errors
    ///
    /// Rejects mismatched parallel lengths, malformed geometry, out-of-range
    /// indices, non-finite values, colors outside RGBA bounds, and unsupported
    /// thickness or rotation values.
    pub fn from_parallel_arrays(
        positions: Vec<f32>,
        edges: Vec<usize>,
        rotation_speed: f32,
        colors: Vec<[f32; 4]>,
        thicknesses: Vec<f32>,
    ) -> Result<Self, SceneError> {
        let edge_count = edges.len() / 2;
        if colors.len() != edge_count || thicknesses.len() != edge_count || edge_count == 0 {
            return Err(SceneError::Malformed);
        }
        let mut scene =
            Self::from_arrays(positions, edges, rotation_speed, colors[0], thicknesses[0])?;
        for color in &colors {
            validate_color(*color)?;
        }
        for thickness in &thicknesses {
            validate_thickness(*thickness)?;
        }
        scene.line_styles = Some(colors.into_iter().zip(thicknesses).collect());
        Ok(scene)
    }

    pub(crate) fn style(&self, edge: usize) -> ([f32; 4], f32) {
        self.line_styles.as_ref().map_or(
            (self.controls.line_color, self.controls.line_thickness),
            |styles| styles[edge],
        )
    }

    /// Parses and validates one deterministic renderer-neutral line scene.
    ///
    /// # Errors
    ///
    /// Rejects malformed JSON, unsupported schema/version, inconsistent array
    /// shapes, non-finite values, invalid indices, and unsafe controls.
    pub fn parse(source: &str) -> Result<Self, SceneError> {
        let scene: Self = serde_json::from_str(source).map_err(|_| SceneError::Malformed)?;
        scene.validate()?;
        Ok(scene)
    }

    #[must_use]
    pub fn positions(&self) -> &NumericArray {
        &self.positions
    }

    #[must_use]
    pub fn edges(&self) -> Vec<[usize; 2]> {
        self.edges
            .values
            .chunks_exact(2)
            .map(|edge| [edge[0], edge[1]])
            .collect()
    }

    #[must_use]
    pub const fn controls(&self) -> &SceneControls {
        &self.controls
    }

    /// Returns canonical compact JSON in stable struct-field order.
    ///
    /// # Panics
    ///
    /// Serialization cannot fail for this closed, validated data model.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("validated scene is serializable")
    }

    /// Transforms, near-clips, projects, and viewport-clips generic scene edges.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid camera values or a non-finite rotation.
    pub fn plan_lines(
        &self,
        camera: Camera,
        viewport: Viewport,
        rotation_y: f32,
    ) -> Result<Vec<PlannedLine>, RenderError> {
        renderer::plan_lines(self, camera, viewport, rotation_y)
    }

    /// Produces a deterministic CPU-rasterized image for tests and evidence.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid camera values or a non-finite rotation.
    pub fn render_headless(
        &self,
        camera: Camera,
        viewport: Viewport,
        rotation_y: f32,
    ) -> Result<HeadlessImage, RenderError> {
        let lines = self.plan_lines(camera, viewport, rotation_y)?;
        Ok(renderer::rasterize(&lines, viewport))
    }

    fn validate(&self) -> Result<(), SceneError> {
        if self.schema != SCHEMA {
            return Err(SceneError::UnsupportedSchema);
        }
        if self.version != 1 {
            return Err(SceneError::UnsupportedVersion(self.version));
        }
        let vertices = validate_positions(&self.positions)?;
        validate_edges(&self.edges, vertices)?;
        self.controls.validate()
    }
}

impl NumericArray {
    /// Returns the declared row/column shape.
    #[must_use]
    pub const fn shape(&self) -> [usize; 2] {
        self.shape
    }

    /// Returns the row-major numeric storage.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }
}

impl SceneControls {
    /// Returns radians of rotation per second; negative values reverse direction.
    #[must_use]
    pub const fn rotation_speed(&self) -> f32 {
        self.rotation_speed
    }

    /// Returns linear RGBA channels in the inclusive range `0..=1`.
    #[must_use]
    pub const fn line_color(&self) -> [f32; 4] {
        self.line_color
    }

    /// Returns the requested line thickness in logical pixels.
    #[must_use]
    pub const fn line_thickness(&self) -> f32 {
        self.line_thickness
    }

    fn validate(&self) -> Result<(), SceneError> {
        if !self.rotation_speed.is_finite() || !(-10.0..=10.0).contains(&self.rotation_speed) {
            return Err(SceneError::RotationSpeed);
        }
        validate_color(self.line_color)?;
        validate_thickness(self.line_thickness)?;
        Ok(())
    }
}

fn validate_color(color: [f32; 4]) -> Result<(), SceneError> {
    if color
        .iter()
        .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
    {
        Ok(())
    } else {
        Err(SceneError::LineColor)
    }
}

fn validate_thickness(thickness: f32) -> Result<(), SceneError> {
    if thickness.is_finite() && (0.5..=20.0).contains(&thickness) {
        Ok(())
    } else {
        Err(SceneError::LineThickness)
    }
}

fn validate_positions(positions: &NumericArray) -> Result<usize, SceneError> {
    let [vertices, columns] = positions.shape;
    if columns != 3 || vertices == 0 || vertices > MAX_VERTICES {
        return Err(SceneError::PositionShape);
    }
    if vertices.checked_mul(columns) != Some(positions.values.len()) {
        return Err(SceneError::PositionShape);
    }
    if !positions.values.iter().all(|value| value.is_finite()) {
        return Err(SceneError::PositionValue);
    }
    Ok(vertices)
}

fn validate_edges(edges: &IndexArray, vertices: usize) -> Result<(), SceneError> {
    let [edge_count, columns] = edges.shape;
    if columns != 2 || edge_count == 0 || edge_count > MAX_EDGES {
        return Err(SceneError::EdgeShape);
    }
    if edge_count.checked_mul(columns) != Some(edges.values.len()) {
        return Err(SceneError::EdgeShape);
    }
    for (index, vertex) in edges.values.iter().copied().enumerate() {
        if vertex >= vertices {
            return Err(SceneError::EdgeIndex {
                edge: index / 2,
                vertex,
            });
        }
    }
    Ok(())
}
