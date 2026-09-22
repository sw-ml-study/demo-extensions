use mlpl_extension_hftok::{ByteLevelTokenizer, TokenizerFile};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/tokenizer/tiny-tokenizer.json"
    ))
    .unwrap()
}

fn compile(document: &Value) -> ByteLevelTokenizer {
    ByteLevelTokenizer::new(&TokenizerFile::parse(&document.to_string()).unwrap()).unwrap()
}

#[test]
fn nfc_composes_before_byte_level_encoding_and_decode_returns_normalized_text() {
    let mut document = fixture();
    document["normalizer"] = json!({"type": "NFC"});
    let normalized = compile(&document);
    let plain = compile(&fixture());
    for (input, expected) in [
        ("Cafe\u{301}", "Café"),
        ("\u{1100}\u{1161}", "가"),
        ("a\u{315}\u{300}", "à\u{315}"),
        ("①", "①"),
        ("", ""),
    ] {
        let ids = normalized.encode(input).unwrap();
        assert_eq!(ids, plain.encode(expected).unwrap());
        assert_eq!(normalized.decode(&ids).unwrap(), expected);
        assert_eq!(plain.decode(&plain.encode(input).unwrap()).unwrap(), input);
    }
}

#[test]
fn unnormalized_added_tokens_are_isolated_before_nfc() {
    let mut document = fixture();
    document["normalizer"] = json!({"type": "NFC"});
    document["model"]["vocab"]["e\u{301}"] = json!(999);
    document["added_tokens"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": 999, "content": "e\u{301}", "normalized": false
        }));
    let tokenizer = compile(&document);
    let ids = tokenizer
        .encode("e\u{301}<|im_start|>a\u{300}<|im_end|>")
        .unwrap();
    assert_eq!(ids[0], 999);
    assert_eq!(
        tokenizer.decode(&ids).unwrap(),
        "e\u{301}<|im_start|>à<|im_end|>"
    );
}

#[test]
fn nfc_refuses_normalized_added_tokens_instead_of_silently_matching_them_wrongly() {
    let mut document = fixture();
    document["normalizer"] = json!({"type": "NFC"});
    document["added_tokens"][0]["normalized"] = json!(true);
    let error = TokenizerFile::parse(&document.to_string()).err().unwrap();
    assert!(error.contains("normalized added tokens"), "{error}");
}

#[test]
fn added_tokens_outside_the_bpe_vocabulary_are_reported_and_decoded() {
    let mut document = fixture();
    document["normalizer"] = json!({"type": "NFC"});
    let added = document["added_tokens"].as_array().unwrap().clone();
    for token in &added {
        document["model"]["vocab"]
            .as_object_mut()
            .unwrap()
            .remove(token["content"].as_str().unwrap());
    }
    let tokenizer = compile(&document);
    assert_eq!(tokenizer.vocabulary_size(), 274);
    for token in added {
        let content = token["content"].as_str().unwrap();
        let id = token["id"].as_i64().unwrap();
        assert_eq!(tokenizer.token_to_id(content), Some(id));
        assert_eq!(tokenizer.encode(content).unwrap(), [id]);
        assert_eq!(tokenizer.decode(&[id]).unwrap(), content);
    }
}
