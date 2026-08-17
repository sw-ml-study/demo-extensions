use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers};
use mlpl_native3d_window::live::{
    LiveCommand, RetainedScene, close_event, input_event, key_event, model_atlas_applet_source,
    parse_live_command, parse_scene_command, parse_view_command,
};

#[test]
fn mlpl_model_atlas_filters_and_updates_the_generic_retained_scene() {
    let result = run_applet_with_host(&model_atlas_applet_source(), |receiver, sender| {
        let initial = parse_scene_command(receiver.recv().unwrap()).unwrap();
        assert_eq!(initial.scene.edges().len(), 165);
        assert!(initial.help.contains("MODEL ATLAS"));
        assert!(initial.help.contains("LOG2 BYTE HEIGHT"));
        assert!(initial.help.contains("GGUF ARCH mamba [METADATA]"));
        assert!(initial.help.contains("CYAN=4-BIN HISTOGRAM"));
        assert!(initial.help.contains("NEVER USER MODEL PAYLOAD"));
        let distance = initial.camera.distance();
        let mut retained = RetainedScene::from_scene_command(&initial).unwrap();

        sender.send(key_event("s")).unwrap();
        let LiveCommand::Patch(patch) = parse_live_command(receiver.recv().unwrap()).unwrap()
        else {
            panic!("expected filter patch")
        };
        retained.apply(&patch).unwrap();
        assert_eq!(retained.scene().edges().len(), 117);
        assert!(
            parse_view_command(receiver.recv().unwrap())
                .unwrap()
                .status
                .is_empty()
        );

        sender.send(key_event("s")).unwrap();
        let LiveCommand::Patch(patch) = parse_live_command(receiver.recv().unwrap()).unwrap()
        else {
            panic!("expected toggle-to-all patch")
        };
        retained.apply(&patch).unwrap();
        assert_eq!(retained.scene().edges().len(), 165);
        assert!(
            parse_view_command(receiver.recv().unwrap())
                .unwrap()
                .status
                .is_empty()
        );

        sender.send(key_event("g")).unwrap();
        let LiveCommand::Patch(patch) = parse_live_command(receiver.recv().unwrap()).unwrap()
        else {
            panic!("expected GGUF filter patch")
        };
        retained.apply(&patch).unwrap();
        assert_eq!(retained.scene().edges().len(), 129);
        assert!(
            parse_view_command(receiver.recv().unwrap())
                .unwrap()
                .status
                .is_empty()
        );

        sender.send(key_event("l")).unwrap();
        let LiveCommand::Patch(patch) = parse_live_command(receiver.recv().unwrap()).unwrap()
        else {
            panic!("expected LOD patch")
        };
        retained.apply(&patch).unwrap();
        assert_eq!(retained.scene().edges().len(), 129);
        assert!(
            parse_view_command(receiver.recv().unwrap())
                .unwrap()
                .status
                .is_empty()
        );

        sender
            .send(input_event(InputEvent::wheel(
                [0.0, 40.0],
                [400.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let view = parse_view_command(receiver.recv().unwrap()).unwrap();
        assert!(view.camera.distance() < distance);
        sender.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "Model Atlas applet failed: {result:?}");
}
