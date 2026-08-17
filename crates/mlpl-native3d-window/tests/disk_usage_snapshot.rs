use mlpl_native3d_window::disk_usage::{SnapshotBudgets, capture_snapshot};

#[test]
fn snapshot_is_metadata_only_deterministic_and_bounded() {
    let root = std::env::temp_dir().join(format!("disk-usage-snapshot-{}", std::process::id()));
    std::fs::create_dir_all(root.join("alpha/nested")).unwrap();
    std::fs::create_dir_all(root.join("beta")).unwrap();
    std::fs::write(root.join("alpha/a.bin"), [0_u8; 5]).unwrap();
    std::fs::write(root.join("alpha/nested/b.bin"), [0_u8; 7]).unwrap();
    std::fs::write(root.join("beta/c.bin"), [0_u8; 11]).unwrap();

    let snapshot = capture_snapshot(
        &root,
        SnapshotBudgets {
            max_entries: 4,
            max_depth: 8,
        },
    )
    .unwrap();
    assert_eq!(snapshot.entries.len(), 4);
    assert_eq!(snapshot.entries[0].path, "alpha");
    assert_eq!(snapshot.entries[1].path, "beta");
    assert_eq!(snapshot.entries[2].path, "alpha/a.bin");
    assert!(snapshot.excluded_entries > 0);
    assert_eq!(
        snapshot.excluded_bytes, 0,
        "uninspected bytes are not guessed"
    );
    assert_eq!(snapshot.inaccessible_entries, 0);

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn snapshot_depth_budget_accounts_for_excluded_descendants() {
    let root = std::env::temp_dir().join(format!("disk-usage-depth-{}", std::process::id()));
    std::fs::create_dir_all(root.join("one/two")).unwrap();
    std::fs::write(root.join("one/two/data.bin"), [0_u8; 13]).unwrap();

    let snapshot = capture_snapshot(
        &root,
        SnapshotBudgets {
            max_entries: 20,
            max_depth: 1,
        },
    )
    .unwrap();
    assert!(snapshot.entries.iter().any(|entry| entry.path == "one"));
    assert!(snapshot.entries.iter().any(|entry| entry.path == "one/two"));
    assert!(
        !snapshot
            .entries
            .iter()
            .any(|entry| entry.path == "one/two/data.bin")
    );
    assert_eq!(
        snapshot.excluded_bytes, 0,
        "depth-excluded bytes remain explicitly unknown"
    );

    std::fs::remove_dir_all(root).ok();
}
