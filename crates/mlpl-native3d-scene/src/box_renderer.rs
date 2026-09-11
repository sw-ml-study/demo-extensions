//! Pure filled-triangle projection, depth ordering, and CPU rasterization.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::cmp::Ordering;

use crate::{BoxScene, Camera, HeadlessImage, RenderError, Viewport};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlannedScreenTriangle {
    vertices: [[f32; 2]; 3],
    depth: f32,
    color: [f32; 4],
    id: u64,
}

impl PlannedScreenTriangle {
    #[must_use]
    pub const fn vertices(self) -> [[f32; 2]; 3] {
        self.vertices
    }
    #[must_use]
    pub const fn depth(self) -> f32 {
        self.depth
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
pub struct BoxRenderPlan {
    triangles: Vec<PlannedScreenTriangle>,
}

impl BoxRenderPlan {
    #[must_use]
    pub fn triangles(&self) -> &[PlannedScreenTriangle] {
        &self.triangles
    }
}

pub(crate) fn plan_boxes(
    scene: &BoxScene,
    camera: Camera,
    viewport: Viewport,
    rotation_y: f32,
) -> Result<BoxRenderPlan, RenderError> {
    crate::renderer::validate_render_inputs(camera, rotation_y)?;
    let [width, height] = viewport.dimensions();
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
    let focal = height as f32 / (2.0 * (camera.vertical_fov_radians() / 2.0).tan());
    let mut triangles = Vec::new();
    for triangle in scene
        .triangle_plan()
        .expect("validated box scene plans")
        .triangles()
    {
        let mut screen = [[0.0; 2]; 3];
        let mut depths = [0.0; 3];
        let mut visible = true;
        for (index, position) in triangle.vertices().into_iter().enumerate() {
            let rotated = [
                position[0].mul_add(cos, position[2] * sin),
                position[1],
                (-position[0]).mul_add(sin, position[2] * cos),
            ];
            let relative = sub(rotated, eye);
            let depth = dot(relative, forward);
            if depth < camera.near() {
                visible = false;
                break;
            }
            screen[index] = [
                width as f32 / 2.0 + dot(relative, right) * focal / depth,
                height as f32 / 2.0 - dot(relative, up) * focal / depth,
            ];
            depths[index] = depth;
        }
        if visible && triangle_overlaps(screen, width as f32, height as f32) {
            triangles.push(PlannedScreenTriangle {
                vertices: screen,
                depth: (depths[0] + depths[1] + depths[2]) / 3.0,
                color: triangle.color(),
                id: triangle.id(),
            });
        }
    }
    triangles.sort_by(|a, b| {
        b.depth
            .partial_cmp(&a.depth)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(BoxRenderPlan { triangles })
}

pub(crate) fn rasterize(plan: &BoxRenderPlan, viewport: Viewport) -> HeadlessImage {
    let [width, height] = viewport.dimensions();
    let mut rgba = vec![0; width as usize * height as usize * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&HeadlessImage::BACKGROUND);
    }
    for triangle in &plan.triangles {
        let xs = triangle.vertices.map(|v| v[0]);
        let ys = triangle.vertices.map(|v| v[1]);
        let min_x = xs
            .into_iter()
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let max_x = xs
            .into_iter()
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(width as f32 - 1.0) as u32;
        let min_y = ys
            .into_iter()
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let max_y = ys
            .into_iter()
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(height as f32 - 1.0) as u32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if contains(triangle.vertices, [x as f32 + 0.5, y as f32 + 0.5]) {
                    blend(
                        &mut rgba[(y as usize * width as usize + x as usize) * 4..][..4],
                        triangle.color,
                    );
                }
            }
        }
    }
    HeadlessImage::from_rgba([width, height], rgba)
}

fn triangle_overlaps(v: [[f32; 2]; 3], width: f32, height: f32) -> bool {
    v.iter()
        .any(|p| p[0] >= 0.0 && p[0] < width && p[1] >= 0.0 && p[1] < height)
        || (v.iter().any(|p| p[0] < 0.0) && v.iter().any(|p| p[0] >= width))
        || (v.iter().any(|p| p[1] < 0.0) && v.iter().any(|p| p[1] >= height))
}
fn contains(v: [[f32; 2]; 3], p: [f32; 2]) -> bool {
    let edge =
        |a: [f32; 2], b: [f32; 2]| (p[0] - b[0]) * (a[1] - b[1]) - (p[1] - b[1]) * (a[0] - b[0]);
    let e = [edge(v[0], v[1]), edge(v[1], v[2]), edge(v[2], v[0])];
    (e.iter().all(|x| *x >= 0.0) || e.iter().all(|x| *x <= 0.0))
        && e.iter().any(|x| x.abs() > f32::EPSILON)
}
fn blend(dst: &mut [u8], src: [f32; 4]) {
    for c in 0..3 {
        let bg = f32::from(dst[c]) / 255.0;
        dst[c] = (src[c].mul_add(src[3], bg * (1.0 - src[3])) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    dst[3] = 255;
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1].mul_add(b[2], -a[2] * b[1]),
        a[2].mul_add(b[0], -a[0] * b[2]),
        a[0].mul_add(b[1], -a[1] * b[0]),
    ]
}
fn normalize(v: [f32; 3]) -> Option<[f32; 3]> {
    let l = dot(v, v).sqrt();
    if !l.is_finite() || l <= f32::EPSILON {
        None
    } else {
        Some([v[0] / l, v[1] / l, v[2] / l])
    }
}
