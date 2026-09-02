use mlpl_microscope_model::{
    Action, Playback, RunStatus, ViewerState, parse_recording, repeated_series,
};

const REGRESSION: &str =
    include_str!("../../../integration/ml-microscope/fixtures/yew/linear-regression-run-v0.json");

#[test]
fn playback_clamps_restarts_ticks_and_auto_pauses() {
    let mut state = ViewerState::default();
    state.load(parse_recording(REGRESSION).unwrap());
    state.reduce(Action::Previous);
    assert_eq!(state.frame_index(), 0);
    state.reduce(Action::Play);
    assert_eq!(state.playback, Playback::Playing);
    for expected in 1..=4 {
        state.reduce(Action::Tick);
        assert_eq!(state.frame_index(), expected);
    }
    assert_eq!(state.playback, Playback::Paused);
    state.reduce(Action::Play);
    assert_eq!(state.frame_index(), 0);
    state.reduce(Action::Pause);
    state.reduce(Action::Tick);
    assert_eq!(state.frame_index(), 0);
}

#[test]
fn seek_selection_preservation_fallback_and_failure_are_atomic() {
    let recording = parse_recording(REGRESSION).unwrap();
    let mut state = ViewerState::default();
    state.load(recording.clone());
    state.reduce(Action::Select(2));
    state.reduce(Action::SeekIndex(1));
    assert_eq!(state.observation_index(), 0);
    state.reduce(Action::Select(3));
    state.reduce(Action::Next);
    assert_eq!(state.observation_index(), 3);
    state.reduce(Action::SeekIndex(99));
    assert_eq!(state.frame_index(), 2);
    state.reduce(Action::Fail("transport failed".into()));
    assert_eq!(state.recording(), Some(&recording));
    assert_eq!(
        state.run_status,
        RunStatus::Failed("transport failed".into())
    );
    assert_eq!(state.playback, Playback::Paused);
}

#[test]
fn series_are_keyed_by_repeated_semantic_name_only() {
    let recording = parse_recording(REGRESSION).unwrap();
    let loss = repeated_series(&recording, "regression/training/loss");
    assert_eq!(
        loss.iter().map(|item| item.0).collect::<Vec<_>>(),
        [0, 1, 2, 4, 8]
    );
    assert!(repeated_series(&recording, &recording.lesson).is_empty());
}
