use std::fs;
use std::path::PathBuf;

use mlpl_extension_hftok::{ByteLevelTokenizer, TokenizerFile};

fn fixture_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("fixtures/tokenizer");
    path
}

fn tokenizer() -> ByteLevelTokenizer {
    let source = fs::read_to_string(fixture_dir().join("tiny-tokenizer.json")).unwrap();
    ByteLevelTokenizer::new(&TokenizerFile::parse(&source).unwrap()).unwrap()
}

fn expected_cases() -> Vec<(String, Vec<i64>)> {
    let document: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture_dir().join("tiny-expected-encodings.json")).unwrap(),
    )
    .unwrap();
    document["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            (
                case["text"].as_str().unwrap().to_owned(),
                case["ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|id| id.as_i64().unwrap())
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn every_published_expectation_encodes_exactly() {
    let tokenizer = tokenizer();

    for (text, expected) in expected_cases() {
        assert_eq!(
            tokenizer.encode(&text).unwrap(),
            expected,
            "encoding {text:?}"
        );
    }
}

#[test]
fn every_published_expectation_decodes_back_to_its_text() {
    let tokenizer = tokenizer();

    for (text, ids) in expected_cases() {
        assert_eq!(tokenizer.decode(&ids).unwrap(), text, "decoding {text:?}");
    }
}

#[test]
fn merges_are_applied_in_rank_order_not_left_to_right() {
    let tokenizer = tokenizer();

    // "the" must become "t" + "he", because (h,e) outranks nothing else here.
    // A naive left-to-right pass would try (t,h) first and produce three ids.
    let ids = tokenizer.encode("the").unwrap();
    assert_eq!(ids.len(), 2, "{ids:?}");
    assert_eq!(tokenizer.decode(&ids).unwrap(), "the");

    // " the world" exercises the longer chains: Ġ+t -> Ġt+he -> Ġthe.
    let ids = tokenizer.encode(" the world").unwrap();
    assert_eq!(ids.len(), 2, "{ids:?}");
}

#[test]
fn control_tokens_map_to_their_ids_and_never_merge_with_neighbours() {
    let tokenizer = tokenizer();
    let start = tokenizer.token_to_id("<|im_start|>").unwrap();
    let end = tokenizer.token_to_id("<|im_end|>").unwrap();
    let think_open = tokenizer.token_to_id("<think>").unwrap();
    let think_close = tokenizer.token_to_id("</think>").unwrap();

    let ids = tokenizer.encode("<|im_start|>the<|im_end|>").unwrap();
    assert_eq!(ids.first(), Some(&start));
    assert_eq!(ids.last(), Some(&end));
    assert_eq!(ids.len(), 4, "the middle text stays two tokens: {ids:?}");

    let ids = tokenizer.encode("<think>the</think>").unwrap();
    assert_eq!(ids.first(), Some(&think_open));
    assert_eq!(ids.last(), Some(&think_close));

    // Adjacent control tokens stay separate ids.
    let ids = tokenizer.encode("<|im_start|><|im_end|>").unwrap();
    assert_eq!(ids, vec![start, end]);
}

#[test]
fn decode_keeps_control_tokens_visible() {
    let tokenizer = tokenizer();

    let text = "<|im_start|>the<|im_end|>";
    let ids = tokenizer.encode(text).unwrap();

    assert_eq!(tokenizer.decode(&ids).unwrap(), text);
    assert!(tokenizer.decode(&ids).unwrap().contains("<|im_start|>"));
}

#[test]
fn arbitrary_byte_sequences_round_trip() {
    let tokenizer = tokenizer();
    // A deterministic pseudo-random sweep, plus text that stresses the
    // pre-tokenization pattern: punctuation runs, digits, newlines, and
    // trailing whitespace.
    let mut samples = vec![
        String::new(),
        " ".to_owned(),
        "   ".to_owned(),
        "\n".to_owned(),
        "a\nb".to_owned(),
        "!!!???".to_owned(),
        "12345".to_owned(),
        "the the the".to_owned(),
        "  leading and trailing  ".to_owned(),
        "tab\there".to_owned(),
    ];
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    for _ in 0..200 {
        let length = (state % 24) as usize;
        let mut sample = String::new();
        for _ in 0..length {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            // Printable ASCII, so the sample is ordinary text.
            let byte = u8::try_from(0x20 + (state >> 33) % 0x5F).unwrap();
            sample.push(char::from(byte));
        }
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        samples.push(sample);
    }

    for sample in samples {
        let ids = tokenizer
            .encode(&sample)
            .unwrap_or_else(|error| panic!("encoding {sample:?} failed: {error}"));
        assert_eq!(
            tokenizer.decode(&ids).unwrap(),
            sample,
            "round trip of {sample:?}"
        );
    }
}

#[test]
fn multi_byte_text_round_trips_through_the_byte_alphabet() {
    let tokenizer = tokenizer();

    // The fixture's vocabulary has all 256 byte values, so non-ASCII text
    // encodes as its UTF-8 bytes even though no merge covers it.
    for sample in ["é", "日本語", "🙂", "a é b"] {
        let ids = tokenizer.encode(sample).unwrap();
        assert!(!ids.is_empty());
        assert_eq!(tokenizer.decode(&ids).unwrap(), sample, "{sample}");
    }
}

#[test]
fn empty_input_encodes_to_nothing_and_decodes_to_nothing() {
    let tokenizer = tokenizer();

    assert!(tokenizer.encode("").unwrap().is_empty());
    assert_eq!(tokenizer.decode(&[]).unwrap(), "");
}

#[test]
fn unknown_ids_and_unknown_tokens_are_errors_not_panics() {
    let tokenizer = tokenizer();

    let error = tokenizer.decode(&[99_999]).unwrap_err();
    assert!(error.contains("not in the vocabulary"), "{error}");

    let error = tokenizer.decode(&[-1]).unwrap_err();
    assert!(error.contains("not in the vocabulary"), "{error}");

    assert_eq!(tokenizer.token_to_id("definitely-not-a-token"), None);
}

#[test]
fn decoding_a_partial_multi_byte_sequence_reports_invalid_utf8() {
    let tokenizer = tokenizer();

    // The first byte of a two-byte sequence on its own is not valid UTF-8.
    let ids = tokenizer.encode("é").unwrap();
    assert_eq!(ids.len(), 2, "two bytes, so two ids: {ids:?}");

    let error = tokenizer.decode(&ids[..1]).unwrap_err();
    assert!(error.contains("not UTF-8"), "{error}");
}

#[test]
fn a_pattern_that_does_not_compile_is_reported_rather_than_panicking() {
    let source = fs::read_to_string(fixture_dir().join("tiny-tokenizer.json")).unwrap();
    let mut document: serde_json::Value = serde_json::from_str(&source).unwrap();
    document["pre_tokenizer"]["pretokenizers"][0]["pattern"]["Regex"] =
        serde_json::json!("(unclosed");
    let file = TokenizerFile::parse(&document.to_string()).unwrap();

    let Err(error) = ByteLevelTokenizer::new(&file) else {
        panic!("an uncompilable pattern must be rejected");
    };

    assert!(error.contains("does not compile"), "{error}");
}

#[test]
fn a_pattern_that_does_not_cover_the_input_is_refused_not_truncated() {
    // A pattern matching only letters leaves digits and punctuation
    // unmatched. Encoding a prefix of the caller's text would hand back
    // plausible but wrong ids, so this must be an error.
    let source = fs::read_to_string(fixture_dir().join("tiny-tokenizer.json")).unwrap();
    let mut document: serde_json::Value = serde_json::from_str(&source).unwrap();
    document["pre_tokenizer"]["pretokenizers"][0]["pattern"]["Regex"] = serde_json::json!("[a-z]+");
    let file = TokenizerFile::parse(&document.to_string()).unwrap();
    let tokenizer = ByteLevelTokenizer::new(&file).unwrap();

    // Pure letters still encode.
    assert!(!tokenizer.encode("the").unwrap().is_empty());

    let Err(error) = tokenizer.encode("the 9") else {
        panic!("uncovered input must be refused");
    };
    assert!(error.contains("pre-tokenization"), "{error}");
}

#[test]
fn the_files_own_pattern_is_used_and_reported() {
    let tokenizer = tokenizer();

    assert!(tokenizer.pattern_source.contains("\\p{L}"));
    assert_eq!(tokenizer.vocabulary_size(), 274);
    assert!(tokenizer.control_ids().iter().all(|id| *id >= 256));
}
