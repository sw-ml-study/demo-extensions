use std::time::Duration;

use mlpl_eval::run_applet_with_host;
use mlpl_native3d_scene::Viewport;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers};
use mlpl_native3d_window::live::{
    LiveCommand, RetainedPointScene, close_event, input_event, parse_frame_ack, parse_live_command,
    point_cloud_applet_source, point_selection_event,
};

#[test]
fn mlpl_point_cloud_drives_complete_patch_view_selection_and_teardown() {
    let mut selected_id = None;
    let mut help = String::new();
    let result = run_applet_with_host(&point_cloud_applet_source(), |receiver, sender| {
        let LiveCommand::PointScene(initial) =
            parse_live_command(receiver.recv().unwrap()).unwrap()
        else {
            panic!("expected complete point scene")
        };
        help = initial.help.clone();
        assert_eq!(initial.scene.len(), 24);
        let mut retained = RetainedPointScene::from_command(&initial).unwrap();
        let viewport = Viewport::new(800, 600).unwrap();
        let plan = retained
            .scene()
            .plan_points(initial.camera, viewport, 0.0)
            .unwrap();
        let target = plan.points()[0];
        selected_id = Some(target.id());
        sender
            .send(
                point_selection_event(
                    retained.scene(),
                    initial.camera,
                    viewport,
                    0.0,
                    target.center().map(f64::from),
                    retained.revision(),
                )
                .unwrap(),
            )
            .unwrap();
        let LiveCommand::PointPatch(patch) = parse_live_command(receiver.recv().unwrap()).unwrap()
        else {
            panic!("selection must emit a point patch")
        };
        retained.apply(&patch).unwrap();
        assert_eq!(retained.revision(), 1);

        sender
            .send(input_event(InputEvent::wheel(
                [0.0, 30.0],
                [400.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let LiveCommand::View(view) = parse_live_command(receiver.recv().unwrap()).unwrap() else {
            panic!("camera change must emit a view command")
        };
        assert!(view.camera.distance() < initial.camera.distance());

        sender
            .send(input_event(InputEvent::frame(16.0, 16.0)))
            .unwrap();
        parse_frame_ack(receiver.recv().unwrap()).unwrap();
        assert!(receiver.recv_timeout(Duration::from_millis(20)).is_err());
        sender.send(close_event()).unwrap();
    });

    assert!(result.is_ok(), "point-cloud applet failed: {result:?}");
    assert!(selected_id.is_some());
    assert!(help.contains("MLPL OWNS XYZ / SIZE / COLOR / OPACITY / ID ARRAYS"));
    assert!(help.contains("NOT EMBEDDINGS"));
}
