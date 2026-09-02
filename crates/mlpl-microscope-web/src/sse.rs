//! Incremental UTF-8-safe server-sent event framing.

/// One decoded SSE event block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseEvent {
    pub event: String,
    pub data: String,
}

/// Incremental SSE parser that retains incomplete bytes between chunks.
#[derive(Clone, Debug, Default)]
pub struct SseParser {
    pending: Vec<u8>,
}

impl SseParser {
    /// Push arbitrary bytes and return every newly completed event.
    ///
    /// # Errors
    /// Returns an error when a completed event block is not valid UTF-8.
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<SseEvent>, String> {
        self.pending.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(end) = find_boundary(&self.pending) {
            let block = self.pending.drain(..end).collect::<Vec<_>>();
            let delimiter = if self.pending.starts_with(b"\r\n\r\n") {
                4
            } else {
                2
            };
            self.pending.drain(..delimiter);
            let text = std::str::from_utf8(&block).map_err(|error| error.to_string())?;
            if let Some(event) = decode_block(text) {
                events.push(event);
            }
        }
        Ok(events)
    }

    /// Reject a stream that ends with an incomplete event or invalid UTF-8.
    ///
    /// # Errors
    /// Returns an error when bytes remain after the final complete event.
    pub fn finish(self) -> Result<(), String> {
        if self.pending.is_empty() {
            Ok(())
        } else {
            Err("incomplete SSE event".into())
        }
    }
}

fn find_boundary(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(2)
        .position(|pair| pair == b"\n\n")
        .or_else(|| bytes.windows(4).position(|pair| pair == b"\r\n\r\n"))
}

fn decode_block(block: &str) -> Option<SseEvent> {
    let mut event = "message".to_owned();
    let mut data = Vec::new();
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            value.trim_start().clone_into(&mut event);
        }
        if let Some(value) = line.strip_prefix("data:") {
            data.push(value.trim_start());
        }
    }
    (!data.is_empty()).then(|| SseEvent {
        event,
        data: data.join("\n"),
    })
}
