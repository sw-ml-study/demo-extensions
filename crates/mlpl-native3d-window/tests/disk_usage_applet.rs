use mlpl_native3d_window::disk_usage::{SnapshotBudgets, capture_snapshot};
use mlpl_native3d_window::live::{
    close_event, disk_usage_applet_source, key_event, parse_scene_command,
    parse_scene_patch_command, parse_view_command, run_applet_with_host_root,
};

#[test]
fn bounded_snapshot_drives_read_only_mlpl_applet() {
    let root = std::env::temp_dir().join(format!("disk-usage-applet-{}", std::process::id()));
    std::fs::create_dir_all(root.join("large")).unwrap();
    std::fs::write(root.join("large/data.bin"), [0_u8; 32]).unwrap();
    std::fs::write(root.join("small.bin"), [0_u8; 4]).unwrap();
    let snapshot = capture_snapshot(
        &root,
        SnapshotBudgets {
            max_entries: 64,
            max_depth: 8,
        },
    )
    .unwrap();
    let source = disk_usage_applet_source(&snapshot);

    let result = run_applet_with_host_root(&source, &root, |commands, events| {
        let Ok(command) = commands.recv_timeout(std::time::Duration::from_secs(2)) else {
            return;
        };
        let scene = parse_scene_command(command).expect("valid scene command");
        assert!(scene.help.contains("READ-ONLY SNAPSHOT"));
        assert!(
            scene
                .help
                .contains("NO REFRESH, DELETE, MOVE, RENAME, OR WRITE")
        );
        assert!(scene.status.contains("large"));
        assert!(scene.status.contains("SELECTED KIND DIR"));
        events.send(key_event("arrow_right")).unwrap();
        let Ok(drilled_value) = commands.recv() else {
            return;
        };
        let drilled_patch = parse_scene_patch_command(drilled_value).unwrap();
        assert_eq!(drilled_patch.upserts.len(), 16 * 12);
        let drilled = parse_view_command(commands.recv().unwrap()).unwrap();
        assert!(drilled.status.contains("BREADCRUMB /large"));
        events.send(key_event("arrow_left")).unwrap();
        let parent_patch = parse_scene_patch_command(commands.recv().unwrap()).unwrap();
        assert_eq!(parent_patch.upserts.len(), 16 * 12);
        let parent = parse_view_command(commands.recv().unwrap()).unwrap();
        assert!(parent.status.contains("BREADCRUMB / |"));
        assert!(parent.status.contains("large"));
        events.send(key_event("enter")).unwrap();
        assert!(parse_scene_patch_command(commands.recv().unwrap()).is_ok());
        assert!(
            parse_view_command(commands.recv().unwrap())
                .unwrap()
                .status
                .contains("BREADCRUMB /large")
        );
        events.send(key_event("backspace")).unwrap();
        assert!(parse_scene_patch_command(commands.recv().unwrap()).is_ok());
        assert!(
            parse_view_command(commands.recv().unwrap())
                .unwrap()
                .status
                .contains("BREADCRUMB / |")
        );
        events.send(close_event()).unwrap();
    });
    assert!(result.is_ok(), "disk-usage applet failed: {result:?}");
    std::fs::remove_dir_all(root).ok();
}
