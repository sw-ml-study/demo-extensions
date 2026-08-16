use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers};
use mlpl_native3d_window::live::{
    close_event, input_event, key_event, life_applet_source, parse_scene_command,
    parse_view_command,
};
use std::time::Duration;

#[test]
fn mlpl_life_applet_drives_generic_scene_animation_and_teardown() {
    let mut commands = Vec::new();
    let result = run_applet_with_host(&life_applet_source(), |receiver, sender| {
        let Ok(initial) = receiver.recv() else { return };
        commands.push(parse_scene_command(initial).unwrap());
        sender.send(key_event("g")).unwrap();
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("space")).unwrap();
        assert!(
            receiver.recv_timeout(Duration::from_millis(30)).is_err(),
            "nonvisual run toggle must not rebuild the scene"
        );
        sender
            .send(input_event(InputEvent::frame(1000.0, 1000.0)))
            .unwrap();
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender
            .send(input_event(InputEvent::wheel(
                [0.0, 40.0],
                [400.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let view = parse_view_command(receiver.recv().unwrap()).unwrap();
        assert!(view.camera.distance() < commands[0].camera.distance());
        sender.send(close_event()).unwrap();
    });

    assert!(result.is_ok(), "Life applet failed: {result:?}");
    assert_eq!(commands.len(), 3, "only geometry changes emit scenes");
    assert_eq!(commands[0].scene.edges().len(), 82);
    assert_eq!(commands[1].scene.edges().len(), 102);
    assert_eq!(
        commands[2].scene.edges().len(),
        102,
        "glider retains five cells"
    );
    assert!(commands[0].help.contains("CTRL+LEFT DRAG PAINT"));
    assert!(commands[0].help.contains("U GUN"));
}
