use mlpl_native3d_window::interaction::{InputEvent, Modifiers, PointerButton};
use mlpl_native3d_window::live::{
    LiveCommand, close_event, input_event, key_event, parse_live_command, parse_scene_command,
    run_applet_with_host_root, weight_distribution_applet_source,
};

fn orbit_then_select_bar(
    commands: &std::sync::mpsc::Receiver<mlpl_eval::Value>,
    events: &std::sync::mpsc::Sender<mlpl_eval::Value>,
) -> bool {
    for event in [
        InputEvent::pointer_button(PointerButton::Left, true, [420.0, 320.0], Modifiers::NONE),
        InputEvent::pointer_move(
            [475.0, 350.0],
            mlpl_native3d_window::interaction::PointerButtons::LEFT,
            Modifiers::NONE,
        ),
        InputEvent::pointer_button(PointerButton::Left, false, [475.0, 350.0], Modifiers::NONE),
    ] {
        events.send(input_event(event)).unwrap();
        parse_live_command(commands.recv().unwrap()).unwrap();
    }
    for y in (180..=620).step_by(40) {
        for x in (120..=900).step_by(40) {
            events
                .send(input_event(InputEvent::pointer_button(
                    PointerButton::Left,
                    true,
                    [f64::from(x), f64::from(y)],
                    Modifiers::NONE,
                )))
                .unwrap();
            parse_live_command(commands.recv().unwrap()).unwrap();
            events
                .send(input_event(InputEvent::pointer_button(
                    PointerButton::Left,
                    false,
                    [f64::from(x), f64::from(y)],
                    Modifiers::NONE,
                )))
                .unwrap();
            let released = parse_live_command(commands.recv().unwrap()).unwrap();
            if matches!(released, LiveCommand::Patch(_)) {
                let Ok(view) = commands.recv() else {
                    return false;
                };
                parse_live_command(view).unwrap();
                return true;
            }
        }
    }
    false
}

