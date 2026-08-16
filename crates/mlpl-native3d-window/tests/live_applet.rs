use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::live::{
    applet_source, close_event, key_event, parse_scene_command, resize_event,
};

#[test]
fn mlpl_worker_drives_scene_commands_from_normalized_events() {
    let mut commands = Vec::new();
    let result = run_applet_with_host(&applet_source(), |receiver, sender| {
        let Ok(initial) = receiver.recv() else { return };
        commands.push(parse_scene_command(initial).unwrap());
        sender.send(key_event("w")).unwrap();
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("s")).unwrap();
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("space")).unwrap();
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("space")).unwrap();
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender.send(key_event("c")).unwrap();
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender.send(resize_event(1024, 768)).unwrap();
        commands.push(parse_scene_command(receiver.recv().unwrap()).unwrap());
        sender.send(close_event()).unwrap();
        assert!(matches!(
            receiver.recv_timeout(Duration::from_millis(20)),
            Err(RecvTimeoutError::Disconnected | RecvTimeoutError::Timeout)
        ));
    });
    assert!(result.is_ok(), "applet failed: {result:?}");
    assert_eq!(commands.len(), 7);
    assert_eq!(commands[0].revision, 0);
    assert_eq!(commands[1].revision, 1);
    assert_eq!(commands[2].revision, 2);
    assert_eq!(commands[3].revision, 3);
    assert_eq!(commands[4].revision, 4);
    assert_eq!(commands[5].revision, 5);
    assert_eq!(commands[6].revision, 6);
    assert!(
        (commands[0].scene.positions().values()[0] - commands[1].scene.positions().values()[0])
            .abs()
            > f32::EPSILON
    );
    assert!(
        (commands[2].scene.positions().values()[0] - commands[0].scene.positions().values()[0])
            .abs()
            < f32::EPSILON
    );
    assert!(
        commands[3].rotation_speed.abs() < f32::EPSILON,
        "first Space must pause rotation"
    );
    assert!(
        (commands[4].rotation_speed - commands[0].rotation_speed).abs() < f32::EPSILON,
        "second Space must restore the prior signed speed"
    );
    assert!(
        (commands[4].scene.controls().line_color()[0]
            - commands[5].scene.controls().line_color()[0])
            .abs()
            > f32::EPSILON
    );
}
