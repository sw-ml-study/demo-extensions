//! Private state transition helpers.
use crate::{Playback, RunStatus, ViewerState};
pub(crate) fn move_to(state: &mut ViewerState, index: usize) {
    let Some(recording) = &state.recording else {
        return;
    };
    let target = index.min(recording.frames.len().saturating_sub(1));
    let selected = state.current_observation().map(|item| item.name.clone());
    state.frame_index = target;
    state.observation_index = selected
        .and_then(|name| {
            recording.frames[target]
                .observations
                .iter()
                .position(|item| item.name == name)
        })
        .unwrap_or(0);
    if target + 1 == recording.frames.len() {
        state.playback = Playback::Paused;
    }
}
pub(crate) fn advance(state: &mut ViewerState, tick: bool) {
    if !tick || state.playback == Playback::Playing {
        move_to(state, state.frame_index.saturating_add(1));
    }
}
pub(crate) fn seek(state: &mut ViewerState, index: usize) {
    if state
        .recording
        .as_ref()
        .is_some_and(|recording| index < recording.frames.len())
    {
        move_to(state, index);
    }
}
pub(crate) fn select(state: &mut ViewerState, index: usize) {
    if state
        .current_frame()
        .is_some_and(|frame| index < frame.observations.len())
    {
        state.observation_index = index;
    }
}
pub(crate) fn play(state: &mut ViewerState) {
    let Some(recording) = &state.recording else {
        return;
    };
    if recording.frames.len() < 2 {
        return;
    }
    if state.frame_index + 1 == recording.frames.len() {
        move_to(state, 0);
    }
    state.playback = Playback::Playing;
}
pub(crate) fn fail(state: &mut ViewerState, error: String) {
    state.playback = Playback::Paused;
    state.run_status = RunStatus::Failed(error);
}
