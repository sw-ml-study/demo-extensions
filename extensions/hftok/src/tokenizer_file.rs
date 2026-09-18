//! Parsing and validation of a Hugging Face `tokenizer.json`.
//!
//! Pure: this module reads a string and decides whether the file describes a
//! byte-level BPE tokenizer this extension can encode correctly. It performs
//! no encoding and holds no application semantics.

use std::collections::BTreeMap;

use serde_json::Value as Json;

/// A validated byte-level BPE tokenizer definition.
pub struct TokenizerFile {
    /// Token text to id.
    pub vocab: BTreeMap<String, i64>,
    /// Merge pairs in rank order; earlier entries bind first.
    pub merges: Vec<(String, String)>,
    /// Added tokens matched literally before pre-tokenization.
    pub added_tokens: Vec<(String, i64)>,
    /// The pre-tokenization pattern the file itself specifies.
    pub pattern: String,
}

/// Control tokens the work order names, reported by `info` in this order.
pub const CONTROL_TOKENS: [&str; 5] = [
    "<|endoftext|>",
    "<|im_start|>",
    "<|im_end|>",
    "<think>",
    "</think>",
];

impl TokenizerFile {
    /// Parses and validates one `tokenizer.json` document.
    ///
    /// # Errors
    ///
    /// Returns a human-readable reason when the JSON is malformed, the model
    /// is not BPE, the pre-tokenizer or decoder is not byte-level, or a
    /// required section is missing or malformed.
    pub fn parse(source: &str) -> Result<Self, String> {
        let document: Json =
            serde_json::from_str(source).map_err(|error| format!("malformed JSON: {error}"))?;
        let model = document
            .get("model")
            .ok_or_else(|| "tokenizer.json has no model section".to_owned())?;
        match model.get("type").and_then(Json::as_str) {
            Some("BPE") => {}
            Some(other) => {
                return Err(format!(
                    "unsupported model type {other}; only BPE is supported"
                ));
            }
            None => return Err("model section has no type".to_owned()),
        }
        require_byte_level_decoder(&document)?;
        let pattern = pre_tokenizer_pattern(&document)?;
        let vocab = parse_vocab(model)?;
        let merges = parse_merges(model)?;
        let added_tokens = parse_added_tokens(&document)?;
        if let Some(normalizer) = document.get("normalizer") {
            if !normalizer.is_null() {
                let kind = normalizer
                    .get("type")
                    .and_then(Json::as_str)
                    .unwrap_or("unknown");
                return Err(format!(
                    "unsupported normalizer {kind}; a normalizer would rewrite text before encoding"
                ));
            }
        }
        Ok(Self {
            vocab,
            merges,
            added_tokens,
            pattern,
        })
    }

    /// Looks up one token's id.
    #[must_use]
    pub fn token_to_id(&self, token: &str) -> Option<i64> {
        self.vocab.get(token).copied()
    }

    /// Reports the id of each control token, or `-1` when the file does not
    /// define it, so the reported shape is fixed.
    #[must_use]
    pub fn control_ids(&self) -> [i64; 5] {
        let mut ids = [-1; 5];
        for (slot, name) in ids.iter_mut().zip(CONTROL_TOKENS) {
            if let Some(id) = self.token_to_id(name) {
                *slot = id;
            }
        }
        ids
    }

    /// Vocabulary size, added tokens included.
    #[must_use]
    pub fn vocabulary_size(&self) -> i64 {
        i64::try_from(self.vocab.len()).unwrap_or(i64::MAX)
    }
}

fn require_byte_level_decoder(document: &Json) -> Result<(), String> {
    match document
        .get("decoder")
        .and_then(|decoder| decoder.get("type"))
    {
        Some(Json::String(kind)) if kind == "ByteLevel" => Ok(()),
        Some(Json::String(kind)) => Err(format!(
            "unsupported decoder {kind}; only ByteLevel is supported"
        )),
        _ => Err("tokenizer.json has no ByteLevel decoder".to_owned()),
    }
}

