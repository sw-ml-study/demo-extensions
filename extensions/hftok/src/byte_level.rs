//! Byte-level BPE encoding and decoding.
//!
//! Pure: this module turns text into ids and ids back into text using only the
//! vocabulary, merge list, added tokens, and pattern read from a
//! `tokenizer.json`. It holds no chat roles, prompts, conversation structure,
//! or model semantics of any kind.

use std::collections::HashMap;

use fancy_regex::Regex;

use crate::tokenizer_file::TokenizerFile;

/// The GPT-2 byte-to-unicode alphabet that every byte-level BPE file uses.
///
/// Each of the 256 byte values maps to one printable code point, so arbitrary
/// bytes survive a round trip through a text vocabulary. Bytes that are
/// already printable ASCII map to themselves; the rest are shifted into an
/// unused range, which is why a leading space appears as `Ġ` in real files.
fn byte_to_unicode() -> [char; 256] {
    let mut mapping = ['\0'; 256];
    let mut next_spare = 0_u32;
    // Iterating over u8 keeps every conversion here infallible, so the
    // alphabet is built without a panic path.
    for byte in 0_u8..=u8::MAX {
        let code = u32::from(byte);
        let printable = (0x21..=0x7E).contains(&code)
            || (0xA1..=0xAC).contains(&code)
            || (0xAE..=0xFF).contains(&code);
        let point = if printable {
            code
        } else {
            let spare = 256 + next_spare;
            next_spare += 1;
            spare
        };
        // Every value used here is a valid scalar; the replacement character
        // is an unreachable fallback that keeps this total.
        mapping[usize::from(byte)] = char::from_u32(point).unwrap_or(char::REPLACEMENT_CHARACTER);
    }
    mapping
}

/// A compiled tokenizer ready to encode and decode.
pub struct ByteLevelTokenizer {
    vocab: HashMap<String, i64>,
    id_to_token: HashMap<i64, String>,
    /// Merge pair to rank; a lower rank binds first.
    ranks: HashMap<(String, String), usize>,
    /// Added tokens, longest first so the longest match wins.
    added: Vec<(String, i64)>,
    pattern: Option<Regex>,
    byte_to_unicode: [char; 256],
    unicode_to_byte: HashMap<char, u8>,
    /// Reported by `info`, and the pattern the file itself specified.
    pub pattern_source: String,
    control_ids: [i64; 5],
    vocabulary_size: i64,
}

impl ByteLevelTokenizer {
    /// Compiles a validated file into an encoder.
    ///
    /// # Errors
    ///
    /// Returns a reason when the file's pre-tokenization pattern will not
    /// compile.
    pub fn new(file: &TokenizerFile) -> Result<Self, String> {
        let pattern =
            if file.pattern.is_empty() {
                None
            } else {
                Some(Regex::new(&file.pattern).map_err(|error| {
                    format!("pre-tokenization pattern does not compile: {error}")
                })?)
            };
        let mapping = byte_to_unicode();
        let unicode_to_byte = (0_u8..=u8::MAX)
            .map(|byte| (mapping[usize::from(byte)], byte))
            .collect();
        let ranks = file
            .merges
            .iter()
            .enumerate()
            .map(|(rank, pair)| (pair.clone(), rank))
            .collect();
        let mut added = file.added_tokens.clone();
        // Longest first, so `<|im_start|>` wins over any shorter prefix.
        added.sort_by(|left, right| right.0.len().cmp(&left.0.len()).then(left.0.cmp(&right.0)));
        let id_to_token = file
            .vocab
            .iter()
            .map(|(token, id)| (*id, token.clone()))
            .collect();
        Ok(Self {
            vocab: file.vocab.iter().map(|(t, i)| (t.clone(), *i)).collect(),
            id_to_token,
            ranks,
            added,
            pattern,
            byte_to_unicode: mapping,
            unicode_to_byte,
            pattern_source: file.pattern.clone(),
            control_ids: file.control_ids(),
            vocabulary_size: file.vocabulary_size(),
        })
    }

    /// Encodes text into token ids.
    ///
    /// Added tokens, which include the control tokens, are matched literally
    /// and isolated before pre-tokenization, so they never merge with
    /// surrounding text.
    ///
    /// # Errors
    ///
    /// Returns a reason when a byte-level fragment is absent from the
    /// vocabulary, which means the file's alphabet is incomplete.
    pub fn encode(&self, text: &str) -> Result<Vec<i64>, String> {
        let mut ids = Vec::new();
        for segment in self.split_on_added_tokens(text) {
            match segment {
                Segment::Added(id) => ids.push(id),
                Segment::Text(text) => self.encode_text(text, &mut ids)?,
            }
        }
        Ok(ids)
    }

    /// Decodes token ids back into text, leaving control tokens visible as
    /// their literal spelling rather than dropping them.
    ///
    /// # Errors
    ///
    /// Returns a reason when an id is absent from the vocabulary or the
    /// decoded bytes are not valid UTF-8.
    pub fn decode(&self, ids: &[i64]) -> Result<String, String> {
        let mut bytes = Vec::new();
        for id in ids {
            let token = self
                .id_to_token
                .get(id)
                .ok_or_else(|| format!("id {id} is not in the vocabulary"))?;
            if self
                .added
                .iter()
                .any(|(content, added)| added == id && content == token)
            {
                // An added token is literal text, not byte-level glyphs.
                bytes.extend_from_slice(token.as_bytes());
                continue;
            }
            for glyph in token.chars() {
                let byte = self.unicode_to_byte.get(&glyph).ok_or_else(|| {
                    format!("token {token} contains a glyph outside the byte-level alphabet")
                })?;
                bytes.push(*byte);
            }
        }
        String::from_utf8(bytes).map_err(|error| format!("decoded bytes are not UTF-8: {error}"))
    }

