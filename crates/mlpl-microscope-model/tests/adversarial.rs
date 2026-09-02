use mlpl_microscope_model::{ValidationKind, parse_recording};
use serde_json::{Value, json};

fn base() -> Value {
    json!({
        "schema": "sw-ml-study.ml-microscope-recording",
        "version": 0,
        "lesson": "generic",
        "budgets": {
            "max_frames": 2,
            "max_observations_per_frame": 2,
            "max_values_per_observation": 4,
            "max_total_values": 8
        },
        "frames": [{"step": 0, "observations": [
            {"name": "path/value", "shape": [2], "values": [1.0, 2.0]}
        ]}]
    })
}

fn kind(value: &Value) -> ValidationKind {
    parse_recording(&value.to_string()).unwrap_err().kind
}

#[test]
fn validation_order_and_structural_rejections_are_stable() {
    let mut unsupported = base();
    unsupported["version"] = json!(1);
    unsupported.as_object_mut().unwrap().remove("frames");
    assert_eq!(kind(&unsupported), ValidationKind::Schema);
    assert_eq!(
        parse_recording(
            r#"{"schema":"sw-ml-study.ml-microscope-recording","schema":"again","version":0}"#
        )
        .unwrap_err()
        .kind,
        ValidationKind::Structure
    );
    let mut empty_name = base();
    empty_name["frames"][0]["observations"][0]["name"] = json!("");
    assert_eq!(kind(&empty_name), ValidationKind::Structure);
}

#[test]
fn numeric_budget_shape_and_step_failures_are_distinct() {
    let mut negative = base();
    negative["frames"][0]["step"] = json!(-1);
    assert_eq!(kind(&negative), ValidationKind::Numeric);

    let mut frame_budget = base();
    frame_budget["budgets"]["max_frames"] = json!(0);
    assert_eq!(kind(&frame_budget), ValidationKind::Budget);

    let mut value_budget = base();
    value_budget["budgets"]["max_values_per_observation"] = json!(1);
    assert_eq!(kind(&value_budget), ValidationKind::Budget);

    let mut mismatch = base();
    mismatch["frames"][0]["observations"][0]["shape"] = json!([3]);
    assert_eq!(kind(&mismatch), ValidationKind::Shape);

    let mut overflow = base();
    overflow["frames"][0]["observations"][0]["shape"] = json!([u64::MAX, 2]);
    assert_eq!(kind(&overflow), ValidationKind::Shape);

    let mut duplicate_step = base();
    let frame = duplicate_step["frames"][0].clone();
    duplicate_step["frames"].as_array_mut().unwrap().push(frame);
    assert_eq!(kind(&duplicate_step), ValidationKind::StepOrder);
}
