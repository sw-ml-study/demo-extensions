//! Bounded renderer-neutral filled-box scenes and triangle planning.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{BoxRenderPlan, Camera, HeadlessImage, NumericArray, Ray3, RenderError, Viewport};

const BOX_SCHEMA: &str = "sw-ml-study.native3d.box-scene";
const BOX_VERSION: u32 = 1;
const MAX_BOX_LIMIT: usize = 100_000;
const OWNED_BYTES_PER_BOX: usize =
    3 * size_of::<f32>() + 3 * size_of::<f32>() + 4 * size_of::<f32>() + size_of::<u64>();
const TRIANGLE_BYTES: usize = 3 * (3 * size_of::<f32>() + 4 * size_of::<f32>() + size_of::<u64>());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoxLimits {
    max_boxes: usize,
    max_bytes: usize,
}

impl BoxLimits {
    /// Creates explicit nonzero source limits within the implementation cap.
    ///
    /// # Errors
    ///
    /// Rejects zero limits and counts above 100,000 boxes.
    pub const fn new(max_boxes: usize, max_bytes: usize) -> Result<Self, BoxSceneError> {
        if max_boxes == 0 || max_boxes > MAX_BOX_LIMIT || max_bytes == 0 {
            return Err(BoxSceneError::InvalidLimits);
        }
        Ok(Self {
            max_boxes,
            max_bytes,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoxScene {
    schema: String,
    version: u32,
    centers: NumericArray,
    sizes: NumericArray,
    colors: Vec<[f32; 4]>,
    ids: Vec<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxTriangle {
    vertices: [[f32; 3]; 3],
    color: [f32; 4],
    id: u64,
}

impl BoxTriangle {
    #[must_use]
    pub const fn vertices(self) -> [[f32; 3]; 3] {
        self.vertices
    }
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        self.color
    }
    #[must_use]
    pub const fn id(self) -> u64 {
        self.id
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoxTrianglePlan {
    triangles: Vec<BoxTriangle>,
    byte_len: usize,
}

/// The nearest generic box hit along a world-space ray.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxHit {
    id: u64,
    distance: f32,
}

impl BoxHit {
    #[must_use]
    pub const fn id(self) -> u64 {
        self.id
    }

    #[must_use]
    pub const fn distance(self) -> f32 {
        self.distance
    }
}

impl BoxTrianglePlan {
    #[must_use]
    pub fn triangles(&self) -> &[BoxTriangle] {
        &self.triangles
    }
    #[must_use]
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoxSceneError {
    Malformed,
    UnsupportedSchema,
    UnsupportedVersion(u32),
    InvalidLimits,
    CenterShape,
    CenterValue,
    SizeShape,
    BoxSize,
    ParallelLength,
    BoxColor,
    DuplicateId(u64),
    UnknownId(u64),
    BoxBudget { actual: usize, limit: usize },
    ByteBudget { actual: usize, limit: usize },
}

impl BoxScene {
    /// Builds one validated scene by taking ownership of all parallel arrays.
    ///
    /// # Errors
    ///
    /// Rejects invalid shapes, values, identities, and source budgets.
    pub fn from_parallel_arrays(
        centers: Vec<f32>,
        sizes: Vec<f32>,
        colors: Vec<[f32; 4]>,
        ids: Vec<u64>,
        limits: BoxLimits,
    ) -> Result<Self, BoxSceneError> {
        if centers.len().checked_rem(3) != Some(0) {
            return Err(BoxSceneError::CenterShape);
        }
        if sizes.len().checked_rem(3) != Some(0) {
            return Err(BoxSceneError::SizeShape);
        }
        let scene = Self {
            schema: BOX_SCHEMA.to_owned(),
            version: BOX_VERSION,
            centers: NumericArray {
                shape: [centers.len() / 3, 3],
                values: centers,
            },
            sizes: NumericArray {
                shape: [sizes.len() / 3, 3],
                values: sizes,
            },
            colors,
            ids,
        };
        scene.validate(limits)?;
        Ok(scene)
    }

    /// Parses JSON and copies every accepted array into owned Rust storage.
    ///
    /// # Errors
    ///
    /// Rejects malformed, unsupported, invalid, duplicate, or over-budget data.
    pub fn parse(source: &str, limits: BoxLimits) -> Result<Self, BoxSceneError> {
        let scene: Self = serde_json::from_str(source).map_err(|_| BoxSceneError::Malformed)?;
        scene.validate(limits)?;
        Ok(scene)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
    #[must_use]
    pub fn ids(&self) -> &[u64] {
        &self.ids
    }

    /// Replaces only the parallel colors while preserving validated geometry and IDs.
    ///
    /// # Errors
    ///
    /// Rejects a length mismatch or any non-finite/out-of-range RGBA component.
    pub fn recolored(&self, colors: Vec<[f32; 4]>) -> Result<Self, BoxSceneError> {
        let mut scene = self.clone();
        scene.colors = colors;
        scene.validate(BoxLimits {
            max_boxes: self.len().max(1),
            max_bytes: usize::MAX,
        })?;
        Ok(scene)
    }

    /// Replaces parallel centers and sizes while preserving colors and IDs.
    ///
    /// # Errors
    ///
    /// Rejects malformed, non-finite, nonpositive, or misaligned geometry.
    pub fn relayout(&self, centers: Vec<f32>, sizes: Vec<f32>) -> Result<Self, BoxSceneError> {
        Self::from_parallel_arrays(
            centers,
            sizes,
            self.colors.clone(),
            self.ids.clone(),
            BoxLimits {
                max_boxes: self.len().max(1),
                max_bytes: usize::MAX,
            },
        )
    }

    /// Validates an optional caller-owned selection against current stable IDs.
    ///
    /// # Errors
    ///
    /// Rejects an ID absent from this immutable scene.
    pub fn validate_selection(&self, selected_id: Option<u64>) -> Result<(), BoxSceneError> {
        if let Some(id) = selected_id
            && !self.ids.contains(&id)
        {
            return Err(BoxSceneError::UnknownId(id));
        }
        Ok(())
    }

    /// Returns the nearest filled-box intersection with a deterministic ID tie break.
    ///
    /// The scene rotation is inverted onto the ray, preserving axis-aligned slab
    /// tests without changing caller geometry.
    ///
    /// # Errors
    ///
    /// Rejects a non-finite scene rotation.
    pub fn pick(&self, ray: Ray3, rotation_y: f32) -> Result<Option<BoxHit>, RenderError> {
        if !rotation_y.is_finite() {
            return Err(RenderError::NonFiniteRotation);
        }
        let ray = inverse_rotate_ray(ray, rotation_y);
        let mut nearest: Option<BoxHit> = None;
        for (((center, size), _color), id) in self
            .centers
            .values
            .chunks_exact(3)
            .zip(self.sizes.values.chunks_exact(3))
            .zip(&self.colors)
            .zip(&self.ids)
        {
            let half = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
            let minimum = [
                center[0] - half[0],
                center[1] - half[1],
                center[2] - half[2],
            ];
            let maximum = [
                center[0] + half[0],
                center[1] + half[1],
                center[2] + half[2],
            ];
            if let Some(distance) = intersect_aabb(ray, minimum, maximum) {
                let candidate = BoxHit { id: *id, distance };
                if nearest.is_none_or(|current| {
                    distance.total_cmp(&current.distance).is_lt()
                        || (distance.total_cmp(&current.distance).is_eq() && *id < current.id)
                }) {
                    nearest = Some(candidate);
                }
            }
        }
        Ok(nearest)
    }

    /// Expands each box to twelve consistently wound owned triangles.
    ///
    /// # Errors
    ///
    /// Returns a byte-budget error if platform size arithmetic overflows.
    pub fn triangle_plan(&self) -> Result<BoxTrianglePlan, BoxSceneError> {
        let triangle_count = self
            .len()
            .checked_mul(12)
            .ok_or(BoxSceneError::ByteBudget {
                actual: usize::MAX,
                limit: usize::MAX - 1,
            })?;
        let byte_len =
            triangle_count
                .checked_mul(TRIANGLE_BYTES)
                .ok_or(BoxSceneError::ByteBudget {
                    actual: usize::MAX,
                    limit: usize::MAX - 1,
                })?;
        let mut triangles = Vec::with_capacity(triangle_count);
        for (((center, size), color), id) in self
            .centers
            .values
            .chunks_exact(3)
            .zip(self.sizes.values.chunks_exact(3))
            .zip(&self.colors)
            .zip(&self.ids)
        {
            append_box(&mut triangles, center, size, *color, *id);
        }
        Ok(BoxTrianglePlan {
            triangles,
            byte_len,
        })
    }

    /// Projects, culls, and far-to-near orders filled triangles.
    ///
    /// # Errors
    ///
    /// Rejects invalid camera state or non-finite rotation.
    pub fn plan_box_triangles(
        &self,
        camera: Camera,
        viewport: Viewport,
        rotation_y: f32,
    ) -> Result<BoxRenderPlan, RenderError> {
        crate::box_renderer::plan_boxes(self, camera, viewport, rotation_y)
    }

    /// Produces deterministic CPU-rasterized filled-box evidence.
    ///
    /// # Errors
    ///
    /// Rejects invalid camera state or non-finite rotation.
    pub fn render_boxes_headless(
        &self,
        camera: Camera,
        viewport: Viewport,
        rotation_y: f32,
    ) -> Result<HeadlessImage, RenderError> {
        Ok(crate::box_renderer::rasterize(
            &self.plan_box_triangles(camera, viewport, rotation_y)?,
            viewport,
        ))
    }

    fn validate(&self, limits: BoxLimits) -> Result<(), BoxSceneError> {
        if self.schema != BOX_SCHEMA {
            return Err(BoxSceneError::UnsupportedSchema);
        }
        if self.version != BOX_VERSION {
            return Err(BoxSceneError::UnsupportedVersion(self.version));
        }
        let [count, columns] = self.centers.shape;
        if count == 0 || columns != 3 || count.checked_mul(3) != Some(self.centers.values.len()) {
            return Err(BoxSceneError::CenterShape);
        }
        if count > limits.max_boxes {
            return Err(BoxSceneError::BoxBudget {
                actual: count,
                limit: limits.max_boxes,
            });
        }
        let bytes = count
            .checked_mul(OWNED_BYTES_PER_BOX)
            .ok_or(BoxSceneError::ByteBudget {
                actual: usize::MAX,
                limit: limits.max_bytes,
            })?;
        if bytes > limits.max_bytes {
            return Err(BoxSceneError::ByteBudget {
                actual: bytes,
                limit: limits.max_bytes,
            });
        }
        if !self.centers.values.iter().all(|value| value.is_finite()) {
            return Err(BoxSceneError::CenterValue);
        }
        if self.sizes.shape != [count, 3] || self.sizes.values.len() != count * 3 {
            return Err(BoxSceneError::SizeShape);
        }
        if !self
            .sizes
            .values
            .iter()
            .all(|value| value.is_finite() && *value > 0.0)
        {
            return Err(BoxSceneError::BoxSize);
        }
        if self.colors.len() != count || self.ids.len() != count {
            return Err(BoxSceneError::ParallelLength);
        }
        if !self
            .colors
            .iter()
            .flatten()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        {
            return Err(BoxSceneError::BoxColor);
        }
        let mut ids = BTreeSet::new();
        for id in &self.ids {
            if !ids.insert(*id) {
                return Err(BoxSceneError::DuplicateId(*id));
            }
        }
        Ok(())
    }
}

fn inverse_rotate_ray(ray: Ray3, angle: f32) -> Ray3 {
    let (sin, cos) = angle.sin_cos();
    let rotate = |value: [f32; 3]| {
        [
            value[0].mul_add(cos, -value[2] * sin),
            value[1],
            value[0].mul_add(sin, value[2] * cos),
        ]
    };
    Ray3::new(rotate(ray.origin()), rotate(ray.direction())).expect("rotation preserves valid ray")
}

fn intersect_aabb(ray: Ray3, minimum: [f32; 3], maximum: [f32; 3]) -> Option<f32> {
    let mut near = 0.0_f32;
    let mut far = f32::INFINITY;
    for axis in 0..3 {
        let origin = ray.origin()[axis];
        let direction = ray.direction()[axis];
        if direction.abs() <= f32::EPSILON {
            if origin < minimum[axis] || origin > maximum[axis] {
                return None;
            }
            continue;
        }
        let first = (minimum[axis] - origin) / direction;
        let second = (maximum[axis] - origin) / direction;
        near = near.max(first.min(second));
        far = far.min(first.max(second));
        if near > far {
            return None;
        }
    }
    (far >= 0.0).then_some(near)
}

fn append_box(
    output: &mut Vec<BoxTriangle>,
    center: &[f32],
    size: &[f32],
    color: [f32; 4],
    id: u64,
) {
    let h = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
    let v = [
        [center[0] - h[0], center[1] - h[1], center[2] - h[2]],
        [center[0] + h[0], center[1] - h[1], center[2] - h[2]],
        [center[0] + h[0], center[1] + h[1], center[2] - h[2]],
        [center[0] - h[0], center[1] + h[1], center[2] - h[2]],
        [center[0] - h[0], center[1] - h[1], center[2] + h[2]],
        [center[0] + h[0], center[1] - h[1], center[2] + h[2]],
        [center[0] + h[0], center[1] + h[1], center[2] + h[2]],
        [center[0] - h[0], center[1] + h[1], center[2] + h[2]],
    ];
    for indices in [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [3, 7, 6],
        [3, 6, 2],
        [0, 4, 7],
        [0, 7, 3],
        [1, 2, 6],
        [1, 6, 5],
    ] {
        output.push(BoxTriangle {
            vertices: [v[indices[0]], v[indices[1]], v[indices[2]]],
            color,
            id,
        });
    }
}
