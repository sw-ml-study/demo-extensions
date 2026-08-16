use crate::Viewport;

const PITCH_LIMIT: f32 = 1.55;
const EPSILON: f32 = 1.0e-6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InteractionError {
    NonFinite,
    InvalidCamera,
    InvalidScreenPoint,
    InvalidRay,
}

/// Renderer-neutral orbit camera controlled by MLPL application state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
    distance: f32,
    vertical_fov_radians: f32,
    near: f32,
}

/// World-space ray with a normalized direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3 {
    origin: [f32; 3],
    direction: [f32; 3],
}

impl OrbitCamera {
    /// Creates a bounded finite orbit camera.
    ///
    /// # Errors
    ///
    /// Rejects non-finite fields, pole-singular pitch, invalid clip distance,
    /// and invalid field of view.
    pub fn new(
        target: [f32; 3],
        yaw: f32,
        pitch: f32,
        distance: f32,
        vertical_fov_radians: f32,
        near: f32,
    ) -> Result<Self, InteractionError> {
        if !target.into_iter().all(f32::is_finite)
            || ![yaw, pitch, distance, vertical_fov_radians, near]
                .into_iter()
                .all(f32::is_finite)
        {
            return Err(InteractionError::NonFinite);
        }
        if !(-PITCH_LIMIT..=PITCH_LIMIT).contains(&pitch)
            || distance <= near
            || near <= 0.0
            || !(0.01..3.13).contains(&vertical_fov_radians)
        {
            return Err(InteractionError::InvalidCamera);
        }
        Ok(Self {
            target,
            yaw,
            pitch,
            distance,
            vertical_fov_radians,
            near,
        })
    }

    /// Builds a world ray for a physical-pixel point with a top-left origin.
    ///
    /// # Errors
    ///
    /// Rejects non-finite or out-of-viewport points and a degenerate camera
    /// basis. Viewports are bounded to 8192, which is exactly representable by
    /// `f32`.
    #[allow(clippy::cast_precision_loss)]
    pub fn pick_ray(self, viewport: Viewport, point: [f32; 2]) -> Result<Ray3, InteractionError> {
        let [viewport_width, viewport_height] = viewport.dimensions();
        let width = viewport_width as f32;
        let height = viewport_height as f32;
        if !point.into_iter().all(f32::is_finite)
            || point[0] < 0.0
            || point[1] < 0.0
            || point[0] > width
            || point[1] > height
        {
            return Err(InteractionError::InvalidScreenPoint);
        }
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let offset = [
            self.distance * cos_pitch * sin_yaw,
            self.distance * sin_pitch,
            self.distance * cos_pitch * cos_yaw,
        ];
        let origin = add(self.target, offset);
        let forward = normalize(sub(self.target, origin)).ok_or(InteractionError::InvalidCamera)?;
        let right =
            normalize(cross(forward, [0.0, 1.0, 0.0])).ok_or(InteractionError::InvalidCamera)?;
        let up = cross(right, forward);
        let ndc_x = point[0].mul_add(2.0 / width, -1.0);
        let ndc_y = 1.0 - point[1] * 2.0 / height;
        let half_height = (self.vertical_fov_radians * 0.5).tan();
        let direction = add(
            forward,
            add(
                scale(right, ndc_x * width / height * half_height),
                scale(up, ndc_y * half_height),
            ),
        );
        Ray3::new(origin, direction)
    }
}

impl Ray3 {
    /// Creates a finite ray and normalizes its direction.
    ///
    /// # Errors
    ///
    /// Rejects non-finite input and zero-length directions.
    pub fn new(origin: [f32; 3], direction: [f32; 3]) -> Result<Self, InteractionError> {
        if !origin.into_iter().all(f32::is_finite) || !direction.into_iter().all(f32::is_finite) {
            return Err(InteractionError::NonFinite);
        }
        let direction = normalize(direction).ok_or(InteractionError::InvalidRay)?;
        Ok(Self { origin, direction })
    }

    #[must_use]
    pub const fn origin(self) -> [f32; 3] {
        self.origin
    }

    #[must_use]
    pub const fn direction(self) -> [f32; 3] {
        self.direction
    }

    #[must_use]
    pub fn intersect_plane(self, point: [f32; 3], normal: [f32; 3]) -> Option<[f32; 3]> {
        if !point.into_iter().all(f32::is_finite) || !normal.into_iter().all(f32::is_finite) {
            return None;
        }
        let normal = normalize(normal)?;
        let denominator = dot(self.direction, normal);
        if denominator.abs() <= EPSILON {
            return None;
        }
        let distance = dot(sub(point, self.origin), normal) / denominator;
        if distance < 0.0 {
            return None;
        }
        Some(add(self.origin, scale(self.direction, distance)))
    }
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(v: [f32; 3], value: f32) -> [f32; 3] {
    [v[0] * value, v[1] * value, v[2] * value]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn normalize(v: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(v, v).sqrt();
    (length > EPSILON && length.is_finite()).then(|| scale(v, length.recip()))
}
