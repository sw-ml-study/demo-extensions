//! Parity against the artifacts published by `../reasoning-from-scratch`.
//!
//! Every test here is skipped, loudly, when the upstream artifact is absent,
//! so this suite never invents a fixture and never silently passes.

use std::fs;
use std::path::{Path, PathBuf};

use mlpl_extension_hftok::{ByteLevelTokenizer, TokenizerFile};

fn upstream() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.pop();
    path.push("reasoning-from-scratch");
    path
}

fn tokenizer_at(path: &Path) -> Option<ByteLevelTokenizer> {
    let source = fs::read_to_string(path).ok()?;
    let file = TokenizerFile::parse(&source)
        .unwrap_or_else(|error| panic!("{} must parse: {error}", path.display()));
    Some(
        ByteLevelTokenizer::new(&file)
            .unwrap_or_else(|error| panic!("{} must compile: {error}", path.display())),
    )
}

#[test]
fn the_upstream_tiny_fixture_parses_and_encodes() {
    let path = upstream().join("fixtures/tokenizer/tiny-tokenizer.json");
    let Some(tokenizer) = tokenizer_at(&path) else {
        println!("SKIP: {} is absent", path.display());
        return;
    };

    // Their fixture differs from ours: string-form merges, a bare ByteLevel
    // pre-tokenizer, and two control tokens rather than five. It must parse
    // unmodified.
    assert!(tokenizer.vocabulary_size() > 0);
    let ids = tokenizer.encode("ab").expect("encoding must succeed");
    assert_eq!(tokenizer.decode(&ids).unwrap(), "ab");
}

#[test]
fn every_upstream_tiny_expectation_matches() {
    let fixture = upstream().join("fixtures/tokenizer/tiny-tokenizer.json");
    let expectations = upstream().join("fixtures/tokenizer/tiny-expected.jsonl");
    let (Some(tokenizer), Ok(lines)) = (tokenizer_at(&fixture), fs::read_to_string(&expectations))
    else {
        println!("SKIP: {} or its fixture is absent", expectations.display());
        return;
    };

    let mut checked = 0;
    for line in lines.lines().filter(|line| !line.trim().is_empty()) {
        let case: serde_json::Value = serde_json::from_str(line).unwrap();
        let Some(text) = case["text"].as_str() else {
            continue;
        };
        let expected = case["ids"]
            .as_array()
            .unwrap_or_else(|| panic!("case {case} needs ids"))
            .iter()
            .map(|id| id.as_i64().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            tokenizer.encode(text).unwrap(),
            expected,
            "upstream expectation for {text:?}"
        );
        checked += 1;
    }
    assert!(checked > 0, "the expectation file must contain cases");
    println!("checked {checked} upstream tiny expectations");
}

#[test]
fn every_qwen3_golden_encodes_and_round_trips() {
    let vocabulary = upstream().join("models/qwen3-0.6b-base/tokenizer.json");
    let goldens = upstream().join("fixtures/tokenizer/qwen3-goldens.jsonl");
    let (Some(tokenizer), Ok(lines)) = (tokenizer_at(&vocabulary), fs::read_to_string(&goldens))
    else {
        println!(
            "SKIP: {} or {} is absent",
            vocabulary.display(),
            goldens.display()
        );
        return;
    };

    let mut checked = 0;
    for line in lines.lines().filter(|line| !line.trim().is_empty()) {
        let case: serde_json::Value = serde_json::from_str(line).unwrap();
        let text = case["text"].as_str().unwrap();
        let expected = case["ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_i64().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            tokenizer.encode(text).unwrap(),
            expected,
            "golden encoding for {text:?}"
        );
        assert_eq!(
            tokenizer.decode(&expected).unwrap(),
            case["round_trip"].as_str().unwrap(),
            "golden round trip for {text:?}"
        );
        checked += 1;
    }
    assert!(checked > 0, "the golden file must contain cases");
    println!("checked {checked} Qwen3 goldens");
}

#[test]
fn the_published_merge_rank_invariant_holds_on_the_real_vocabulary() {
    let path = upstream().join("models/qwen3-0.6b-base/tokenizer.json");
    let Ok(source) = fs::read_to_string(&path) else {
        println!("SKIP: {} is absent", path.display());
        return;
    };
    let file = TokenizerFile::parse(&source).unwrap();

    // The work order states, verified across all merges, that a merged
    // token's vocabulary id is its merge rank plus 256. Checked here rather
    // than relied on: this encoder uses a rank table instead.
    let mut mismatches = Vec::new();
    for (rank, (left, right)) in file.merges.iter().enumerate() {
        let merged = format!("{left}{right}");
        let Some(id) = file.token_to_id(&merged) else {
            mismatches.push(format!("rank {rank}: {merged} absent"));
            continue;
        };
        let expected = i64::try_from(rank).unwrap() + 256;
        if id != expected {
            mismatches.push(format!("rank {rank}: {merged} is {id}, not {expected}"));
        }
    }
    println!(
        "merge-rank invariant: {} merges, {} mismatches",
        file.merges.len(),
        mismatches.len()
    );
    for line in mismatches.iter().take(5) {
        println!("  {line}");
    }
    assert!(
        mismatches.is_empty(),
        "{} of {} merges break the published invariant",
        mismatches.len(),
        file.merges.len()
    );
}
