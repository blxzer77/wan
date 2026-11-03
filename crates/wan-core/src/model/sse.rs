//! Incremental Server-Sent Events parser. Chunks may split lines anywhere, including inside a
//! multi-byte character, so the buffer holds raw bytes until a full line is available.

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

#[derive(Debug, Default)]
pub struct SseParser {
    buffer: Vec<u8>,
    event: Option<String>,
    data: Vec<String>,
}

impl SseParser {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(end) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=end).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches(['\n', '\r']);
            if let Some(event) = self.line(line) {
                events.push(event);
            }
        }
        events
    }

    /// Flushes an event that was not terminated by a blank line before the stream closed.
    pub fn finish(&mut self) -> Option<SseEvent> {
        if !self.buffer.is_empty() {
            let line = String::from_utf8_lossy(&self.buffer).into_owned();
            self.buffer.clear();
            if let Some(event) = self.line(line.trim_end_matches('\r')) {
                return Some(event);
            }
        }
        self.dispatch()
    }

    fn line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            return self.dispatch();
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "event" => self.event = Some(value.to_owned()),
            "data" => self.data.push(value.to_owned()),
            _ => {}
        }
        None
    }

    fn dispatch(&mut self) -> Option<SseEvent> {
        let event = self.event.take();
        if self.data.is_empty() {
            return None;
        }
        Some(SseEvent {
            event,
            data: std::mem::take(&mut self.data).join("\n"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(text: &str) -> SseEvent {
        SseEvent {
            event: None,
            data: text.to_owned(),
        }
    }

    #[test]
    fn parses_named_events_and_comments() {
        let mut parser = SseParser::default();

        let events = parser.push(b": ping\nevent: response.created\ndata: {\"a\":1}\n\n");

        assert_eq!(
            events,
            vec![SseEvent {
                event: Some("response.created".to_owned()),
                data: "{\"a\":1}".to_owned(),
            }]
        );
    }

    #[test]
    fn joins_multiple_data_lines_and_handles_crlf() {
        let mut parser = SseParser::default();

        let events = parser.push(b"data: one\r\ndata: two\r\n\r\n");

        assert_eq!(events, vec![data("one\ntwo")]);
    }

    #[test]
    fn reassembles_lines_split_across_chunks_and_characters() {
        let mut parser = SseParser::default();
        let bytes = "data: 你好\n\n".as_bytes();

        // Split inside the three-byte encoding of the first character.
        let mut events = parser.push(&bytes[..8]);
        events.extend(parser.push(&bytes[8..]));

        assert_eq!(events, vec![data("你好")]);
    }

    #[test]
    fn finish_flushes_an_unterminated_event() {
        let mut parser = SseParser::default();

        assert!(parser.push(b"data: [DONE]").is_empty());

        assert_eq!(parser.finish(), Some(data("[DONE]")));
        assert_eq!(parser.finish(), None);
    }
}
