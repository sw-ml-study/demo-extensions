use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers};
use mlpl_native3d_window::live::{
    close_event, input_event, key_event, life_applet_source, parse_scene_command,
};

#[test]
fn mlpl_life_applet_drives_generic_scene_animation_and_teardown() {
    let mut commands = Vec::new();
    let result = run_applet_with_host(&life_applet_source(), |receiver, sender| {
        let Ok(initial) = receiver.recv() else { return };
        commands.push(parse_scene_command(initial).unwrap());
        for event in [
            key_event("b"),
            key_event("space"),
            input_event(InputEvent::frame(1000.0, 1000.0)),
            input_event(InputEvent::wheel(
                [0.0, 40.0],
                [400.0, 300.0],
                Modifiers::NONE,
            )),
        ] {
            sender.send(event).unwrap();
            let Ok(command) = receiver.recv() else { return };
            commands.push(parse_scene_command(command).unwrap());
        }
        sender.send(close_event()).unwrap();
    });

    assert!(result.is_ok(), "Life applet failed: {result:?}");
    assert_eq!(
        commands.len(),
        5,
        "one bounded command per input transition"
    );
    assert_eq!(commands[0].scene.edges().len(), 82);
    assert_eq!(commands[1].scene.edges().len(), 98);
    assert_eq!(commands[3].scene.edges().len(), 98, "block remains stable");
    assert!(commands[4].camera.distance() < commands[0].camera.distance());
    assert!(commands[0].help.contains("CTRL+LEFT DRAG PAINT"));
    assert!(commands[0].help.contains("U GUN"));
}
