use mlpl_native3d_scene::{PointLimits, PointScene, PointSceneError};

fn assert_f32s(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }
}

const VALID: &str = r#"{
  "schema":"sw-ml-study.native3d.point-scene",
  "version":1,
  "positions":{"shape":[3,3],"values":[-1.0,0.0,1.0,0.0,1.0,0.0,1.0,0.0,-1.0]},
  "sizes":[2.0,4.0,6.0],
  "colors":[[1.0,0.0,0.0,1.0],[0.0,1.0,0.0,1.0],[0.0,0.0,1.0,1.0]],
  "opacities":[1.0,0.5,0.25],
  "ids":[7,8,9]
}"#;

#[test]
fn parses_and_plans_owned_parallel_point_attributes_deterministically() {
    let scene = PointScene::parse(VALID, PointLimits::new(3, 132).unwrap()).unwrap();
    assert_eq!(scene.len(), 3);
    assert_eq!(scene.positions().shape(), [3, 3]);
    assert_eq!(scene.ids(), &[7, 8, 9]);

    let first = scene.upload_plan().unwrap();
    let second = scene.upload_plan().unwrap();
    assert_eq!(first, second);
    assert_eq!(first.byte_len(), 120);
    assert_f32s(&first.points()[1].position(), &[0.0, 1.0, 0.0]);
    assert_f32s(&first.points()[1].color(), &[0.0, 1.0, 0.0, 0.5]);
    assert!((first.points()[1].size() - 4.0).abs() < f32::EPSILON);
    assert_eq!(first.points()[1].id(), 8);
}

#[test]
fn rejects_shapes_values_attributes_and_duplicate_ids() {
    let limits = PointLimits::new(10, 1_000).unwrap();
    assert_eq!(
        PointScene::parse("{", limits),
        Err(PointSceneError::Malformed)
    );
    assert_eq!(
        PointScene::parse(&VALID.replace("[3,3]", "[3,2]"), limits),
        Err(PointSceneError::PositionShape)
    );
    assert_eq!(
        PointScene::parse(&VALID.replace("-1.0,0.0,1.0", "null,0.0,1.0"), limits),
        Err(PointSceneError::Malformed)
    );
    assert_eq!(
        PointScene::parse(&VALID.replace("[2.0,4.0,6.0]", "[2.0,0.0,6.0]"), limits),
        Err(PointSceneError::PointSize)
    );
    assert_eq!(
        PointScene::parse(
            &VALID.replace("[0.0,1.0,0.0,1.0]", "[0.0,1.2,0.0,1.0]"),
            limits
        ),
        Err(PointSceneError::PointColor)
    );
    assert_eq!(
        PointScene::parse(&VALID.replace("[1.0,0.5,0.25]", "[1.0,-0.5,0.25]"), limits),
        Err(PointSceneError::PointOpacity)
    );
    assert_eq!(
        PointScene::parse(&VALID.replace("[7,8,9]", "[7,8,8]"), limits),
        Err(PointSceneError::DuplicateId(8))
    );
    assert_eq!(
        PointScene::parse(&VALID.replace("[7,8,9]", "[7,8]"), limits),
        Err(PointSceneError::ParallelLength)
    );
    assert_eq!(
        PointScene::from_parallel_arrays(
            vec![f32::NAN, 0.0, 0.0],
            vec![1.0],
            vec![[1.0; 4]],
            vec![1.0],
            vec![1],
            limits,
        ),
        Err(PointSceneError::PositionValue)
    );
}

#[test]
fn enforces_explicit_point_and_owned_byte_budgets_before_planning() {
    assert_eq!(
        PointScene::parse(VALID, PointLimits::new(2, 132).unwrap()),
        Err(PointSceneError::PointBudget {
            actual: 3,
            limit: 2
        })
    );
    assert_eq!(
        PointScene::parse(VALID, PointLimits::new(3, 131).unwrap()),
        Err(PointSceneError::ByteBudget {
            actual: 132,
            limit: 131
        })
    );
    assert_eq!(PointLimits::new(0, 1), Err(PointSceneError::InvalidLimits));
}
