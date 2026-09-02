//! Structural and duplicate-field validation.
use crate::validation::failure;
use crate::{Recording, ValidationError, ValidationKind};

macro_rules! space {
    ($parser:expr) => {
        while $parser
            .bytes
            .get($parser.cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            $parser.cursor += 1;
        }
    };
}
pub(crate) fn validate_structure(recording: &Recording) -> Result<(), ValidationError> {
    if recording.lesson.is_empty() || recording.frames.is_empty() {
        return Err(failure(
            ValidationKind::Structure,
            "lesson and frames must be non-empty",
        ));
    }
    for frame in &recording.frames {
        if frame.observations.is_empty()
            || frame.observations.iter().any(|item| item.name.is_empty())
        {
            return Err(failure(
                ValidationKind::Structure,
                "observation names and frame observations must be non-empty",
            ));
        }
        if frame
            .observations
            .iter()
            .flat_map(|item| &item.values)
            .any(|value| !value.is_finite())
        {
            return Err(failure(
                ValidationKind::Numeric,
                "observation values must be finite",
            ));
        }
    }
    Ok(())
}
pub(crate) fn validate_steps(recording: &Recording) -> Result<(), ValidationError> {
    if recording
        .frames
        .windows(2)
        .any(|pair| pair[0].step >= pair[1].step)
    {
        Err(failure(
            ValidationKind::StepOrder,
            "steps must be strictly increasing",
        ))
    } else {
        Ok(())
    }
}
pub(crate) fn reject_duplicate_keys(input: &str) -> Result<(), ValidationError> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        cursor: 0,
    };
    parser.value()?;
    space!(parser);
    if parser.cursor == parser.bytes.len() {
        Ok(())
    } else {
        Err(failure(ValidationKind::Structure, "trailing JSON data"))
    }
}
struct Parser<'a> {
    bytes: &'a [u8],
    cursor: usize,
}
impl Parser<'_> {
    fn value(&mut self) -> Result<(), ValidationError> {
        space!(self);
        match self.bytes.get(self.cursor) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(|_| ()),
            Some(_) => {
                let start = self.cursor;
                while self.bytes.get(self.cursor).is_some_and(|byte| {
                    !matches!(byte, b',' | b']' | b'}' | b' ' | b'\n' | b'\r' | b'\t')
                }) {
                    self.cursor += 1;
                }
                if start == self.cursor {
                    Err(failure(ValidationKind::Structure, "invalid JSON value"))
                } else {
                    Ok(())
                }
            }
            None => Err(failure(ValidationKind::Structure, "unexpected end of JSON")),
        }
    }
    fn object(&mut self) -> Result<(), ValidationError> {
        self.cursor += 1;
        let mut keys = std::collections::BTreeSet::new();
        space!(self);
        while self.bytes.get(self.cursor) != Some(&b'}') {
            let key = self.string()?;
            if !keys.insert(key) {
                return Err(failure(
                    ValidationKind::Structure,
                    "duplicate structural field",
                ));
            }
            space!(self);
            if self.bytes.get(self.cursor) != Some(&b':') {
                return Err(failure(ValidationKind::Structure, "missing object colon"));
            }
            self.cursor += 1;
            self.value()?;
            space!(self);
            if self.bytes.get(self.cursor) == Some(&b',') {
                self.cursor += 1;
                space!(self);
            } else {
                break;
            }
        }
        if self.bytes.get(self.cursor) != Some(&b'}') {
            return Err(failure(ValidationKind::Structure, "unterminated object"));
        }
        self.cursor += 1;
        Ok(())
    }
    fn array(&mut self) -> Result<(), ValidationError> {
        self.cursor += 1;
        space!(self);
        while self.bytes.get(self.cursor) != Some(&b']') {
            self.value()?;
            space!(self);
            if self.bytes.get(self.cursor) == Some(&b',') {
                self.cursor += 1;
                space!(self);
            } else {
                break;
            }
        }
        if self.bytes.get(self.cursor) != Some(&b']') {
            return Err(failure(ValidationKind::Structure, "unterminated array"));
        }
        self.cursor += 1;
        Ok(())
    }
    fn string(&mut self) -> Result<String, ValidationError> {
        space!(self);
        if self.bytes.get(self.cursor) != Some(&b'"') {
            return Err(failure(
                ValidationKind::Structure,
                "object key must be a string",
            ));
        }
        let start = self.cursor;
        self.cursor += 1;
        let mut escaped = false;
        while let Some(byte) = self.bytes.get(self.cursor) {
            self.cursor += 1;
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                return serde_json::from_slice(&self.bytes[start..self.cursor])
                    .map_err(|error| failure(ValidationKind::Structure, error.to_string()));
            }
        }
        Err(failure(ValidationKind::Structure, "unterminated string"))
    }
}