    /// Looks up one token's id.
    #[must_use]
    pub fn token_to_id(&self, token: &str) -> Option<i64> {
        self.vocab.get(token).copied()
    }

    /// Control-token ids in `CONTROL_TOKENS` order.
    #[must_use]
    pub fn control_ids(&self) -> [i64; 5] {
        self.control_ids
    }

    /// Vocabulary size, added tokens included.
    #[must_use]
    pub fn vocabulary_size(&self) -> i64 {
        self.vocabulary_size
    }

    fn encode_text(&self, text: &str, ids: &mut Vec<i64>) -> Result<(), String> {
        for fragment in self.pre_tokenize(text)? {
            let mapped = fragment
                .bytes()
                .map(|byte| self.byte_to_unicode[usize::from(byte)])
                .collect::<String>();
            for part in self.apply_merges(&mapped) {
                let id = self
                    .vocab
                    .get(&part)
                    .ok_or_else(|| format!("token {part} is not in the vocabulary"))?;
                ids.push(*id);
            }
        }
        Ok(())
    }

    /// Splits text into pre-tokens using the file's own pattern. Without a
    /// pattern the whole segment is one pre-token, which is the byte-level
    /// default.
    ///
    /// # Errors
    ///
    /// Returns a reason when the pattern fails at run time, or when its
    /// matches do not cover the input. Either case would otherwise encode a
    /// silently truncated prefix, which is worse than a refusal: the caller
    /// would receive plausible ids for text it did not supply.
    fn pre_tokenize<'a>(&self, text: &'a str) -> Result<Vec<&'a str>, String> {
        let Some(pattern) = &self.pattern else {
            return Ok(if text.is_empty() {
                Vec::new()
            } else {
                vec![text]
            });
        };
        let mut fragments = Vec::new();
        let mut covered = 0;
        for found in pattern.find_iter(text) {
            let found =
                found.map_err(|error| format!("pre-tokenization failed on {text:?}: {error}"))?;
            if found.start() != covered {
                return Err(format!(
                    "pre-tokenization skipped bytes {covered}..{} of {text:?}",
                    found.start()
                ));
            }
            covered = found.end();
            fragments.push(found.as_str());
        }
        if covered != text.len() {
            return Err(format!(
                "pre-tokenization covered {covered} of {} bytes in {text:?}",
                text.len()
            ));
        }
        Ok(fragments)
    }

    /// Repeatedly merges the lowest-ranked adjacent pair, which is the BPE
    /// rule: the merge list is ordered, and earlier entries bind first.
    fn apply_merges(&self, token: &str) -> Vec<String> {
        let mut parts = token.chars().map(|c| c.to_string()).collect::<Vec<_>>();
        while parts.len() > 1 {
            let mut best: Option<(usize, usize)> = None;
            for index in 0..parts.len() - 1 {
                let pair = (parts[index].clone(), parts[index + 1].clone());
                if let Some(rank) = self.ranks.get(&pair) {
                    if best.is_none_or(|(_, best_rank)| *rank < best_rank) {
                        best = Some((index, *rank));
                    }
                }
            }
            let Some((index, _)) = best else { break };
            let merged = format!("{}{}", parts[index], parts[index + 1]);
            parts.splice(index..=index + 1, [merged]);
        }
        parts
    }

    /// Finds added tokens by literal longest match, leaving the text between
    /// them for ordinary pre-tokenization.
    fn split_on_added_tokens<'a>(&self, text: &'a str) -> Vec<Segment<'a>> {
        let mut segments = Vec::new();
        let mut cursor = 0;
        let mut pending = 0;
        while cursor < text.len() {
            let mut matched = None;
            for (content, id) in &self.added {
                if text[cursor..].starts_with(content.as_str()) {
                    matched = Some((content.len(), *id));
                    break;
                }
            }
            if let Some((length, id)) = matched {
                if pending < cursor {
                    segments.push(Segment::Text(&text[pending..cursor]));
                }
                segments.push(Segment::Added(id));
                cursor += length;
                pending = cursor;
            } else {
                // Advance one character, not one byte, so multi-byte text is
                // never split mid-scalar.
                cursor += text[cursor..].chars().next().map_or(1, char::len_utf8);
            }
        }
        if pending < text.len() {
            segments.push(Segment::Text(&text[pending..]));
        }
        segments
    }
}

enum Segment<'a> {
    Text(&'a str),
    Added(i64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_byte_alphabet_is_a_bijection_with_the_expected_spellings() {
        let mapping = byte_to_unicode();
        let unique = mapping.iter().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), 256, "every byte needs a distinct glyph");
        assert_eq!(mapping[b'a' as usize], 'a');
        assert_eq!(mapping[b' ' as usize], '\u{0120}', "space is the Ġ glyph");
        assert_eq!(mapping[b'\n' as usize], '\u{010A}');
    }
}
