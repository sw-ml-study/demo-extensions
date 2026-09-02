use mlpl_microscope_model::{ViewKind, ViewerState, parse_recording};
use sha2::{Digest, Sha256};

const MATRIX: &str =
    include_str!("../../../integration/ml-microscope/fixtures/yew/matrix-run-v0.json");
const REGRESSION: &str =
    include_str!("../../../integration/ml-microscope/fixtures/yew/linear-regression-run-v0.json");

#[test]
fn pinned_recordings_match_counts_steps_and_hashes() {
    let cases = [
        (
            MATRIX,
            "6342be3ca9a8e17465310761c7039193ec2b0bc2d2e9eb3a835534a0747b148a",
            5,
            8,
            24,
            vec![0, 1, 2, 3, 4],
        ),
        (
            REGRESSION,
            "92f80e2997b6fc5b0cf0ee46484062f836d2f79f119f705b34fc29a6fa39e9b4",
            5,
            32,
            88,
            vec![0, 1, 2, 4, 8],
        ),
    ];
    for (json, hash, frames, observations, values, steps) in cases {
        assert_eq!(format!("{:x}", Sha256::digest(json.as_bytes())), hash);
        let recording = parse_recording(json).unwrap();
        assert_eq!(recording.frames.len(), frames);
        assert_eq!(recording.observation_count(), observations);
        assert_eq!(recording.value_count(), values);
        assert_eq!(
            recording
                .frames
                .iter()
                .map(|frame| frame.step)
                .collect::<Vec<_>>(),
            steps
        );
    }
}

#[test]
fn nonconsecutive_steps_navigate_by_retained_index() {
    let recording = parse_recording(REGRESSION).unwrap();
    let mut state = ViewerState::default();
    state.load(recording);
    state.reduce(mlpl_microscope_model::Action::Next);
    state.reduce(mlpl_microscope_model::Action::Next);
    state.reduce(mlpl_microscope_model::Action::Next);
    assert_eq!(state.frame_index(), 3);
    assert_eq!(state.current_frame().unwrap().step, 4);
}

#[test]
fn shape_directed_plans_keep_exact_fallbacks() {
    let recording = parse_recording(MATRIX).unwrap();
    let matrix = &recording.frames[0].observations[0];
    let plan = matrix.view_plan();
    assert_eq!(plan.kind, ViewKind::MatrixHeatmap);
    assert_eq!(plan.exact_values, matrix.values);
}
