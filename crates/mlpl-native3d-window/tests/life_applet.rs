use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers};
use mlpl_native3d_window::live::{
    LiveCommand, RetainedScene, close_event, input_event, key_event, life_applet_source,
    parse_frame_ack, parse_live_command, parse_scene_command, parse_view_command, resync_event,
};
use std::time::Duration;

#[test]
fn mlpl_life_applet_drives_generic_scene_animation_and_teardown() {
    let mut edge_counts = Vec::new();
    let mut initial_help = String::new();
    let result = run_applet_with_host(&life_applet_source(), |receiver, sender| {
        let Ok(initial) = receiver.recv() else { return };
        let initial = parse_scene_command(initial).unwrap();
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
        assert!(
            receiver.recv_timeout(Duration::from_millis(30)).is_err(),
            "nonvisual run toggle must not rebuild the scene"
        );
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
        sender.send(resync_event()).unwrap();
        let resynced = parse_scene_command(receiver.recv().unwrap()).unwrap();
        assert_eq!(resynced.revision, retained.revision());
        assert_eq!(resynced.scene.edges().len(), retained.scene().edges().len());
        sender
            .send(input_event(InputEvent::wheel(
                [0.0, 40.0],
                [400.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let view = parse_view_command(receiver.recv().unwrap()).unwrap();
        assert!(view.camera.distance() < initial_distance);
        sender.send(close_event()).unwrap();
    });

    assert!(result.is_ok(), "Life applet failed: {result:?}");
    assert_eq!(edge_counts.len(), 3, "only geometry changes update scenes");
    assert_eq!(edge_counts[0], 82);
    assert_eq!(edge_counts[1], 102);
    assert_eq!(edge_counts[2], 102, "glider retains five cells");
    assert!(initial_help.contains("CTRL+LEFT DRAG PAINT"));
    assert!(initial_help.contains("U GUN"));
}
