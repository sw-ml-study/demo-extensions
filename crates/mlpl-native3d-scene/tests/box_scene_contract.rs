use mlpl_native3d_scene::{BoxLimits, BoxScene, BoxSceneError};

const FIXTURE: &str = include_str!("../../../fixtures/native3d-box-scene.json");

fn limits() -> BoxLimits {
    BoxLimits::new(16, 4096).unwrap()
}

fn assert_f32s(actual: [f32; 4], expected: [f32; 4]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
    );
}

#[test]
fn parses_the_renderer_neutral_fixture() {
    let scene = BoxScene::parse(FIXTURE, limits()).unwrap();
    assert_eq!(scene.ids(), &[17, 23]);
    assert_eq!(scene.triangle_plan().unwrap().triangles().len(), 24);
}

#[test]
fn plans_owned_parallel_boxes_as_twelve_triangles_each() {
    let scene = BoxScene::from_parallel_arrays(
        vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0],
        vec![2.0, 4.0, 6.0, 1.0, 1.0, 1.0],
        vec![[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 0.5]],
        vec![17, 23],
        limits(),
    )
    .unwrap();

    let plan = scene.triangle_plan().unwrap();
    assert_eq!(scene.len(), 2);
    assert_eq!(plan.triangles().len(), 24);
    assert_eq!(plan.triangles()[0].id(), 17);
    assert_f32s(plan.triangles()[0].color(), [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(plan.byte_len(), 24 * 3 * (3 * 4 + 4 * 4 + 8));
}

#[test]
fn rejects_shapes_values_colors_ids_and_budgets() {
    let make = |centers, sizes, colors, ids, limits| {
        BoxScene::from_parallel_arrays(centers, sizes, colors, ids, limits)
    };
    assert_eq!(
        make(
            vec![0.0, 0.0],
            vec![1.0, 1.0, 1.0],
            vec![[1.0; 4]],
            vec![1],
            limits()
        ),
        Err(BoxSceneError::CenterShape)
    );
    assert_eq!(
        make(
            vec![0.0; 3],
            vec![1.0, 0.0, 1.0],
            vec![[1.0; 4]],
            vec![1],
            limits()
        ),
        Err(BoxSceneError::BoxSize)
    );
    assert_eq!(
        make(
            vec![0.0; 3],
            vec![1.0; 3],
            vec![[1.1; 4]],
            vec![1],
            limits()
        ),
        Err(BoxSceneError::BoxColor)
    );
    assert_eq!(
        make(
            vec![0.0; 6],
            vec![1.0; 6],
            vec![[1.0; 4]; 2],
            vec![1, 1],
            limits()
        ),
        Err(BoxSceneError::DuplicateId(1))
    );
    assert_eq!(
        make(
            vec![0.0; 6],
            vec![1.0; 6],
            vec![[1.0; 4]; 2],
            vec![1, 2],
            BoxLimits::new(1, 4096).unwrap()
        ),
        Err(BoxSceneError::BoxBudget {
            actual: 2,
            limit: 1
        })
    );
}
