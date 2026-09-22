use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mlpl_extension_hftok::{CONTROL_TOKENS, TokenizerFile, validate_value};
use mlpl_extension_sdk::Value;

fn fixture_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("fixtures/tokenizer");
    path
}

fn fixture_source() -> String {
    fs::read_to_string(fixture_dir().join("tiny-tokenizer.json")).unwrap()
}

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn load_request(root: &Path, path: &str) -> Value {
    record([
        ("root", Value::String(root.to_str().unwrap().to_owned())),
        ("path", Value::String(path.to_owned())),
    ])
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Record(fields) = value else {
        panic!("result must be a record");
    };
    fields
        .get(name)
        .unwrap_or_else(|| panic!("missing field {name}"))
}

/// Rewrites one top-level section of the fixture, so each rejection test
/// differs from a known-good file by exactly the thing under test.
fn fixture_with(mutate: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut document: serde_json::Value = serde_json::from_str(&fixture_source()).unwrap();
    mutate(&mut document);
    document.to_string()
}

#[test]
fn published_fixture_is_a_valid_byte_level_bpe_tokenizer() {
    let parsed = TokenizerFile::parse(&fixture_source()).unwrap();

    assert_eq!(parsed.vocabulary_size(), 274);
    assert_eq!(parsed.merges.len(), 13);
    assert_eq!(parsed.added_tokens.len(), CONTROL_TOKENS.len());
    // The first 256 ids are the byte-level alphabet, so every byte encodes.
    assert_eq!(parsed.token_to_id("t"), Some(116));
    // The merge list is genuinely applied, not decorative.
    assert_eq!(parsed.merges[0], ("h".to_owned(), "e".to_owned()));
    assert!(parsed.token_to_id("he").is_some());
    assert!(parsed.token_to_id("\u{0120}world").is_some());
    assert!(parsed.pattern.contains("\\p{L}"), "{}", parsed.pattern);
}

#[test]
fn fixture_defines_every_control_token_the_work_order_names() {
    let parsed = TokenizerFile::parse(&fixture_source()).unwrap();

    let ids = parsed.control_ids();
    assert!(
        ids.iter().all(|id| *id >= 0),
        "every control token must have an id, got {ids:?}"
    );
    // Control ids follow the learned vocabulary, as in real files.
    assert!(ids.iter().all(|id| *id >= 256), "{ids:?}");
    for name in CONTROL_TOKENS {
        assert_eq!(
            parsed.token_to_id(name),
            Some(
                parsed
                    .added_tokens
                    .iter()
                    .find(|(content, _)| content == name)
                    .unwrap()
                    .1
            ),
            "{name} must agree between vocab and added_tokens"
        );
    }
}

#[test]
fn absent_control_tokens_report_minus_one_rather_than_changing_shape() {
    let source = fixture_with(|document| {
        let vocab = document["model"]["vocab"].as_object_mut().unwrap();
        vocab.remove("<think>");
        vocab.remove("</think>");
        document["added_tokens"]
            .as_array_mut()
            .unwrap()
            .retain(|token| token["content"] != "<think>" && token["content"] != "</think>");
    });

    let ids = TokenizerFile::parse(&source).unwrap().control_ids();

    assert_eq!(ids.len(), CONTROL_TOKENS.len());
    assert_eq!(ids[3], -1);
    assert_eq!(ids[4], -1);
    assert!(ids[0] >= 0);
}

#[test]
fn merges_parse_in_both_the_pair_and_string_forms() {
    let pairs = TokenizerFile::parse(&fixture_source()).unwrap().merges;
    let as_strings = fixture_with(|document| {
        let merges = document["model"]["merges"].as_array().unwrap().clone();
        let rewritten = merges
            .iter()
            .map(|pair| {
                let halves = pair.as_array().unwrap();
                serde_json::Value::String(format!(
                    "{} {}",
                    halves[0].as_str().unwrap(),
                    halves[1].as_str().unwrap()
                ))
            })
            .collect::<Vec<_>>();
        document["model"]["merges"] = serde_json::Value::Array(rewritten);
    });

    let strings = TokenizerFile::parse(&as_strings).unwrap().merges;

    assert_eq!(pairs, strings);
}

#[test]
fn a_bare_byte_level_pre_tokenizer_is_accepted_with_an_empty_pattern() {
    let source = fixture_with(|document| {
        document["pre_tokenizer"] = serde_json::json!({
            "type": "ByteLevel", "add_prefix_space": false,
            "trim_offsets": true, "use_regex": true
        });
    });

    let parsed = TokenizerFile::parse(&source).unwrap();

    assert!(parsed.pattern.is_empty());
}

