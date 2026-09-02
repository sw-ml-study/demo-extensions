//! Exact fallback and generic graphic rendering.
use crate::app::ViewerProps;
use mlpl_microscope_model::{Action, ViewerState};
use yew::prelude::*;
pub(crate) fn observation_buttons(props: &ViewerProps) -> Html {
    let Some(frame) = props.state.current_frame() else {
        return html! {<p>{"No frame loaded"}</p>};
    };
    html! {<ul>{for frame.observations.iter().enumerate().map(|(index,observation)|{let handle=props.state_handle.clone();let selected=index==props.state.observation_index();html!{<li><button type="button" class={classes!(selected.then_some("selected"))} onclick={Callback::from(move|_|crate::callbacks::dispatch(&handle,Action::Select(index)))}>{observation.name.replace('/'," › ")}</button></li>}})}</ul>}
}
pub(crate) fn tensor_view(state: &ViewerState) -> Html {
    let Some(observation) = state.current_observation() else {
        return html! {<p>{"Select an observation"}</p>};
    };
    let plan = observation.view_plan();
    let summary = &plan.summary;
    html! {<><p class="eyebrow">{"Selected tensor"}</p><h3>{&observation.name}</h3><p>{format!("Shape {:?} · {} exact value(s)",observation.shape,observation.values.len())}</p><p>{format!("Minimum {} · Maximum {} · Mean {}",number(summary.minimum),number(summary.maximum),number(summary.mean))}</p><div class={classes!("graphic",format!("rank-{}",observation.shape.len()))} aria-hidden="true">{for observation.values.iter().map(|value|html!{<span style={format!("--level:{}",normalized(*value,summary.minimum,summary.maximum))}></span>})}</div><table><caption>{"Exact numeric fallback"}</caption><thead><tr><th>{"Flat index"}</th><th>{"Value"}</th></tr></thead><tbody>{for plan.exact_values.iter().enumerate().map(|(index,value)|html!{<tr><td>{index}</td><td>{format!("{value:.12}")}</td></tr>})}</tbody></table></>}
}
pub(crate) fn control(
    label: &'static str,
    state: UseStateHandle<ViewerState>,
    action: Action,
) -> Html {
    html! {<button type="button" onclick={Callback::from(move|_|crate::callbacks::dispatch(&state,action.clone()))}>{label}</button>}
}
fn number(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |value| format!("{value:.6}"))
}
fn normalized(value: f64, minimum: Option<f64>, maximum: Option<f64>) -> f64 {
    match (minimum, maximum) {
        (Some(low), Some(high)) if high > low => (value - low) / (high - low),
        _ => 0.5,
    }
}
