//! Accessible Yew shell over the pure microscope reducer.
use gloo_timers::callback::Interval;
use mlpl_microscope_model::{Action, MotionPreference, Playback, ViewerState};
use web_sys::HtmlInputElement;
use yew::prelude::*;
pub(crate) const MATRIX_JSON: &str =
    include_str!("../../../integration/ml-microscope/fixtures/yew/matrix-run-v0.json");
pub(crate) const REGRESSION_JSON: &str =
    include_str!("../../../integration/ml-microscope/fixtures/yew/linear-regression-run-v0.json");
pub(crate) const MATRIX_SOURCE: &str =
    include_str!("../../../integration/ml-microscope/demos/matrix_microscope.mlpl");
pub(crate) const REGRESSION_SOURCE: &str =
    include_str!("../../../integration/ml-microscope/demos/linear_regression_microscope.mlpl");
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fixture {
    Matrix,
    Regression,
}

#[function_component(App)]
pub fn app() -> Html {
    let fixture = use_state(|| Fixture::Matrix);
    let source = use_state(|| MATRIX_SOURCE.to_owned());
    let state = use_state(|| crate::callbacks::loaded_state(MATRIX_JSON));
    let status = use_state(|| "Offline fixture ready".to_owned());
    let motion = crate::callbacks::detect_motion();
    use_effect_with((*state).playback, {
        let state = state.clone();
        move |playback| {
            let timer = (*playback == Playback::Playing).then(|| {
                Interval::new(700, move || {
                    crate::callbacks::dispatch(&state, Action::Tick)
                })
            });
            move || drop(timer)
        }
    });
    let choose = crate::callbacks::fixture_callback(&fixture, &source, &state, &status);
    let keyboard = crate::callbacks::keyboard_callback(&state);
    let edit = crate::callbacks::source_callback(&source);
    let run = crate::callbacks::run_callback(&status);
    html! {<main class="shell" tabindex="0" onkeydown={keyboard}><header><p class="eyebrow">{"sw-MLPL execution microscope"}</p><h1>{"Inspect the numbers behind each step"}</h1></header><section class="workspace"><aside class="source-panel"><label for="fixture">{"Prerecorded lesson"}</label><select id="fixture" onchange={choose}><option value="matrix">{"Matrix multiplication (MM01)"}</option><option value="regression">{"Linear regression (LR01)"}</option></select><label for="source">{"MLPL source"}</label><textarea id="source" value={(*source).clone()} oninput={edit} spellcheck="false"/><button type="button" onclick={run}>{"Run against configured server"}</button><p class="status" role="status">{(*status).clone()}</p></aside><Viewer state={(*state).clone()} state_handle={state.clone()} motion={motion}/></section><footer>{"Copyright 2026 Software Wrighter LLC · MIT License · github.com/sw-ml-study/demo-extensions · Offline fixture playback; live execution requires an explicit server."}</footer></main>}
}

#[derive(Properties, PartialEq)]
pub(crate) struct ViewerProps {
    pub(crate) state: ViewerState,
    pub(crate) state_handle: UseStateHandle<ViewerState>,
    pub(crate) motion: MotionPreference,
}
#[function_component(Viewer)]
fn viewer(props: &ViewerProps) -> Html {
    let state = &props.state;
    let frame = state.current_frame();
    let count = state
        .recording()
        .map_or(0, |recording| recording.frames.len());
    let lesson = state
        .recording()
        .map_or("No lesson", |recording| recording.lesson.as_str());
    let index = state.frame_index();
    let seek = {
        let handle = props.state_handle.clone();
        Callback::from(move |event: InputEvent| {
            let value = event
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            if value.is_finite() && value >= 0.0 {
                crate::callbacks::dispatch(&handle, Action::SeekIndex(value as usize));
            }
        })
    };
    html! {<section class="viewer" aria-label="Recording viewer"><div class="viewer-heading"><div><p class="eyebrow">{"Lesson"}</p><h2>{lesson}</h2></div><p>{motion_label(props.motion)}</p></div><div class="transport" aria-label="Playback controls">{crate::render::control("Previous",props.state_handle.clone(),Action::Previous)}{crate::render::control(if state.playback==Playback::Playing{"Pause"}else{"Play"},props.state_handle.clone(),if state.playback==Playback::Playing{Action::Pause}else{Action::Play})}{crate::render::control("Next",props.state_handle.clone(),Action::Next)}<label for="seek">{format!("Frame {} of {}; producer step {}",index.saturating_add(1),count,frame.map_or(0,|item|item.step))}</label><input id="seek" type="range" min="0" max={count.saturating_sub(1).to_string()} value={index.to_string()} oninput={seek}/></div><div class="inspection"><nav aria-label="Observations"><h3>{"Observations"}</h3>{crate::render::observation_buttons(props)}</nav><article class="tensor">{crate::render::tensor_view(state)}</article></div></section>}
}
fn motion_label(motion: MotionPreference) -> &'static str {
    if motion == MotionPreference::Reduced {
        "Reduced motion: on"
    } else {
        "Reduced motion: off"
    }
}
