//! Native-window rendering primitives over renderer-neutral planned lines.

use bytemuck::{Pod, Zeroable};
use mlpl_native3d_scene::{PlannedLine, Viewport};

pub mod audio;
pub mod disk_usage;
pub mod interaction;
pub mod live;

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

/// Rasterizes a compact ASCII help overlay into GPU triangles.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn text_vertices(text: &str, viewport: Viewport) -> Vec<GpuVertex> {
    text_vertices_colored(text, viewport, [14.0, 14.0], [0.85, 0.9, 1.0, 1.0])
}

/// Rasterizes compact ASCII text at a screen-space origin with a caller-owned color.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn text_vertices_colored(
    text: &str,
    viewport: Viewport,
    origin: [f32; 2],
    color: [f32; 4],
) -> Vec<GpuVertex> {
    let [width, height] = viewport.dimensions();
    let (width, height) = (width as f32, height as f32);
    let mut output = Vec::new();
    let (mut x, mut y) = (origin[0], origin[1]);
    for character in text.chars() {
        if character == '\n' {
            x = origin[0];
            y += 18.0;
            continue;
        }
        for (row, bits) in glyph(character).into_iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    push_quad(
                        &mut output,
                        x + column as f32 * 2.0,
                        y + row as f32 * 2.0,
                        width,
                        height,
                        color,
                    );
                }
            }
        }
        x += 12.0;
    }
    output
}

fn push_quad(
    output: &mut Vec<GpuVertex>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
) {
    let point = |px: f32, py: f32| GpuVertex {
        position: [px / width * 2.0 - 1.0, 1.0 - py / height * 2.0],
        color,
    };
    let corners = [
        point(x, y),
        point(x, y + 2.0),
        point(x + 2.0, y),
        point(x + 2.0, y + 2.0),
    ];
    output.extend_from_slice(&[
        corners[0], corners[1], corners[2], corners[2], corners[1], corners[3],
    ]);
}

fn glyph(character: char) -> [u8; 7] {
    match character.to_ascii_uppercase() {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        ',' => [0, 0, 0, 0, 4, 4, 8],
        ':' => [0, 12, 12, 0, 12, 12, 0],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        '%' => [17, 2, 4, 8, 16, 17, 0],
        '=' => [0, 0, 31, 0, 31, 0, 0],
        '|' => [4, 4, 4, 4, 4, 4, 4],
        '>' => [16, 8, 4, 2, 4, 8, 16],
        '<' => [1, 2, 4, 8, 4, 2, 1],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        '[' => [14, 8, 8, 8, 8, 8, 14],
        ']' => [14, 2, 2, 2, 2, 2, 14],
        _ => [0; 7],
    }
}
