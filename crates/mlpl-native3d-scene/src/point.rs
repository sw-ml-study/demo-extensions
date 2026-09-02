//! Bounded renderer-neutral point scenes and deterministic upload planning.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::NumericArray;

const POINT_SCHEMA: &str = "sw-ml-study.native3d.point-scene";
const POINT_VERSION: u32 = 1;
const MAX_POINT_LIMIT: usize = 1_000_000;
const OWNED_BYTES_PER_POINT: usize = 3 * size_of::<f32>()
    + size_of::<f32>()
    + 4 * size_of::<f32>()
    + size_of::<f32>()
    + size_of::<u64>();
const UPLOAD_BYTES_PER_POINT: usize =
    3 * size_of::<f32>() + size_of::<f32>() + 4 * size_of::<f32>() + size_of::<u64>();

/// Caller-selected limits for one owned point scene.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointLimits {
    max_points: usize,
    max_bytes: usize,
}

impl PointLimits {
    /// Creates explicit nonzero limits within the implementation hard cap.
    ///
    /// # Errors
    ///
    /// Rejects zero limits and point counts above one million.
    pub const fn new(max_points: usize, max_bytes: usize) -> Result<Self, PointSceneError> {
        if max_points == 0 || max_points > MAX_POINT_LIMIT || max_bytes == 0 {
            return Err(PointSceneError::InvalidLimits);
        }
        Ok(Self {
            max_points,
            max_bytes,
        })
    }
}

/// One validated, owned, renderer-neutral point cloud.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointScene {
    schema: String,
    version: u32,
    positions: NumericArray,
    sizes: Vec<f32>,
    colors: Vec<[f32; 4]>,
    opacities: Vec<f32>,
    ids: Vec<u64>,
}

/// One interleaved point record ready for a backend-owned upload copy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlannedPoint {
    position: [f32; 3],
    size: f32,
    color: [f32; 4],
    id: u64,
}

impl PlannedPoint {
    #[must_use]
    pub const fn position(self) -> [f32; 3] {
        self.position
    }

    #[must_use]
    pub const fn size(self) -> f32 {
        self.size
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

/// Deterministic backend-neutral upload description.
#[derive(Clone, Debug, PartialEq)]
pub struct PointUploadPlan {
    points: Vec<PlannedPoint>,
    byte_len: usize,
}

impl PointUploadPlan {
    #[must_use]
    pub fn points(&self) -> &[PlannedPoint] {
        &self.points
    }

    #[must_use]
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }
}

/// Fail-closed point-scene validation errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PointSceneError {
    Malformed,
    UnsupportedSchema,
    UnsupportedVersion(u32),
    InvalidLimits,
    PositionShape,
    PositionValue,
    ParallelLength,
    PointSize,
    PointColor,
    PointOpacity,
    DuplicateId(u64),
    PointBudget { actual: usize, limit: usize },
    ByteBudget { actual: usize, limit: usize },
}

impl PointScene {
    /// Builds and validates a point scene from owned parallel arrays.
    ///
    /// # Errors
    ///
    /// Applies the same finite-value, shape, attribute, identity, and budget
    /// checks as JSON parsing.
    pub fn from_parallel_arrays(
        positions: Vec<f32>,
        sizes: Vec<f32>,
        colors: Vec<[f32; 4]>,
        opacities: Vec<f32>,
        ids: Vec<u64>,
        limits: PointLimits,
    ) -> Result<Self, PointSceneError> {
        if positions.len().checked_rem(3) != Some(0) {
            return Err(PointSceneError::PositionShape);
        }
        let scene = Self {
            schema: POINT_SCHEMA.to_owned(),
            version: POINT_VERSION,
            positions: NumericArray {
                shape: [positions.len() / 3, 3],
                values: positions,
            },
            sizes,
            colors,
            opacities,
            ids,
        };
        scene.validate(limits)?;
        Ok(scene)
    }

    /// Parses and validates an owned point scene under caller-selected limits.
    ///
    /// # Errors
    ///
    /// Rejects malformed data, unsupported schema versions, invalid parallel
    /// arrays or values, duplicate IDs, and point/byte budget overruns.
    pub fn parse(source: &str, limits: PointLimits) -> Result<Self, PointSceneError> {
        let scene: Self = serde_json::from_str(source).map_err(|_| PointSceneError::Malformed)?;
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
    pub const fn positions(&self) -> &NumericArray {
        &self.positions
    }

    #[must_use]
    pub fn ids(&self) -> &[u64] {
        &self.ids
    }

    /// Produces a deterministic owned upload copy in stable input order.
    ///
    /// # Errors
    ///
    /// Returns a byte-budget error if platform arithmetic cannot represent the
    /// already validated upload size.
    pub fn upload_plan(&self) -> Result<PointUploadPlan, PointSceneError> {
        let byte_len =
            self.len()
                .checked_mul(UPLOAD_BYTES_PER_POINT)
                .ok_or(PointSceneError::ByteBudget {
                    actual: usize::MAX,
                    limit: usize::MAX - 1,
                })?;
        let points = self
            .positions
            .values
            .chunks_exact(3)
            .zip(&self.sizes)
            .zip(&self.colors)
            .zip(&self.opacities)
            .zip(&self.ids)
            .map(|((((position, size), color), opacity), id)| PlannedPoint {
                position: [position[0], position[1], position[2]],
                size: *size,
                color: [color[0], color[1], color[2], color[3] * opacity],
                id: *id,
            })
            .collect();
        Ok(PointUploadPlan { points, byte_len })
    }

    fn validate(&self, limits: PointLimits) -> Result<(), PointSceneError> {
        if self.schema != POINT_SCHEMA {
            return Err(PointSceneError::UnsupportedSchema);
        }
        if self.version != POINT_VERSION {
            return Err(PointSceneError::UnsupportedVersion(self.version));
        }
        let [count, columns] = self.positions.shape;
        if count == 0
            || columns != 3
            || count.checked_mul(columns) != Some(self.positions.values.len())
        {
            return Err(PointSceneError::PositionShape);
        }
        if count > limits.max_points {
            return Err(PointSceneError::PointBudget {
                actual: count,
                limit: limits.max_points,
            });
        }
        let bytes =
            count
                .checked_mul(OWNED_BYTES_PER_POINT)
                .ok_or(PointSceneError::ByteBudget {
                    actual: usize::MAX,
                    limit: limits.max_bytes,
                })?;
        if bytes > limits.max_bytes {
            return Err(PointSceneError::ByteBudget {
                actual: bytes,
                limit: limits.max_bytes,
            });
        }
        if !self.positions.values.iter().all(|value| value.is_finite()) {
            return Err(PointSceneError::PositionValue);
        }
        if self.sizes.len() != count
            || self.colors.len() != count
            || self.opacities.len() != count
            || self.ids.len() != count
        {
            return Err(PointSceneError::ParallelLength);
        }
        if !self
            .sizes
            .iter()
            .all(|size| size.is_finite() && (0.5..=256.0).contains(size))
        {
            return Err(PointSceneError::PointSize);
        }
        if !self
            .colors
            .iter()
            .flatten()
            .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
        {
            return Err(PointSceneError::PointColor);
        }
        if !self
            .opacities
            .iter()
            .all(|opacity| opacity.is_finite() && (0.0..=1.0).contains(opacity))
        {
            return Err(PointSceneError::PointOpacity);
        }
        let mut seen = BTreeSet::new();
        for id in &self.ids {
            if !seen.insert(*id) {
                return Err(PointSceneError::DuplicateId(*id));
            }
        }
        Ok(())
    }
}
