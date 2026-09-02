//! Pure transform, projection, clipping, and deterministic CPU rasterization.

use crate::LineScene;

const MAX_VIEWPORT_EDGE: u32 = 8_192;
const BACKGROUND: [u8; 4] = [8, 10, 16, 255];

/// Perspective camera parameters for the renderer-neutral line pipeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
    distance: f32,
    vertical_fov_radians: f32,
    near: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: [0.0; 3],
            yaw: 0.0,
            pitch: 0.0,
            distance: 4.0,
            vertical_fov_radians: 60.0_f32.to_radians(),
            near: 0.1,
        }
    }
}

impl Camera {
    /// Creates a perspective camera looking toward the origin along its depth axis.
    ///
    /// # Errors
    ///
    /// Rejects non-finite values, a non-positive near plane, a distance not
    /// beyond the near plane, or a field of view outside `(0.01, 3.13)` radians.
    pub fn perspective(
        distance: f32,
        vertical_fov_radians: f32,
        near: f32,
    ) -> Result<Self, RenderError> {
        let camera = Self {
            target: [0.0; 3],
            yaw: 0.0,
            pitch: 0.0,
            distance,
            vertical_fov_radians,
            near,
        };
        validate_camera(camera)?;
        Ok(camera)
    }

    /// Creates an orbit camera looking at `target`.
    ///
    /// # Errors
    ///
    /// Rejects non-finite values, pitch outside `-1.55..=1.55`, invalid clip
    /// distance, or field of view outside `(0.01, 3.13)` radians.
    pub fn orbit(
        target: [f32; 3],
        yaw: f32,
        pitch: f32,
        distance: f32,
        vertical_fov_radians: f32,
        near: f32,
    ) -> Result<Self, RenderError> {
        let camera = Self {
            target,
            yaw,
            pitch,
            distance,
            vertical_fov_radians,
            near,
        };
        validate_camera(camera)?;
        Ok(camera)
    }

    #[must_use]
    pub const fn target(self) -> [f32; 3] {
        self.target
    }
    #[must_use]
    pub const fn yaw(self) -> f32 {
        self.yaw
    }
    #[must_use]
    pub const fn pitch(self) -> f32 {
        self.pitch
    }
    #[must_use]
    pub const fn distance(self) -> f32 {
        self.distance
    }

    pub(crate) const fn vertical_fov_radians(self) -> f32 {
        self.vertical_fov_radians
    }

    pub(crate) const fn near(self) -> f32 {
        self.near
    }
}

/// A bounded output surface measured in physical pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Viewport {
    width: u32,
    height: u32,
}

impl Viewport {
    /// Creates a non-empty viewport no larger than 8192 pixels per edge.
    ///
    /// # Errors
    ///
    /// Rejects zero-sized and excessively large dimensions.
    pub const fn new(width: u32, height: u32) -> Result<Self, RenderError> {
        if width == 0 || height == 0 || width > MAX_VIEWPORT_EDGE || height > MAX_VIEWPORT_EDGE {
            return Err(RenderError::InvalidViewport);
        }
        Ok(Self { width, height })
    }

    /// Returns `[width, height]` in pixels.
    #[must_use]
    pub const fn dimensions(self) -> [u32; 2] {
        [self.width, self.height]
    }
}

/// One projected, viewport-clipped line ready for a rendering backend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlannedLine {
    start: [f32; 2],
    end: [f32; 2],
    color: [f32; 4],
    thickness: f32,
}

impl PlannedLine {
    /// Returns the first endpoint in top-left-origin pixel coordinates.
    #[must_use]
    pub const fn start(self) -> [f32; 2] {
        self.start
    }

    /// Returns the second endpoint in top-left-origin pixel coordinates.
    #[must_use]
    pub const fn end(self) -> [f32; 2] {
        self.end
    }

    /// Returns linear RGBA.
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        self.color
    }

    /// Returns thickness in logical pixels.
    #[must_use]
    pub const fn thickness(self) -> f32 {
        self.thickness
    }

    /// Returns projected length in pixels.
    #[must_use]
    pub fn length(self) -> f32 {
        let dx = self.end[0] - self.start[0];
        let dy = self.end[1] - self.start[1];
        dx.hypot(dy)
    }
}

/// Deterministic CPU-rasterized RGBA image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeadlessImage {
    dimensions: [u32; 2],
    rgba: Vec<u8>,
}

impl HeadlessImage {
    pub(crate) const BACKGROUND: [u8; 4] = BACKGROUND;

