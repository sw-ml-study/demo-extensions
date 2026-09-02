//! Bounded release-mode CPU planning evidence for the generic point path.

// Counts are capped at 100,000 and side is a finite positive square root, so
// these conversions are exact or intentionally quantized probe coordinates.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::hint::black_box;
use std::time::Instant;

use mlpl_native3d_scene::{Camera, PointLimits, PointScene, Viewport};
use mlpl_native3d_window::point_vertices;

const MAX_POINTS: usize = 100_000;

fn main() -> Result<(), String> {
    let counts = std::env::args()
        .skip(1)
        .map(|value| value.parse::<usize>().map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    if counts.is_empty()
        || counts
            .iter()
            .any(|count| *count == 0 || *count > MAX_POINTS)
    {
        return Err("provide point counts in 1..=100000".into());
    }
    for count in counts {
        probe(count)?;
    }
    println!("note=headless_cpu_timing_only_gpu_throughput_not_measured");
    Ok(())
}

fn probe(count: usize) -> Result<(), String> {
    let side = (count as f32).sqrt().ceil() as usize;
    let mut positions = Vec::with_capacity(count * 3);
    let mut sizes = Vec::with_capacity(count);
    let mut colors = Vec::with_capacity(count);
    let mut opacities = Vec::with_capacity(count);
    let mut ids = Vec::with_capacity(count);
    for index in 0..count {
        let row = index / side;
        let column = index % side;
        positions.extend([
            column as f32 / side as f32 * 3.6 - 1.8,
            row as f32 / side as f32 * 3.6 - 1.8,
            (index % 7) as f32 * 0.02,
        ]);
        sizes.push(3.0 + (index % 5) as f32);
        colors.push([0.2, 0.6, 1.0, 1.0]);
        opacities.push(0.8);
        ids.push(u64::try_from(index).map_err(|error| error.to_string())?);
    }
    let owned_bytes = count
        .checked_mul(44)
        .ok_or_else(|| "owned byte accounting overflow".to_owned())?;
    let limits = PointLimits::new(count, owned_bytes).map_err(|error| format!("{error:?}"))?;
    let scene = PointScene::from_parallel_arrays(positions, sizes, colors, opacities, ids, limits)
        .map_err(|error| format!("{error:?}"))?;
    let upload_bytes = scene
        .upload_plan()
        .map_err(|error| format!("{error:?}"))?
        .byte_len();
    let viewport = Viewport::new(1024, 768).map_err(|error| format!("{error:?}"))?;
    let started = Instant::now();
    let plan = black_box(&scene)
        .plan_points(Camera::default(), viewport, 0.0)
        .map_err(|error| format!("{error:?}"))?;
    let plan_us = started.elapsed().as_micros();
    let started = Instant::now();
    let vertices = point_vertices(black_box(plan.points()), viewport);
    let expand_us = started.elapsed().as_micros();
    let plan_bytes = size_of_val(plan.points());
    let expanded_bytes = size_of_val(vertices.as_slice());
    println!(
        "points={count} visible={} owned_bytes={owned_bytes} upload_bytes={upload_bytes} plan_bytes={plan_bytes} expanded_bytes={expanded_bytes} plan_us={plan_us} expand_us={expand_us}",
        plan.points().len()
    );
    Ok(())
}
