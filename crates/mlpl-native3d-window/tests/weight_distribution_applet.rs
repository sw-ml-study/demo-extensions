use mlpl_native3d_window::live::{
    close_event, key_event, parse_scene_command, run_applet_with_host_root,
    weight_distribution_applet_source,
};

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
    let result = run_applet_with_host_root(&source, &root, |commands, events| {
        let Ok(menu_value) = commands.recv_timeout(std::time::Duration::from_secs(2)) else {
            return;
        };
        let menu = parse_scene_command(menu_value).unwrap();
        assert!(menu.help.contains("WEIGHT DISTRIBUTION — MODEL FILES"));
        events.send(key_event("enter")).unwrap();
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
        assert_eq!(histogram.objects.len(), 8 * 12 + 24);
        events.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "weight applet failed: {result:?}");
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
        let tensors = parse_scene_command(commands.recv().unwrap()).unwrap();
        assert!(tensors.help.contains("CHOOSE TENSOR"));
        assert!(tensors.help.contains("[SUPPORTED]"));
        assert!(!tensors.status.contains("CATALOG REJECTED"));
        events.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "real Q8_0 applet failed: {result:?}");
}
