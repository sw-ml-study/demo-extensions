use mlpl_native3d_window::audio::PcmChunk;
use mlpl_native3d_window::live::{
    LiveCommand, audio_chunk_event, audio_spectrum_applet_source, close_event, key_event,
    parse_live_command, parse_scene_command, parse_scene_patch_command, parse_view_command,
    run_applet_with_host_root,
};

#[test]
fn mlpl_picker_analyzes_one_bounded_chunk_and_keeps_input_live() {
    let root = std::env::temp_dir().join(format!("audio-spectrum-applet-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("tone.mp3"), [0_u8; 4]).unwrap();
    let source = audio_spectrum_applet_source(&["tone.mp3".into(), "tone.ogg".into()]);
    let result = run_applet_with_host_root(&source, &root, |commands, events| {
        let scene = parse_scene_command(commands.recv().unwrap()).unwrap();
        assert!(scene.help.contains(">> tone.mp3"));
        events.send(key_event("enter")).unwrap();
        assert!(matches!(
            parse_live_command(commands.recv().unwrap()).unwrap(),
            LiveCommand::AudioOpen(path) if path == "tone.mp3"
        ));
        assert!(parse_view_command(commands.recv().unwrap()).is_ok());
        events
            .send(audio_chunk_event(&PcmChunk {
                left: vec![0.0; 64],
                right: vec![0.0; 64],
                sample_rate_hz: 44_100,
                start_frame: 0,
            }))
            .unwrap();
        assert_eq!(
            parse_scene_patch_command(commands.recv().unwrap())
                .unwrap()
                .upserts
                .len(),
            16
        );
        assert!(parse_view_command(commands.recv().unwrap()).is_ok());
        assert!(matches!(
            parse_live_command(commands.recv().unwrap()).unwrap(),
            LiveCommand::AudioAck
        ));
        events.send(key_event("space")).unwrap();
        assert!(matches!(
            parse_live_command(commands.recv().unwrap()).unwrap(),
            LiveCommand::AudioPlay(false)
        ));
        assert!(parse_view_command(commands.recv().unwrap()).is_ok());
        events.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "audio applet failed: {result:?}");
    std::fs::remove_dir_all(root).ok();
}
