use mlpl_native3d_scene::{BoxLimits, BoxScene, Camera, Viewport};
use mlpl_native3d_window::box_vertices;

fn plan(viewport: Viewport) -> mlpl_native3d_scene::BoxRenderPlan {
    BoxScene::from_parallel_arrays(
        vec![0.0, 0.0, 0.0],
        vec![1.0, 2.0, 3.0],
        vec![[0.2, 0.4, 0.6, 0.75]],
        vec![0x0123_4567_89ab_cdef],
        BoxLimits::new(1, 48).unwrap(),
    )
    .unwrap()
    .plan_box_triangles(Camera::default(), viewport, 0.0)
    .unwrap()
}

#[test]
fn expands_box_triangles_to_depth_and_identity_retaining_vertices() {
    let viewport = Viewport::new(200, 100).unwrap();
    let vertices = box_vertices(plan(viewport).triangles(), viewport, None);
    assert_eq!(vertices.len(), 36);
    assert_eq!(std::mem::size_of_val(&vertices[0]), 52);
    assert!(vertices.iter().all(|vertex| {
        vertex.position[0].is_finite()
            && vertex.position[1].is_finite()
            && (0.0..1.0).contains(&vertex.position[2])
            && vertex.stable_id == [0x89ab_cdef, 0x0123_4567]
    }));
}

#[test]
fn selection_outline_flag_is_deterministic_and_viewport_relative() {
    let small = Viewport::new(200, 100).unwrap();
    let large = Viewport::new(400, 200).unwrap();
    let normal = box_vertices(plan(small).triangles(), small, None);
    let selected = box_vertices(plan(small).triangles(), small, Some(0x0123_4567_89ab_cdef));
    let resized = box_vertices(plan(large).triangles(), large, None);
    assert!(
        normal[0]
            .color
            .into_iter()
            .zip(selected[0].color)
            .all(|(left, right)| (left - right).abs() < f32::EPSILON)
    );
    assert!(normal.iter().all(|vertex| vertex.selected == 0));
    assert!(selected.iter().all(|vertex| vertex.selected == 1));
    assert!(
        selected[0]
            .barycentric
            .into_iter()
            .zip([1.0, 0.0, 0.0])
            .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
    );
    assert!((normal[0].position[0] - resized[0].position[0]).abs() < 0.001);
    assert!((normal[0].position[1] - resized[0].position[1]).abs() < 0.001);
}

#[test]
fn empty_box_plan_produces_no_gpu_work() {
    assert!(box_vertices(&[], Viewport::new(32, 32).unwrap(), None).is_empty());
}
