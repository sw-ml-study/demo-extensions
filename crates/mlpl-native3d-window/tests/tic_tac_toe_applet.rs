use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers, PointerButton};
use mlpl_native3d_window::live::{
    close_event, input_event, key_event, parse_scene_command, tic_tac_toe_applet_source,
};

#[test]
fn mlpl_tic_tac_toe_click_drives_a_variable_styled_scene() {
    let mut commands = Vec::new();
    let result = run_applet_with_host(&tic_tac_toe_applet_source(), |receiver, sender| {
        let Ok(initial) = receiver.recv() else { return };
        commands.push(parse_scene_command(initial).unwrap());
        sender
            .send(input_event(InputEvent::pointer_button(
                PointerButton::Left,
                false,
                [400.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let Ok(updated) = receiver.recv() else { return };
        commands.push(parse_scene_command(updated).unwrap());
        sender.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "tic-tac-toe applet failed: {result:?}");
    assert_eq!(commands[0].scene.edges().len(), 4);
    assert_eq!(commands[1].scene.edges().len(), 14);
    assert!(commands[0].help.contains("CLICK AN EMPTY SQUARE"));
    assert!(commands[0].help.contains("X/O CHOOSE MARK"));
}

#[test]
fn choosing_o_then_clicking_center_keeps_the_applet_alive() {
    let mut commands = Vec::new();
    let result = run_applet_with_host(&tic_tac_toe_applet_source(), |receiver, sender| {
        let Ok(initial) = receiver.recv() else { return };
        commands.push(parse_scene_command(initial).unwrap());
        sender.send(key_event("o")).unwrap();
        let Ok(chosen) = receiver.recv() else { return };
        commands.push(parse_scene_command(chosen).unwrap());
        sender
            .send(input_event(InputEvent::pointer_button(
                PointerButton::Left,
                false,
                [400.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let Ok(played) = receiver.recv() else { return };
        commands.push(parse_scene_command(played).unwrap());
        sender.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "O-first applet failed: {result:?}");
    assert_eq!(commands.len(), 3);
    assert!(commands[1].help.contains("YOU ARE O / FIRST"));
    assert_eq!(commands[2].scene.edges().len(), 14);
}