#[test]
fn bounded_real_safetensors_reaches_a_retained_histogram() {
    let root = std::env::temp_dir().join(format!("weight-distribution-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let header = br#"{"weights":{"dtype":"I8","shape":[8],"data_offsets":[0,8]}}"#;
    let mut artifact = (header.len() as u64).to_le_bytes().to_vec();
    artifact.extend_from_slice(header);
    artifact.extend_from_slice(&[252, 254, 255, 0, 0, 1, 2, 4]);
    std::fs::write(root.join("weights.safetensors"), artifact).unwrap();

    let source = weight_distribution_applet_source(&["weights.safetensors".into()]);
    let mut survived_click = false;
    let result = run_applet_with_host_root(&source, &root, |commands, events| {
        let Ok(menu_value) = commands.recv_timeout(std::time::Duration::from_secs(2)) else {
            return;
        };
        let menu = parse_scene_command(menu_value).unwrap();
        assert!(menu.help.contains("WEIGHT DISTRIBUTION — MODEL FILES"));
        events.send(key_event("enter")).unwrap();
        let loading = parse_scene_command(commands.recv().unwrap()).unwrap();
        assert!(loading.help.contains("LOADING BOUNDED MODEL CATALOG"));
        assert!((loading.rotation_speed - 2.0).abs() < f32::EPSILON);
        let Ok(tensor_value) = commands.recv_timeout(std::time::Duration::from_secs(2)) else {
            return;
        };
        let tensors = parse_scene_command(tensor_value).unwrap();
        assert!(tensors.help.contains("CHOOSE TENSOR"));
        events.send(key_event("enter")).unwrap();
        let Ok(histogram_value) = commands.recv_timeout(std::time::Duration::from_secs(2)) else {
            return;
        };
        let histogram = parse_scene_command(histogram_value).unwrap();
        assert!(histogram.help.contains("X=WEIGHT VALUE"));
        assert!(histogram.help.contains("Y=LOG2 SAMPLE COUNT"));
        assert!(histogram.status.contains("SAMPLED 8 / 8"));
        assert!(histogram.objects.len() > 8 * 12 + 24);
        events
            .send(input_event(InputEvent::pointer_button(
                PointerButton::Left,
                true,
                [400.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        parse_live_command(commands.recv().unwrap()).unwrap();
        events
            .send(input_event(InputEvent::pointer_button(
                PointerButton::Left,
                false,
                [400.0, 300.0],
                Modifiers::NONE,
            )))
            .unwrap();
        let released = parse_live_command(commands.recv().unwrap()).unwrap();
        if matches!(released, LiveCommand::Patch(_)) {
            parse_live_command(commands.recv().unwrap()).unwrap();
        }
        events
            .send(input_event(InputEvent::pointer_move(
                [401.0, 300.0],
                mlpl_native3d_window::interaction::PointerButtons::NONE,
                Modifiers::NONE,
            )))
            .unwrap();
        if let Ok(command) = commands.recv_timeout(std::time::Duration::from_secs(2)) {
            parse_live_command(command).unwrap();
            survived_click = true;
        }
        events.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "weight applet failed: {result:?}");
    assert!(
        survived_click,
        "click must leave a state that accepts the next key"
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn model_discovery_is_sorted_bounded_and_format_limited() {
    let root = std::env::temp_dir().join(format!("weight-discovery-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("b.gguf"), b"GGUF").unwrap();
    std::fs::write(root.join("a.safetensors"), b"safe").unwrap();
    std::fs::write(root.join("ignored.bin"), b"bin").unwrap();
    let found = mlpl_native3d_window::model_files::discover_model_paths(&root, 1).unwrap();
    assert_eq!(found, ["a.safetensors"]);
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn shared_q8_0_fixture_decodes_one_bounded_block() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../demo-ml-utils/fixtures/gguf")
        .canonicalize()
        .unwrap();
    let source = weight_distribution_applet_source(&["valid-catalog.gguf".into()]);
    let result = run_applet_with_host_root(&source, &root, |commands, events| {
        parse_scene_command(commands.recv().unwrap()).unwrap();
        events.send(key_event("enter")).unwrap();
        let loading = parse_scene_command(commands.recv().unwrap()).unwrap();
        assert!(loading.help.contains("LOADING BOUNDED MODEL CATALOG"));
        parse_scene_command(commands.recv().unwrap()).unwrap();
        events.send(key_event("arrow_down")).unwrap();
        let selected = parse_scene_command(commands.recv().unwrap()).unwrap();
        assert!(selected.status.contains("quant"));
        assert!(selected.status.contains("DTYPE 8"));
        events.send(key_event("enter")).unwrap();
        let histogram = parse_scene_command(commands.recv().unwrap()).unwrap();
        assert!(histogram.status.contains("SAMPLED 32 / 32"));
        assert!(histogram.status.contains("READ 34 B"));
        events.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "Q8_0 applet failed: {result:?}");
}

#[test]
#[ignore = "set WEIGHT_MODEL_ROOT to opt into a downloaded real-model acceptance"]
fn downloaded_real_q8_0_model_reaches_supported_tensor_menu() {
    let root = std::path::PathBuf::from(
        std::env::var("WEIGHT_MODEL_ROOT").expect("WEIGHT_MODEL_ROOT must be absolute"),
    );
    let paths = mlpl_native3d_window::model_files::discover_model_paths(&root, 4).unwrap();
    assert!(!paths.is_empty(), "real-model root has no GGUF input");
    let source = weight_distribution_applet_source(&paths);
    let result = run_applet_with_host_root(&source, &root, |commands, events| {
        parse_scene_command(commands.recv().unwrap()).unwrap();
        events.send(key_event("enter")).unwrap();
        let loading = parse_scene_command(commands.recv().unwrap()).unwrap();
        assert!(loading.help.contains("LOADING BOUNDED MODEL CATALOG"));
        let mut tensors = parse_scene_command(commands.recv().unwrap()).unwrap();
        assert!(tensors.help.contains("CHOOSE TENSOR"));
        assert!(tensors.help.contains("[SUPPORTED]"));
        assert!(!tensors.status.contains("CATALOG REJECTED"));
        for _ in 0..12 {
            if tensors
                .help
                .lines()
                .any(|line| line.starts_with(">> ") && line.contains("[SUPPORTED]"))
            {
                break;
            }
            events.send(key_event("arrow_down")).unwrap();
            tensors = parse_scene_command(commands.recv().unwrap()).unwrap();
        }
        assert!(
            tensors
                .help
                .lines()
                .any(|line| line.starts_with(">> ") && line.contains("[SUPPORTED]")),
            "real model menu must expose a selectable supported tensor"
        );
        events.send(key_event("enter")).unwrap();
        let histogram = parse_scene_command(commands.recv().unwrap()).unwrap();
        assert!(histogram.help.contains("BOUNDED SAMPLE HISTOGRAM"));
        for _ in 0..7 {
            events.send(key_event("arrow_down")).unwrap();
            assert!(matches!(
                parse_live_command(commands.recv().unwrap()).unwrap(),
                LiveCommand::Patch(_)
            ));
            parse_live_command(commands.recv().unwrap()).unwrap();
        }
        let selected_a_bar = orbit_then_select_bar(&commands, &events);
        assert!(
            selected_a_bar,
            "screen-space click scan must select a real bar"
        );
        events
            .send(input_event(InputEvent::pointer_move(
                [401.0, 300.0],
                mlpl_native3d_window::interaction::PointerButtons::NONE,
                Modifiers::NONE,
            )))
            .unwrap();
        parse_live_command(commands.recv().unwrap()).unwrap();
        events.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "real Q8_0 applet failed: {result:?}");
}
