use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers, PointerButton, PointerButtons};
use mlpl_native3d_window::live::{
    LiveCommand, applet_source, close_event, input_event, key_event, parse_live_command,
    parse_scene_command, resize_event,
};

#[test]
fn mlpl_worker_drives_scene_commands_from_normalized_events() {
    let mut commands = Vec::new();
    let result = run_applet_with_host(&applet_source(), |receiver, sender| {
        let Ok(initial) = receiver.recv() else { return };
        commands.push(parse_live_command(initial).unwrap());
        sender.send(key_event("w")).unwrap();
        commands.push(parse_live_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("s")).unwrap();
        commands.push(parse_live_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("space")).unwrap();
        commands.push(parse_live_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("space")).unwrap();
        commands.push(parse_live_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("c")).unwrap();
        commands.push(parse_live_command(receiver.recv().unwrap()).unwrap());
        sender.send(resize_event(1024, 768)).unwrap();
        commands.push(parse_live_command(receiver.recv().unwrap()).unwrap());
        sender.send(close_event()).unwrap();
        assert!(matches!(
            receiver.recv_timeout(Duration::from_millis(20)),
            Err(RecvTimeoutError::Disconnected | RecvTimeoutError::Timeout)
        ));
    });
    assert!(result.is_ok(), "applet failed: {result:?}");
    assert_eq!(commands.len(), 7);
    assert!(matches!(commands[0], LiveCommand::Scene(_)));
    assert!(
        commands[1..]
            .iter()
            .all(|command| !matches!(command, LiveCommand::Scene(_)))
    );
    for command in &commands[1..3] {
        let LiveCommand::Patch(patch) = command else {
            panic!("size edits must be retained patches")
        };
        assert_eq!(patch.upserts.len(), 12);
        assert!(patch.remove_ids.is_empty());
    }
    assert!(matches!(commands[3], LiveCommand::View(_)));
    assert!(matches!(commands[4], LiveCommand::View(_)));
    let LiveCommand::Patch(color) = &commands[5] else {
        panic!("color edit must be a retained patch")
    };
    assert_eq!(color.upserts.len(), 12);
    assert!(matches!(commands[6], LiveCommand::View(_)));
}

#[test]
fn mlpl_worker_owns_live_pointer_camera_changes() {
    let mut initial = None;
    let mut views = Vec::new();
    let result = run_applet_with_host(&applet_source(), |receiver, sender| {
        initial = Some(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender
            .send(input_event(InputEvent::pointer_button(
                PointerButton::Left,
                true,
                [100.0, 100.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let LiveCommand::View(view) = parse_live_command(receiver.recv().unwrap()).unwrap() else {
            panic!("pointer press must retain geometry")
        };
        views.push(view);
        sender
            .send(input_event(InputEvent::pointer_move(
                [130.0, 80.0],
                PointerButtons::LEFT,
                Modifiers::NONE,
            )))
            .unwrap();
        let LiveCommand::View(view) = parse_live_command(receiver.recv().unwrap()).unwrap() else {
            panic!("pointer motion must retain geometry")
        };
        views.push(view);
        sender
            .send(input_event(InputEvent::wheel(
                [0.0, 40.0],
                [130.0, 80.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let LiveCommand::View(view) = parse_live_command(receiver.recv().unwrap()).unwrap() else {
            panic!("wheel input must retain geometry")
        };
        views.push(view);
        sender.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "applet failed: {result:?}");
    let initial = initial.unwrap();
    assert_eq!(views.len(), 3);
    assert!((views[1].camera.yaw() - 0.3).abs() < f32::EPSILON);
    assert!((views[1].camera.pitch() - 0.2).abs() < f32::EPSILON);
    assert!(views[2].camera.distance() < initial.camera.distance());
    assert!(initial.help.contains("LEFT DRAG ORBIT/TILT"));
    assert!(initial.help.contains("WHEEL ZOOM"));
    assert!(initial.help.contains("MIDDLE DRAG PAN"));
}
