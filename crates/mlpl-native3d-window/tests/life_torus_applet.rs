use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers};
use mlpl_native3d_window::live::{
    close_event, input_event, key_event, life_torus_applet_source, parse_frame_ack,
    parse_scene_command, parse_view_command,
};
use std::time::Duration;

#[test]
fn mlpl_torus_life_applet_wraps_and_drives_the_generic_host() {
    let mut commands = Vec::new();
    let result = run_applet_with_host(&life_torus_applet_source(), |receiver, sender| {
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("g")).unwrap();
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("space")).unwrap();
        assert!(receiver.recv_timeout(Duration::from_millis(30)).is_err());
        sender
            .send(input_event(InputEvent::frame(1000.0, 1000.0)))
            .unwrap();
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        parse_frame_ack(receiver.recv().unwrap()).unwrap();
        sender
            .send(input_event(InputEvent::wheel(
                [0.0, 40.0],
                [520.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let view = parse_view_command(receiver.recv().unwrap()).unwrap();
        assert!(view.camera.distance() < commands[0].camera.distance());
        sender.send(close_event()).unwrap();
    });

    assert!(result.is_ok(), "toroidal Life applet failed: {result:?}");
    assert_eq!(commands[0].scene.edges().len(), 1600);
    assert_eq!(commands[1].scene.edges().len(), 1620);
    assert_eq!(commands[2].scene.edges().len(), 1620);
    assert!(commands[0].help.contains("BOTH GRID AXES WRAP"));
    assert!(commands[0].help.contains("CTRL+LEFT DRAG PAINT"));
}