/// Extracts the pre-tokenization pattern.
///
/// Real vocabularies wrap a `Split` carrying the pattern and a `ByteLevel` in a
/// `Sequence`; a bare `ByteLevel` pre-tokenizer is also accepted and uses the
/// byte-level default of no split pattern.
fn pre_tokenizer_pattern(document: &Json) -> Result<String, String> {
    let pre_tokenizer = document
        .get("pre_tokenizer")
        .ok_or_else(|| "tokenizer.json has no pre_tokenizer".to_owned())?;
    match pre_tokenizer.get("type").and_then(Json::as_str) {
        Some("ByteLevel") => Ok(String::new()),
        Some("Sequence") => {
            let members = pre_tokenizer
                .get("pretokenizers")
                .and_then(Json::as_array)
                .ok_or_else(|| "pre_tokenizer Sequence has no pretokenizers".to_owned())?;
            let mut pattern = None;
            let mut byte_level = false;
            for member in members {
                match member.get("type").and_then(Json::as_str) {
                    Some("Split") => {
                        let found = member
                            .get("pattern")
                            .and_then(|value| value.get("Regex"))
                            .and_then(Json::as_str)
                            .ok_or_else(|| "pre_tokenizer Split has no Regex pattern".to_owned())?;
                        pattern = Some(found.to_owned());
                    }
                    Some("ByteLevel") => byte_level = true,
                    Some(other) => {
                        return Err(format!("unsupported pre-tokenizer member {other}"));
                    }
                    None => return Err("pre-tokenizer member has no type".to_owned()),
                }
            }
            if !byte_level {
                return Err(
                    "pre_tokenizer Sequence has no ByteLevel member; only byte-level BPE is supported"
                        .to_owned(),
                );
            }
            pattern.ok_or_else(|| "pre_tokenizer Sequence has no Split pattern".to_owned())
        }
        Some(other) => Err(format!("unsupported pre-tokenizer {other}")),
        None => Err("pre_tokenizer has no type".to_owned()),
    }
}

fn parse_vocab(model: &Json) -> Result<BTreeMap<String, i64>, String> {
    let entries = model
        .get("vocab")
        .and_then(Json::as_object)
        .ok_or_else(|| "model has no vocab object".to_owned())?;
    if entries.is_empty() {
        return Err("model vocab is empty".to_owned());
    }
    let mut vocab = BTreeMap::new();
    for (token, id) in entries {
        let id = id
            .as_i64()
            .ok_or_else(|| format!("vocab entry {token} has a non-integer id"))?;
        vocab.insert(token.clone(), id);
    }
    Ok(vocab)
}

/// Parses the merge list, accepting both the current two-element array form
/// and the older single-string form with one space between halves.
fn parse_merges(model: &Json) -> Result<Vec<(String, String)>, String> {
    let entries = model
        .get("merges")
        .and_then(Json::as_array)
        .ok_or_else(|| "model has no merges array".to_owned())?;
    let mut merges = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        match entry {
            Json::Array(pair) => {
                let [left, right] = pair.as_slice() else {
                    return Err(format!("merge {index} is not a pair"));
                };
                let (Some(left), Some(right)) = (left.as_str(), right.as_str()) else {
                    return Err(format!("merge {index} has a non-string half"));
                };
                merges.push((left.to_owned(), right.to_owned()));
            }
            Json::String(text) => {
                let mut halves = text.splitn(2, ' ');
                let (Some(left), Some(right)) = (halves.next(), halves.next()) else {
                    return Err(format!("merge {index} is not two space-separated halves"));
                };
                if left.is_empty() || right.is_empty() {
                    return Err(format!("merge {index} has an empty half"));
                }
                merges.push((left.to_owned(), right.to_owned()));
            }
            _ => return Err(format!("merge {index} is neither a pair nor a string")),
        }
    }
    if merges.is_empty() {
        return Err("model merges is empty".to_owned());
    }
    Ok(merges)
}

fn parse_added_tokens(document: &Json) -> Result<Vec<(String, i64)>, String> {
    let Some(entries) = document.get("added_tokens") else {
        return Ok(Vec::new());
    };
    if entries.is_null() {
        return Ok(Vec::new());
    }
    let entries = entries
        .as_array()
        .ok_or_else(|| "added_tokens is not an array".to_owned())?;
    let mut added = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let content = entry
            .get("content")
            .and_then(Json::as_str)
            .ok_or_else(|| format!("added token {index} has no content"))?;
        let id = entry
            .get("id")
            .and_then(Json::as_i64)
            .ok_or_else(|| format!("added token {index} has no integer id"))?;
        added.push((content.to_owned(), id));
    }
    Ok(added)
}
