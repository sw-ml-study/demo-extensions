use mlpl_eval::run_applet_with_host;
use mlpl_native3d_window::interaction::{InputEvent, Modifiers};
use mlpl_native3d_window::live::{
    LiveCommand, close_event, input_event, key_event, model_atlas_file_applet_source,
    parse_live_command, parse_scene_command, parse_view_command, run_applet_with_host_root,
};

#[test]
fn real_file_menu_opens_a_bounded_safetensors_catalog_and_returns() {
    let root = std::env::temp_dir().join(format!("atlas-file-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let header = br#"{"tiny.weight":{"dtype":"U8","shape":[4],"data_offsets":[0,4]}}"#;
    let mut artifact = (header.len() as u64).to_le_bytes().to_vec();
    artifact.extend_from_slice(header);
    artifact.extend_from_slice(&[1, 2, 3, 4]);
    std::fs::write(root.join("tiny.safetensors"), &artifact).unwrap();
    std::fs::write(root.join("z-second.safetensors"), artifact).unwrap();

    let result = run_applet_with_host_root(
        &model_atlas_file_applet_source(),
        &root,
        |commands, events| {
            let Ok(menu_value) = commands.recv_timeout(std::time::Duration::from_secs(2)) else {
                return;
            };
            let menu = parse_scene_command(menu_value)
                .unwrap_or_else(|error| panic!("expected initial file menu command: {error}"));
            assert!(menu.help.contains("REAL LOCAL SAFETENSORS"));
            assert!(menu.help.contains("tiny.safetensors"));
            assert!(menu.status.contains("SELECTED [1/2]"));
            assert!(menu.status.contains("tiny.safetensors"));
            assert_eq!(menu.scene.edges().len(), 4);

            events.send(key_event("arrow_down")).unwrap();
            let second = parse_view_command(commands.recv().unwrap()).unwrap();
            assert!(second.status.contains("SELECTED [2/2]"));
            assert!(second.status.contains("z-second.safetensors"));
            events.send(key_event("arrow_up")).unwrap();
            let first_again = parse_view_command(commands.recv().unwrap()).unwrap();
            assert!(first_again.status.contains("SELECTED [1/2]"));

            events.send(key_event("enter")).unwrap();
            let LiveCommand::Patch(scanning_patch) =
                parse_live_command(commands.recv().unwrap()).unwrap()
            else {
                panic!("menu transition must be a retained patch")
            };
            assert!(scanning_patch.upserts.is_empty());
            let scanning = parse_view_command(commands.recv().unwrap()).unwrap();
            assert!(scanning.help.contains("ANALYZING BOUNDED"));
            assert!(scanning.status.contains("tiny.safetensors"));
            let atlas_patch = commands
                .recv_timeout(std::time::Duration::from_secs(1))
                .expect("atlas patch");
            let LiveCommand::Patch(atlas_patch) = parse_live_command(atlas_patch).unwrap() else {
                panic!("analyzed atlas must be a retained patch")
            };
            assert!(atlas_patch.upserts.len() > 12);
            let atlas = parse_view_command(commands.recv().unwrap()).unwrap();
            assert!(atlas.help.contains("ONE WIREFRAME BAR PER TENSOR"));
            assert!(atlas.help.contains("CATALOG TENSORS: 1"));
            assert!(atlas.help.contains("LOG2(STORED BYTES)/4"));
            assert!(atlas.help.contains("BLUE T=TENSOR"));

            events
                .send(input_event(InputEvent::Wheel {
                    delta: [0.0, 120.0],
                    position: [400.0, 300.0],
                    modifiers: Modifiers::from_flags([false; 4]),
                }))
                .unwrap();
            let zoomed = parse_view_command(commands.recv().unwrap()).unwrap();
            assert!((zoomed.camera.distance() - atlas.camera.distance()).abs() > f32::EPSILON);

            events.send(key_event("r")).unwrap();
            let reset = parse_view_command(commands.recv().unwrap()).unwrap();
            assert!((reset.camera.distance() - atlas.camera.distance()).abs() < f32::EPSILON);

            events.send(key_event("m")).unwrap();
            let LiveCommand::Patch(menu_patch) =
                parse_live_command(commands.recv().unwrap()).unwrap()
            else {
                panic!("menu return must be a retained patch")
            };
            assert!(!menu_patch.remove_ids.is_empty());
            let menu_again = parse_view_command(commands.recv().unwrap()).unwrap();
            assert!(menu_again.help.contains("REAL LOCAL SAFETENSORS"));
            events.send(close_event()).unwrap();
        },
    );
    assert!(result.is_ok(), "file applet failed: {result:?}");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn ordinary_unrooted_applet_still_has_no_filesystem_authority() {
    let source = "err_message(fs_walk(\".\",{recursive:0,kind:\"file\",pattern:\"*\"}))";
    let result = run_applet_with_host(source, |_commands, _events| {});
    assert!(matches!(result, Ok(mlpl_eval::Value::Str(message)) if message.contains("sandbox")));
}
