use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers};
use mlpl_native3d_window::live::{
    LiveCommand, RetainedScene, close_event, input_event, key_event, life_torus_applet_source,
    parse_frame_ack, parse_live_command, parse_scene_command, parse_view_command,
};
use std::time::Duration;

#[test]
fn mlpl_torus_life_applet_wraps_and_drives_the_generic_host() {
    let mut edge_counts = Vec::new();
    let mut initial_help = String::new();
    let result = run_applet_with_host(&life_torus_applet_source(), |receiver, sender| {
        let initial = parse_scene_command(receiver.recv().unwrap()).unwrap();
        initial_help = initial.help.clone();
        let initial_distance = initial.camera.distance();
        let mut retained = RetainedScene::from_scene_command(&initial).unwrap();
        edge_counts.push(retained.scene().edges().len());
        sender.send(key_event("g")).unwrap();
        let LiveCommand::Patch(patch) = parse_live_command(receiver.recv().unwrap()).unwrap()
        else {
            panic!("expected scene patch")
        };
        retained.apply(&patch).unwrap();
        edge_counts.push(retained.scene().edges().len());
        sender.send(key_event("space")).unwrap();
        assert!(receiver.recv_timeout(Duration::from_millis(30)).is_err());
        sender
            .send(input_event(InputEvent::frame(1000.0, 1000.0)))
            .unwrap();
        let LiveCommand::Patch(patch) = parse_live_command(receiver.recv().unwrap()).unwrap()
        else {
            panic!("expected scene patch")
        };
        retained.apply(&patch).unwrap();
        edge_counts.push(retained.scene().edges().len());
        parse_frame_ack(receiver.recv().unwrap()).unwrap();
        sender
            .send(input_event(InputEvent::wheel(
                [0.0, 40.0],
                [520.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let view = parse_view_command(receiver.recv().unwrap()).unwrap();
        assert!(view.camera.distance() < initial_distance);
        sender.send(close_event()).unwrap();
    });

    assert!(result.is_ok(), "toroidal Life applet failed: {result:?}");
    assert_eq!(edge_counts, [1600, 1620, 1620]);
    assert!(initial_help.contains("BOTH GRID AXES WRAP"));
    assert!(initial_help.contains("CTRL+LEFT DRAG PAINT"));
}