#[test]
fn unsupported_or_malformed_files_are_errors_not_panics() {
    let cases: Vec<(String, &str)> = vec![
        ("{".to_owned(), "malformed JSON"),
        ("{}".to_owned(), "no model section"),
        (
            fixture_with(|document| document["model"]["type"] = serde_json::json!("WordPiece")),
            "unsupported model type WordPiece",
        ),
        (
            fixture_with(|document| {
                document["model"].as_object_mut().unwrap().remove("type");
            }),
            "model section has no type",
        ),
        (
            fixture_with(|document| document["decoder"] = serde_json::json!({"type": "WordPiece"})),
            "unsupported decoder WordPiece",
        ),
        (
            fixture_with(|document| {
                document.as_object_mut().unwrap().remove("decoder");
            }),
            "no ByteLevel decoder",
        ),
        (
            fixture_with(|document| {
                document.as_object_mut().unwrap().remove("pre_tokenizer");
            }),
            "no pre_tokenizer",
        ),
        (
            fixture_with(|document| {
                document["pre_tokenizer"] = serde_json::json!({"type": "Whitespace"});
            }),
            "unsupported pre-tokenizer Whitespace",
        ),
        (
            fixture_with(|document| {
                document["pre_tokenizer"] = serde_json::json!({
                    "type": "Sequence",
                    "pretokenizers": [{"type": "Split", "pattern": {"Regex": "x"}}]
                });
            }),
            "no ByteLevel member",
        ),
        (
            fixture_with(|document| {
                document["model"].as_object_mut().unwrap().remove("vocab");
            }),
            "no vocab object",
        ),
        (
            fixture_with(|document| document["model"]["vocab"] = serde_json::json!({})),
            "vocab is empty",
        ),
        (
            fixture_with(|document| document["model"]["merges"] = serde_json::json!([])),
            "merges is empty",
        ),
        (
            fixture_with(|document| document["model"]["merges"] = serde_json::json!([["a"]])),
            "merge 0 is not a pair",
        ),
        (
            fixture_with(|document| document["model"]["merges"] = serde_json::json!(["ab"])),
            "merge 0 is not two space-separated halves",
        ),
        (
            fixture_with(|document| document["normalizer"] = serde_json::json!({"type": "NFKC"})),
            "unsupported normalizer NFKC",
        ),
        (
            fixture_with(|document| document["model"]["vocab"]["t"] = serde_json::json!("x")),
            "non-integer id",
        ),
    ];
    for (source, expected) in cases {
        let Err(error) = TokenizerFile::parse(&source) else {
            panic!("expected {expected:?} to be rejected");
        };
        assert!(
            error.contains(expected),
            "expected {expected:?} in {error:?}"
        );
    }
}

#[test]
fn validate_reports_the_fixture_summary_through_the_extension_boundary() {
    let result = validate_value(&[load_request(&fixture_dir(), "tiny-tokenizer.json")]).unwrap();

    assert_eq!(field(&result, "vocabulary_size"), &Value::I64(274));
    assert_eq!(field(&result, "merge_count"), &Value::I64(13));
    assert_eq!(field(&result, "added_token_count"), &Value::I64(5));
    for name in [
        "end_of_text_id",
        "turn_start_id",
        "turn_end_id",
        "think_start_id",
        "think_end_id",
    ] {
        let Value::I64(id) = field(&result, name) else {
            panic!("{name} must be an integer");
        };
        assert!(*id >= 256, "{name} was {id}");
    }
    let Value::String(pattern) = field(&result, "pattern") else {
        panic!("pattern must be a string");
    };
    assert!(pattern.contains("\\p{L}"), "{pattern}");
}

#[test]
fn validate_fails_closed_on_unsafe_or_missing_inputs() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("not-json.json"), "not json at all").unwrap();

    let cases: Vec<(Value, &str)> = vec![
        (
            load_request(root.path(), "../escape.json"),
            "confined relative path",
        ),
        (
            load_request(root.path(), "/etc/passwd"),
            "confined relative path",
        ),
        (load_request(root.path(), ""), "confined relative path"),
        (
            load_request(root.path(), "missing.json"),
            "existing file beneath the root",
        ),
        (
            load_request(Path::new("relative/root"), "tiny-tokenizer.json"),
            "root must be an absolute",
        ),
        (load_request(root.path(), "not-json.json"), "malformed JSON"),
        (Value::Nil, "must be a record"),
        (
            record([("root", Value::String("/tmp".into()))]),
            "fields do not match",
        ),
    ];
    for (value, expected) in cases {
        let error = validate_value(&[value]).unwrap_err();
        assert!(
            error.message().contains(expected),
            "expected {expected:?} in {:?}",
            error.message()
        );
    }
}

#[test]
fn expected_encodings_fixture_is_consistent_with_the_vocabulary() {
    let parsed = TokenizerFile::parse(&fixture_source()).unwrap();
    let expectations: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture_dir().join("tiny-expected-encodings.json")).unwrap(),
    )
    .unwrap();

    let cases = expectations["cases"].as_array().unwrap();
    assert!(cases.len() >= 10, "expected a useful number of cases");
    let highest = parsed.vocabulary_size();
    let mut saw_control = false;
    for case in cases {
        assert!(case["text"].is_string(), "each case needs a text");
        for id in case["ids"].as_array().unwrap() {
            let id = id.as_i64().unwrap();
            assert!(
                (0..highest).contains(&id),
                "id {id} is outside the vocabulary"
            );
            if id >= 256 + i64::try_from(parsed.merges.len()).unwrap() {
                saw_control = true;
            }
        }
    }
    assert!(
        saw_control,
        "the expected encodings must exercise control tokens"
    );
    // The empty string must encode to nothing.
    let empty = cases.iter().find(|case| case["text"] == "").unwrap();
    assert!(empty["ids"].as_array().unwrap().is_empty());
}