    pub(crate) fn from_rgba(dimensions: [u32; 2], rgba: Vec<u8>) -> Self {
        Self { dimensions, rgba }
    }
    /// Returns `[width, height]`.
    #[must_use]
    pub const fn dimensions(&self) -> [u32; 2] {
        self.dimensions
    }

    /// Returns row-major RGBA8 pixels.
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Encodes the image as binary PPM for portable headless evidence.
    #[must_use]
    pub fn ppm_bytes(&self) -> Vec<u8> {
        let mut output =
            format!("P6\n{} {}\n255\n", self.dimensions[0], self.dimensions[1]).into_bytes();
        output.reserve(self.rgba.len() / 4 * 3);
        for pixel in self.rgba.chunks_exact(4) {
            output.extend_from_slice(&pixel[..3]);
        }
        output
    }
}

/// Fail-closed errors for headless rendering inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    /// The output dimensions are empty or exceed the bounded allocation.
    InvalidViewport,
    /// Camera projection parameters are non-finite or nonsensical.
    InvalidCamera,
    /// Rotation must be finite.
    NonFiniteRotation,
}

pub(crate) fn validate_render_inputs(camera: Camera, rotation_y: f32) -> Result<(), RenderError> {
    validate_camera(camera)?;
    if !rotation_y.is_finite() {
        return Err(RenderError::NonFiniteRotation);
    }
    Ok(())
}

pub(crate) fn plan_lines(
    scene: &LineScene,
    camera: Camera,
    viewport: Viewport,
    rotation_y: f32,
) -> Result<Vec<PlannedLine>, RenderError> {
    validate_camera(camera)?;
    if !rotation_y.is_finite() {
        return Err(RenderError::NonFiniteRotation);
    }
    let (sin, cos) = rotation_y.sin_cos();
    let (sin_yaw, cos_yaw) = camera.yaw.sin_cos();
    let (sin_pitch, cos_pitch) = camera.pitch.sin_cos();
    let eye = [
        camera.target[0] + camera.distance * cos_pitch * sin_yaw,
        camera.target[1] + camera.distance * sin_pitch,
        camera.target[2] + camera.distance * cos_pitch * cos_yaw,
    ];
    let forward = normalize(sub(camera.target, eye)).ok_or(RenderError::InvalidCamera)?;
    let right = normalize(cross(forward, [0.0, 1.0, 0.0])).ok_or(RenderError::InvalidCamera)?;
    let up = cross(right, forward);
    let transformed: Vec<[f32; 3]> = scene
        .positions
        .values
        .chunks_exact(3)
        .map(|point| {
            let x = point[0].mul_add(cos, point[2] * sin);
            let z = (-point[0]).mul_add(sin, point[2] * cos);
            let relative = sub([x, point[1], z], eye);
            [
                dot(relative, right),
                dot(relative, up),
                dot(relative, forward),
            ]
        })
        .collect();

    let mut output = Vec::with_capacity(scene.edges.shape[0]);
    for (edge_index, edge) in scene.edges.values.chunks_exact(2).enumerate() {
        let Some((start, end)) = clip_near(transformed[edge[0]], transformed[edge[1]], camera.near)
        else {
            continue;
        };
        let start = project(start, camera, viewport);
        let end = project(end, camera, viewport);
        let Some((start, end)) = clip_viewport(start, end, viewport) else {
            continue;
        };
        if (start[0] - end[0]).abs() < f32::EPSILON && (start[1] - end[1]).abs() < f32::EPSILON {
            continue;
        }
        let (color, thickness) = scene.style(edge_index);
        output.push(PlannedLine {
            start,
            end,
            color,
            thickness,
        });
    }
    Ok(output)
}

