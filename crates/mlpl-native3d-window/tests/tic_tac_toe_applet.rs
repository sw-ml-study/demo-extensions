use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers, PointerButton};
use mlpl_native3d_window::live::{
    LiveCommand, close_event, input_event, key_event, parse_live_command, tic_tac_toe_applet_source,
};

#[test]
fn mlpl_tic_tac_toe_click_drives_a_variable_styled_scene() {
    let mut commands = Vec::new();
    let result = run_applet_with_host(&tic_tac_toe_applet_source(), |receiver, sender| {
        let Ok(initial) = receiver.recv() else { return };
        commands.push(parse_live_command(initial).unwrap());
        sender
            .send(input_event(InputEvent::pointer_button(
                PointerButton::Left,
                false,
                [400.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let Ok(updated) = receiver.recv() else { return };
        commands.push(parse_live_command(updated).unwrap());
        sender.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "tic-tac-toe applet failed: {result:?}");
    let LiveCommand::Scene(initial) = &commands[0] else {
        panic!("initial scene")
    };
    assert_eq!(initial.scene.edges().len(), 4);
    let LiveCommand::Patch(patch) = &commands[1] else {
        panic!("move must use a retained patch")
    };
    assert!(patch.upserts.len() <= 14);
    assert!(initial.help.contains("CLICK EMPTY SQUARE"));
    assert!(initial.help.contains("X/O MARK"));
}

#[test]
fn choosing_o_then_clicking_center_keeps_the_applet_alive() {
    let mut commands = Vec::new();
    let result = run_applet_with_host(&tic_tac_toe_applet_source(), |receiver, sender| {
        let Ok(initial) = receiver.recv() else { return };
        commands.push(parse_live_command(initial).unwrap());
        sender.send(key_event("o")).unwrap();
        let Ok(chosen) = receiver.recv() else { return };
        commands.push(parse_live_command(chosen).unwrap());
        sender
            .send(input_event(InputEvent::pointer_button(
                PointerButton::Left,
                false,
                [400.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let Ok(played) = receiver.recv() else { return };
        commands.push(parse_live_command(played).unwrap());
        sender.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "O-first applet failed: {result:?}");
    assert_eq!(commands.len(), 3);
    let LiveCommand::View(chosen) = &commands[1] else {
        panic!("role-only change must retain geometry")
    };
    assert!(chosen.help.contains("YOU ARE O / FIRST"));
    assert!(matches!(commands[2], LiveCommand::Patch(_)));
}

#[test]
fn camera_drag_changes_view_and_suppresses_board_click() {
    let mut commands = Vec::new();
    let result = run_applet_with_host(&tic_tac_toe_applet_source(), |receiver, sender| {
        let Ok(initial) = receiver.recv() else { return };
        commands.push(parse_live_command(initial).unwrap());
        for event in [
            InputEvent::pointer_button(PointerButton::Left, true, [400.0, 300.0], Modifiers::NONE),
            InputEvent::pointer_move(
                [430.0, 280.0],
                mlpl_native3d_window::interaction::PointerButtons::LEFT,
                Modifiers::NONE,
            ),
            InputEvent::pointer_button(PointerButton::Left, false, [430.0, 280.0], Modifiers::NONE),
        ] {
            sender.send(input_event(event)).unwrap();
            let Ok(value) = receiver.recv() else { return };
            commands.push(parse_live_command(value).unwrap());
        }
        sender.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "camera drag applet failed: {result:?}");
    assert_eq!(commands.len(), 4);
    assert!(
        commands[1..]
            .iter()
            .all(|command| matches!(command, LiveCommand::View(_)))
    );
    let LiveCommand::Scene(initial) = &commands[0] else {
        panic!("initial scene")
    };
    let LiveCommand::View(released) = &commands[3] else {
        panic!("release view")
    };
    assert!((released.camera.yaw() - initial.camera.yaw()).abs() > f32::EPSILON);
    assert!(initial.help.contains("LEFT DRAG ORBIT/TILT"));
    assert!(initial.help.contains("DRAG NEVER PLACES A MARK"));
}
