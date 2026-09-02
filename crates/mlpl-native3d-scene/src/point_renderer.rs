//! Pure point projection, culling, ordering, picking, and CPU rasterization.

// Viewports are bounded to 8192 and raster coordinates/channels are clamped
// immediately before conversion, so these otherwise-lossy casts are exact or
// deliberately quantized at the rendering boundary.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::cmp::Ordering;

use crate::{Camera, HeadlessImage, PointScene, RenderError, Viewport};

/// One projected point in physical-pixel coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlannedScreenPoint {
    center: [f32; 2],
    depth: f32,
    size: f32,
    color: [f32; 4],
    id: u64,
}

impl PlannedScreenPoint {
    #[must_use]
    pub const fn center(self) -> [f32; 2] {
        self.center
    }

    #[must_use]
    pub const fn depth(self) -> f32 {
        self.depth
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

/// Bounded far-to-near point plan shared by rasterization and picking.
#[derive(Clone, Debug, PartialEq)]
pub struct PointRenderPlan {
    points: Vec<PlannedScreenPoint>,
}

impl PointRenderPlan {
    #[must_use]
    pub fn points(&self) -> &[PlannedScreenPoint] {
        &self.points
    }

    /// Picks the topmost circular point at a physical-pixel coordinate.
    #[must_use]
    pub fn pick(&self, position: [f32; 2]) -> Option<PlannedScreenPoint> {
        if !position.into_iter().all(f32::is_finite) {
            return None;
        }
        self.points.iter().rev().copied().find(|point| {
            let dx = position[0] - point.center[0];
            let dy = position[1] - point.center[1];
            let radius = point.size / 2.0;
            dx.mul_add(dx, dy * dy) <= radius * radius
        })
    }
}

pub(crate) fn plan_points(
    scene: &PointScene,
    camera: Camera,
    viewport: Viewport,
    rotation_y: f32,
) -> Result<PointRenderPlan, RenderError> {
    crate::renderer::validate_render_inputs(camera, rotation_y)?;
    let upload = scene.upload_plan().expect("validated point scene plans");
    let [width, height] = viewport.dimensions();
    let width = width as f32;
    let height = height as f32;
    let (sin, cos) = rotation_y.sin_cos();
    let (sin_yaw, cos_yaw) = camera.yaw().sin_cos();
    let (sin_pitch, cos_pitch) = camera.pitch().sin_cos();
    let target = camera.target();
    let eye = [
        target[0] + camera.distance() * cos_pitch * sin_yaw,
        target[1] + camera.distance() * sin_pitch,
        target[2] + camera.distance() * cos_pitch * cos_yaw,
    ];
    let forward = normalize(sub(target, eye)).ok_or(RenderError::InvalidCamera)?;
    let right = normalize(cross(forward, [0.0, 1.0, 0.0])).ok_or(RenderError::InvalidCamera)?;
    let up = cross(right, forward);
    let focal = height / (2.0 * (camera.vertical_fov_radians() / 2.0).tan());
    let mut points = Vec::with_capacity(upload.points().len());
    for point in upload.points() {
        let position = point.position();
        let rotated = [
            position[0].mul_add(cos, position[2] * sin),
            position[1],
            (-position[0]).mul_add(sin, position[2] * cos),
        ];
        let relative = sub(rotated, eye);
        let view = [
            dot(relative, right),
            dot(relative, up),
            dot(relative, forward),
        ];
        if view[2] < camera.near() {
            continue;
        }
        let center = [
            width / 2.0 + view[0] * focal / view[2],
            height / 2.0 - view[1] * focal / view[2],
        ];
        let radius = point.size() / 2.0;
        if center[0] + radius < 0.0
            || center[0] - radius >= width
            || center[1] + radius < 0.0
            || center[1] - radius >= height
        {
            continue;
        }
        points.push(PlannedScreenPoint {
            center,
            depth: view[2],
            size: point.size(),
            color: point.color(),
            id: point.id(),
        });
    }
    points.sort_by(|left, right| {
        right
            .depth
            .partial_cmp(&left.depth)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(PointRenderPlan { points })
}

pub(crate) fn rasterize(plan: &PointRenderPlan, viewport: Viewport) -> HeadlessImage {
    let [width, height] = viewport.dimensions();
    let mut rgba = vec![0; width as usize * height as usize * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&HeadlessImage::BACKGROUND);
    }
    for point in &plan.points {
        let radius = point.size / 2.0;
        let min_x = (point.center[0] - radius).floor().max(0.0) as u32;
        let max_x = (point.center[0] + radius).ceil().min(width as f32 - 1.0) as u32;
        let min_y = (point.center[1] - radius).floor().max(0.0) as u32;
        let max_y = (point.center[1] + radius).ceil().min(height as f32 - 1.0) as u32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = x as f32 + 0.5 - point.center[0];
                let dy = y as f32 + 0.5 - point.center[1];
                if dx.mul_add(dx, dy * dy) <= radius * radius {
                    blend(
                        &mut rgba[(y as usize * width as usize + x as usize) * 4..][..4],
                        point.color,
                    );
                }
            }
        }
    }
    HeadlessImage::from_rgba([width, height], rgba)
}

fn blend(destination: &mut [u8], source: [f32; 4]) {
    let alpha = source[3];
    for channel in 0..3 {
        let background = f32::from(destination[channel]) / 255.0;
        destination[channel] = (source[channel].mul_add(alpha, background * (1.0 - alpha)) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    destination[3] = 255;
}

fn sub(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1].mul_add(right[2], -left[2] * right[1]),
        left[2].mul_add(right[0], -left[0] * right[2]),
        left[0].mul_add(right[1], -left[1] * right[0]),
    ]
}

fn normalize(vector: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(vector, vector).sqrt();
    if !length.is_finite() || length <= f32::EPSILON {
        None
    } else {
        Some([vector[0] / length, vector[1] / length, vector[2] / length])
    }
}