fn validate_camera(camera: Camera) -> Result<(), RenderError> {
    if !camera.target.into_iter().all(f32::is_finite)
        || !camera.yaw.is_finite()
        || !camera.pitch.is_finite()
        || !(-1.55..=1.55).contains(&camera.pitch)
        || !camera.distance.is_finite()
        || !camera.vertical_fov_radians.is_finite()
        || !camera.near.is_finite()
        || camera.distance <= camera.near
        || camera.near <= 0.0
        || !(0.01..3.13).contains(&camera.vertical_fov_radians)
    {
        return Err(RenderError::InvalidCamera);
    }
    Ok(())
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
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

fn normalize(value: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(value, value).sqrt();
    (length > f32::EPSILON && length.is_finite())
        .then(|| [value[0] / length, value[1] / length, value[2] / length])
}

fn clip_near(mut start: [f32; 3], mut end: [f32; 3], near: f32) -> Option<([f32; 3], [f32; 3])> {
    if start[2] < near && end[2] < near {
        return None;
    }
    if start[2] < near || end[2] < near {
        let t = (near - start[2]) / (end[2] - start[2]);
        let clipped = [
            start[0] + (end[0] - start[0]) * t,
            start[1] + (end[1] - start[1]) * t,
            near,
        ];
        if start[2] < near {
            start = clipped;
        } else {
            end = clipped;
        }
    }
    Some((start, end))
}

#[allow(clippy::cast_precision_loss)] // Viewport dimensions are bounded to 8192 and exactly fit f32.
fn project(point: [f32; 3], camera: Camera, viewport: Viewport) -> [f32; 2] {
    let focal = 0.5 * viewport.height as f32 / (camera.vertical_fov_radians * 0.5).tan();
    [
        viewport.width as f32 * 0.5 + point[0] * focal / point[2],
        viewport.height as f32 * 0.5 - point[1] * focal / point[2],
    ]
}

#[allow(clippy::cast_precision_loss)] // Viewport dimensions are bounded to 8192 and exactly fit f32.
fn clip_viewport(
    start: [f32; 2],
    end: [f32; 2],
    viewport: Viewport,
) -> Option<([f32; 2], [f32; 2])> {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let mut low = 0.0_f32;
    let mut high = 1.0_f32;
    let bounds = [
        (-delta[0], start[0]),
        (delta[0], viewport.width as f32 - 1.0 - start[0]),
        (-delta[1], start[1]),
        (delta[1], viewport.height as f32 - 1.0 - start[1]),
    ];
    for (p, q) in bounds {
        if p.abs() < f32::EPSILON {
            if q < 0.0 {
                return None;
            }
        } else {
            let ratio = q / p;
            if p < 0.0 {
                low = low.max(ratio);
            } else {
                high = high.min(ratio);
            }
            if low > high {
                return None;
            }
        }
    }
    Some((
        [start[0] + delta[0] * low, start[1] + delta[1] * low],
        [start[0] + delta[0] * high, start[1] + delta[1] * high],
    ))
}

pub(crate) fn rasterize(lines: &[PlannedLine], viewport: Viewport) -> HeadlessImage {
    let mut rgba = vec![0; viewport.width as usize * viewport.height as usize * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&BACKGROUND);
    }
    for line in lines {
        rasterize_line(&mut rgba, viewport, *line);
    }
    HeadlessImage {
        dimensions: viewport.dimensions(),
        rgba,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)] // Coordinates are finite and clipped to the bounded non-negative viewport first.
fn rasterize_line(rgba: &mut [u8], viewport: Viewport, line: PlannedLine) {
    let radius = line.thickness * 0.5;
    let min_x = (line.start[0].min(line.end[0]) - radius).floor().max(0.0) as u32;
    let max_x = (line.start[0].max(line.end[0]) + radius)
        .ceil()
        .min(viewport.width as f32 - 1.0) as u32;
    let min_y = (line.start[1].min(line.end[1]) - radius).floor().max(0.0) as u32;
    let max_y = (line.start[1].max(line.end[1]) + radius)
        .ceil()
        .min(viewport.height as f32 - 1.0) as u32;
    let delta = [line.end[0] - line.start[0], line.end[1] - line.start[1]];
    let length_squared = delta[0].mul_add(delta[0], delta[1] * delta[1]);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = [x as f32 + 0.5, y as f32 + 0.5];
            let relative = [point[0] - line.start[0], point[1] - line.start[1]];
            let t = ((relative[0] * delta[0] + relative[1] * delta[1]) / length_squared)
                .clamp(0.0, 1.0);
            let closest = [line.start[0] + delta[0] * t, line.start[1] + delta[1] * t];
            if (point[0] - closest[0]).hypot(point[1] - closest[1]) <= radius {
                let offset = (y as usize * viewport.width as usize + x as usize) * 4;
                blend(&mut rgba[offset..offset + 4], line.color);
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
// Validated source channels and normalized destination channels bound the result to u8.
fn blend(destination: &mut [u8], source: [f32; 4]) {
    let alpha = source[3];
    for channel in 0..3 {
        let background = f32::from(destination[channel]) / 255.0;
        destination[channel] =
            (source[channel].mul_add(alpha, background * (1.0 - alpha)) * 255.0).round() as u8;
    }
    destination[3] = 255;
}
