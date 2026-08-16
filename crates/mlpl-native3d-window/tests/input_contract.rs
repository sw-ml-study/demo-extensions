use mlpl_eval::Value;
use mlpl_native3d_window::interaction::{
    BoundedInput, FrameGate, InputEvent, Modifiers, PointerButton, PointerButtons,
};
use mlpl_native3d_window::live::input_event;

#[test]
fn bounded_input_coalesces_motion_wheel_and_frame_without_reordering_clicks() {
    let mut events = BoundedInput::new(4).unwrap();
    events
        .push(InputEvent::pointer_move(
            [1.0, 2.0],
            PointerButtons::NONE,
            Modifiers::NONE,
        ))
        .unwrap();
    events
        .push(InputEvent::pointer_move(
            [3.0, 4.0],
            PointerButtons::LEFT,
            Modifiers::NONE,
        ))
        .unwrap();
    events
        .push(InputEvent::pointer_button(
            PointerButton::Left,
            true,
            [3.0, 4.0],
            Modifiers::NONE,
        ))
        .unwrap();
    events
        .push(InputEvent::wheel([0.0, 1.0], [3.0, 4.0], Modifiers::NONE))
        .unwrap();
    events
        .push(InputEvent::wheel([0.0, 2.0], [3.0, 4.0], Modifiers::NONE))
        .unwrap();
    events.push(InputEvent::frame(16.0, 16.0)).unwrap();
    events.push(InputEvent::frame(17.0, 33.0)).unwrap();

    let drained = events.drain();
    assert_eq!(drained.len(), 4);
    assert_eq!(
        drained[0],
        InputEvent::pointer_move([3.0, 4.0], PointerButtons::LEFT, Modifiers::NONE)
    );
    assert!(matches!(
        drained[1],
        InputEvent::PointerButton { pressed: true, .. }
    ));
    assert_eq!(
        drained[2],
        InputEvent::wheel([0.0, 3.0], [3.0, 4.0], Modifiers::NONE)
    );
    assert_eq!(drained[3], InputEvent::frame(17.0, 33.0));
}

#[test]
fn input_contract_rejects_invalid_values_and_capacity_overflow() {
    assert!(BoundedInput::new(0).is_err());
    let mut events = BoundedInput::new(1).unwrap();
    assert!(events.push(InputEvent::frame(f64::NAN, 0.0)).is_err());
    events
        .push(InputEvent::pointer_button(
            PointerButton::Left,
            true,
            [1.0, 1.0],
            Modifiers::NONE,
        ))
        .unwrap();
    assert!(
        events
            .push(InputEvent::pointer_button(
                PointerButton::Left,
                false,
                [1.0, 1.0],
                Modifiers::NONE
            ))
            .is_err()
    );
}

#[test]
fn input_events_cross_as_owned_normalized_mlpl_records() {
    let Value::Record { fields } = input_event(InputEvent::wheel(
        [1.5, -2.0],
        [40.0, 50.0],
        Modifiers::SHIFT,
    )) else {
        panic!("expected record")
    };
    assert_eq!(fields.get("kind"), Some(&Value::Str("wheel".into())));
    assert!(matches!(fields.get("dy"), Some(Value::Array(value)) if value.data() == [-2.0]));
    assert!(matches!(fields.get("shift"), Some(Value::Array(value)) if value.data() == [1.0]));
}

#[test]
fn frame_gate_allows_only_one_outstanding_frame_until_acknowledged() {
    let mut gate = FrameGate::new();
    assert!(gate.begin(), "first frame is admitted");
    for _ in 0..100 {
        assert!(
            !gate.begin(),
            "redraws cannot queue behind an outstanding frame"
        );
    }
    gate.acknowledge();
    assert!(
        gate.begin(),
        "acknowledgement admits exactly one later frame"
    );
}
