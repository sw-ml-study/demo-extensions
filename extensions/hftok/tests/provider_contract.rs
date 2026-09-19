#![allow(unsafe_code)]

use std::path::PathBuf;

use mlpl_extension_loader::{CallError, ProviderKind, Registry, Value};
use mlpl_extension_sdk::{DenseArray, NativeHandle};

fn library() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target/debug/deps");
    path.push(format!(
        "{}mlpl_extension_hftok{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    path
}

fn fixture_path() -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("fixtures/tokenizer/tiny-tokenizer.json");
    path.canonicalize().unwrap().to_str().unwrap().to_owned()
}

fn registries() -> [Registry; 2] {
    [
        Registry::load(library()).unwrap(),
        // SAFETY: the generated entry returns immutable process-lifetime
        // descriptor storage and callable code.
        unsafe { Registry::load_static(mlpl_extension_hftok::static_entry) }.unwrap(),
    ]
}

fn load(registry: &Registry) -> Value {
    registry
        .call("_hftok.load_path", &[Value::String(fixture_path())])
        .unwrap()
}

#[test]
fn dynamic_and_static_providers_publish_the_same_tokenizer_api() {
    for registry in registries() {
        assert!(matches!(
            registry.provider_kind(),
            ProviderKind::Dynamic | ProviderKind::Static
        ));
        assert_eq!(registry.extension_name(), "_hftok");
        assert_eq!(
            registry.function_names(),
            [
                "_hftok.close",
                "_hftok.decode",
                "_hftok.encode",
                "_hftok.info",
                "_hftok.load",
                "_hftok.load_path",
                "_hftok.token_to_id",
                "_hftok.validate",
            ]
        );
        assert_eq!(
            registry.help("_hftok.encode").unwrap(),
            "_hftok.encode(tokenizer: native<Tokenizer>, text: string) -> array\nEncode text into an array of token ids."
        );
    }
}

#[test]
fn a_loaded_tokenizer_encodes_decodes_and_reports_its_summary() {
    for registry in registries() {
        let handle = load(&registry);
        assert!(matches!(handle, Value::Handle(_)));

        let Ok(Value::Array(ids)) = registry.call(
            "_hftok.encode",
            &[handle.clone(), Value::String(" the world".into())],
        ) else {
            panic!("encode must return an array");
        };
        let view = ids.view();
        assert_eq!(view.shape(), [2], "ids cross as a one-dimensional array");
        assert_eq!(view.as_i64().unwrap().len(), 2);

        let decoded = registry
            .call("_hftok.decode", &[handle.clone(), Value::Array(ids)])
            .unwrap();
        assert_eq!(decoded, Value::String(" the world".into()));

        let Ok(Value::Record(info)) = registry.call("_hftok.info", std::slice::from_ref(&handle))
        else {
            panic!("info must return a record");
        };
        assert_eq!(info.get("vocabulary_size"), Some(&Value::I64(274)));
        for name in [
            "end_of_text_id",
            "turn_start_id",
            "turn_end_id",
            "think_start_id",
            "think_end_id",
        ] {
            let Some(Value::I64(id)) = info.get(name) else {
                panic!("{name} must be an integer");
            };
            assert!(*id >= 256, "{name} was {id}");
        }
        let Some(Value::String(pattern)) = info.get("pattern") else {
            panic!("pattern must be a string");
        };
        assert!(pattern.contains("\\p{L}"));

        let id = registry
            .call(
                "_hftok.token_to_id",
                &[handle.clone(), Value::String("<|endoftext|>".into())],
            )
            .unwrap();
        assert!(matches!(id, Value::I64(value) if value >= 256));

        assert!(matches!(
            registry.call(
                "_hftok.token_to_id",
                &[handle.clone(), Value::String("not-a-token".into())]
            ),
            Err(CallError::Extension(message)) if message.contains("not in the vocabulary")
        ));

        assert_eq!(
            registry.call("_hftok.close", &[handle]).unwrap(),
            Value::Bool(true)
        );
    }
}

#[test]
fn a_closed_handle_is_stale_for_every_call() {
    let registry = &registries()[0];
    let handle = load(registry);
    registry
        .call("_hftok.close", std::slice::from_ref(&handle))
        .unwrap();

    for (name, arguments) in [
        ("_hftok.info", vec![handle.clone()]),
        ("_hftok.close", vec![handle.clone()]),
        (
            "_hftok.encode",
            vec![handle.clone(), Value::String("x".into())],
        ),
        (
            "_hftok.token_to_id",
            vec![handle.clone(), Value::String("t".into())],
        ),
    ] {
        assert!(
            matches!(
                registry.call(name, &arguments),
                Err(CallError::InvalidArgument(message)) if message.contains("stale")
            ),
            "{name} must reject a closed handle as stale"
        );
    }
}

