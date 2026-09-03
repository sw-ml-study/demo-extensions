//! Browser callbacks and fixture loading.
use crate::app::{
    Fixture, KMEANS_JSON, KMEANS_SOURCE, MATRIX_JSON, MATRIX_SOURCE, REGRESSION_JSON,
    REGRESSION_SOURCE,
};
use mlpl_microscope_model::{Action, MotionPreference, ViewerState, parse_recording};
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlTextAreaElement, KeyboardEvent};
use yew::prelude::*;
pub(crate) fn loaded_state(json: &str) -> ViewerState {
    let mut state = ViewerState::default();
    if let Ok(recording) = parse_recording(json) {
        state.load(recording);
    }
    state
}
pub(crate) fn dispatch(state: &UseStateHandle<ViewerState>, action: Action) {
    let mut next = (**state).clone();
    next.reduce(action);
    state.set(next);
}
pub(crate) fn source_callback(source: &UseStateHandle<String>) -> Callback<InputEvent> {
    let source = source.clone();
    Callback::from(move |event: InputEvent| {
        source.set(event.target_unchecked_into::<HtmlTextAreaElement>().value())
    })
}
pub(crate) fn run_callback(status: &UseStateHandle<String>) -> Callback<MouseEvent> {
    let status = status.clone();
    Callback::from(move |_| {
        status.set("Live execution requires a reachable CORS-configured sw-MLPL server; offline playback remains active.".to_owned())
    })
}
pub(crate) fn fixture_callback(
    fixture: &UseStateHandle<Fixture>,
    source: &UseStateHandle<String>,
    state: &UseStateHandle<ViewerState>,
    status: &UseStateHandle<String>,
) -> Callback<Event> {
    let (fixture, source, state, status) = (
        fixture.clone(),
        source.clone(),
        state.clone(),
        status.clone(),
    );
    Callback::from(move |event: Event| {
        let selected = match event
            .target_unchecked_into::<HtmlInputElement>()
            .value()
            .as_str()
        {
            "regression" => Fixture::Regression,
            "kmeans" => Fixture::Kmeans,
            _ => Fixture::Matrix,
        };
        fixture.set(selected);
        let (selected_source, selected_json) = match selected {
            Fixture::Matrix => (MATRIX_SOURCE, MATRIX_JSON),
            Fixture::Regression => (REGRESSION_SOURCE, REGRESSION_JSON),
            Fixture::Kmeans => (KMEANS_SOURCE, KMEANS_JSON),
        };
        source.set(selected_source.to_owned());
        state.set(loaded_state(selected_json));
        status.set("Offline fixture ready".to_owned());
    })
}
pub(crate) fn keyboard_callback(state: &UseStateHandle<ViewerState>) -> Callback<KeyboardEvent> {
    let state = state.clone();
    Callback::from(move |event: KeyboardEvent| {
        let tag = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .map(|element| element.tag_name())
            .unwrap_or_default();
        if matches!(tag.as_str(), "TEXTAREA" | "INPUT" | "SELECT") {
            return;
        }
        let action = match event.key().as_str() {
            "ArrowLeft" => Some(Action::Previous),
            "ArrowRight" => Some(Action::Next),
            "Home" => Some(Action::SeekIndex(0)),
            "End" => state
                .recording()
                .map(|recording| Action::SeekIndex(recording.frames.len().saturating_sub(1))),
            _ => None,
        };
        if let Some(action) = action {
            event.prevent_default();
            dispatch(&state, action);
        }
    })
}
pub(crate) fn detect_motion() -> MotionPreference {
    web_sys::window()
        .and_then(|window| {
            window
                .match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .filter(web_sys::MediaQueryList::matches)
        .map_or(MotionPreference::Full, |_| MotionPreference::Reduced)
}
