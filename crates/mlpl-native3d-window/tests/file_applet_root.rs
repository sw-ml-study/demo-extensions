use mlpl_eval::Value;
use mlpl_native3d_window::live::run_applet_with_host_root;

#[test]
fn rooted_applet_discovers_only_contained_model_paths() {
    let root = std::env::temp_dir().join(format!("atlas-root-{}", std::process::id()));
    std::fs::create_dir_all(root.join("nested")).unwrap();
    std::fs::write(root.join("a.safetensors"), [0_u8; 8]).unwrap();
    std::fs::write(root.join("nested/b.gguf"), [0_u8; 8]).unwrap();
    std::fs::write(root.join("ignore.txt"), b"no").unwrap();
    let source = r#"
safe=fs_walk(".",{recursive:1,kind:"file",pattern:"*.safetensors"})?;
gguf=fs_walk(".",{recursive:1,kind:"file",pattern:"*.gguf"})?;
port_send(port,{safe: safe,gguf: gguf}); ok(1)
"#;
    let mut command = None;
    let result = run_applet_with_host_root(source, &root, |commands, _events| {
        command = commands.recv().ok();
    });
    assert!(result.is_ok(), "rooted applet failed: {result:?}");
    let Value::Record { fields } = command.expect("expected discovery record") else {
        panic!("expected discovery record")
    };
    assert!(matches!(&fields["safe"], Value::StrList { items } if items == &["a.safetensors"]));
    assert!(matches!(&fields["gguf"], Value::StrList { items } if items == &["nested/b.gguf"]));
    std::fs::remove_dir_all(root).ok();
}