#[test]
fn a_foreign_or_wrong_type_handle_fails_closed() {
    let registry = &registries()[0];
    let handle = load(registry);
    let Value::Handle(real) = handle else {
        panic!("load must return a handle");
    };

    // Same slot and generation, a different extension.
    let foreign = NativeHandle::from_parts(
        real.extension_id().wrapping_add(1),
        real.type_id(),
        real.slot(),
        real.generation(),
    );
    assert!(matches!(
        registry.call("_hftok.info", &[Value::Handle(foreign)]),
        Err(CallError::InvalidArgument(message)) if message.contains("another extension")
    ));

    // This extension, a resource type it does not define.
    let wrong_type = NativeHandle::from_parts(
        real.extension_id(),
        real.type_id().wrapping_add(7),
        real.slot(),
        real.generation(),
    );
    assert!(matches!(
        registry.call("_hftok.info", &[Value::Handle(wrong_type)]),
        Err(CallError::InvalidArgument(message)) if message.contains("wrong resource type")
    ));

    // A generation that has never been issued.
    let stale = NativeHandle::from_parts(
        real.extension_id(),
        real.type_id(),
        real.slot(),
        real.generation().wrapping_add(9),
    );
    assert!(matches!(
        registry.call("_hftok.info", &[Value::Handle(stale)]),
        Err(CallError::InvalidArgument(message)) if message.contains("stale")
    ));
}

#[test]
fn malformed_arguments_are_errors_not_panics() {
    let registry = &registries()[0];
    let handle = load(registry);

    let cases: Vec<(&str, Vec<Value>, &str)> = vec![
        ("_hftok.load_path", vec![Value::Nil], "must be a string"),
        (
            "_hftok.load_path",
            vec![Value::String("relative/path.json".into())],
            "must be absolute",
        ),
        (
            "_hftok.encode",
            vec![Value::Nil, Value::String("x".into())],
            "expects a tokenizer handle",
        ),
        (
            "_hftok.encode",
            vec![handle.clone(), Value::Nil],
            "text must be a string",
        ),
        (
            "_hftok.decode",
            vec![handle.clone(), Value::Nil],
            "must be an integer array",
        ),
        (
            "_hftok.decode",
            vec![
                handle.clone(),
                Value::Array(DenseArray::from_f64(vec![1], vec![1.5]).unwrap()),
            ],
            "not a whole number",
        ),
        (
            "_hftok.info",
            vec![Value::Nil],
            "expects a tokenizer handle",
        ),
    ];
    for (name, arguments, expected) in cases {
        match registry.call(name, &arguments) {
            Err(CallError::InvalidArgument(message)) => assert!(
                message.contains(expected),
                "expected {expected:?} in {message:?}"
            ),
            other => panic!("{name} must reject its arguments, got {other:?}"),
        }
    }

    // MLPL arrays are f64, so whole-numbered f64 ids decode like i64 ones.
    let Ok(Value::Array(ids)) = registry.call(
        "_hftok.encode",
        &[handle.clone(), Value::String(" the world".into())],
    ) else {
        panic!("encode must return an array");
    };
    let as_f64 = ids
        .view()
        .as_i64()
        .unwrap()
        .iter()
        .map(|id| f64::from(i32::try_from(*id).expect("fixture ids are small")))
        .collect::<Vec<_>>();
    assert_eq!(
        registry
            .call(
                "_hftok.decode",
                &[
                    handle.clone(),
                    Value::Array(DenseArray::from_f64(vec![as_f64.len()], as_f64).unwrap())
                ]
            )
            .unwrap(),
        Value::String(" the world".into())
    );

    // An unknown id decodes to an error rather than a panic.
    assert!(matches!(
        registry.call(
            "_hftok.decode",
            &[
                handle.clone(),
                Value::Array(DenseArray::from_i64(vec![1], vec![99_999]).unwrap())
            ]
        ),
        Err(CallError::Extension(message)) if message.contains("not in the vocabulary")
    ));

    registry.call("_hftok.close", &[handle]).unwrap();
}
