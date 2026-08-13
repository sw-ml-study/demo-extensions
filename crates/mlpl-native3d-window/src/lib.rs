//! Native-window rendering primitives over renderer-neutral planned lines.

use bytemuck::{Pod, Zeroable};
use mlpl_native3d_scene::{PlannedLine, Viewport};

/// One GPU-ready position/color vertex in normalized device coordinates.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct GpuVertex {
    /// XY position in normalized device coordinates.
    pub position: [f32; 2],
    /// Linear RGBA line color.
    pub color: [f32; 4],
}

/// Expands generic thick lines into two GPU triangles per line.
#[must_use]
#[allow(clippy::cast_precision_loss)] // Viewport edges are bounded to 8192 and exactly fit f32.
pub fn line_vertices(lines: &[PlannedLine], viewport: Viewport) -> Vec<GpuVertex> {
    let [width, height] = viewport.dimensions();
    let width = width as f32;
    let height = height as f32;
    let mut output = Vec::with_capacity(lines.len() * 6);
    for line in lines {
        let start = line.start();
        let end = line.end();
        let delta = [end[0] - start[0], end[1] - start[1]];
        let length = delta[0].hypot(delta[1]);
        if length <= f32::EPSILON {
            continue;
        }
        let radius = line.thickness() * 0.5;
        let offset = [-delta[1] / length * radius, delta[0] / length * radius];
        let corners = [
            [start[0] + offset[0], start[1] + offset[1]],
            [start[0] - offset[0], start[1] - offset[1]],
            [end[0] + offset[0], end[1] + offset[1]],
            [end[0] - offset[0], end[1] - offset[1]],
        ];
        let corners = corners.map(|point| GpuVertex {
            position: [point[0] / width * 2.0 - 1.0, 1.0 - point[1] / height * 2.0],
            color: line.color(),
        });
        output.extend_from_slice(&[
            corners[0], corners[1], corners[2], corners[2], corners[1], corners[3],
        ]);
    }
    output
}
