use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct IncrementalSseParser {
    buffer: String,
    event_name: Option<String>,
    data_lines: Vec<String>,
    id: Option<String>,
    retry: Option<u64>,
}

impl IncrementalSseParser {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_chunk(&mut self, chunk: &str) -> Vec<SseEvent> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();

        while let Some(index) = self.buffer.find('\n') {
            let mut line = self.buffer.drain(..=index).collect::<String>();
            if line.ends_with('\n') {
                line.pop();
            }
            if line.ends_with('\r') {
                line.pop();
            }
            self.process_line(&line, &mut events);
        }

        events
    }

    pub fn finish(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();
        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);
            self.process_line(line.trim_end_matches('\r'), &mut events);
        }
        if let Some(event) = self.take_event() {
            events.push(event);
        }
        events
    }

    fn process_line(&mut self, line: &str, events: &mut Vec<SseEvent>) {
        if line.is_empty() {
            if let Some(event) = self.take_event() {
                events.push(event);
            }
            return;
        }

        if line.starts_with(':') {
            return;
        }

        let (field, value) = line.split_once(':').map_or((line, ""), |(field, value)| {
            let trimmed = value.strip_prefix(' ').unwrap_or(value);
            (field, trimmed)
        });

        match field {
            "event" => self.event_name = Some(value.to_owned()),
            "data" => self.data_lines.push(value.to_owned()),
            "id" => self.id = Some(value.to_owned()),
            "retry" => self.retry = value.parse::<u64>().ok(),
            _ => {}
        }
    }

    fn take_event(&mut self) -> Option<SseEvent> {
        if self.data_lines.is_empty()
            && self.event_name.is_none()
            && self.id.is_none()
            && self.retry.is_none()
        {
            return None;
        }

        let data = self.data_lines.join("\n");
        self.data_lines.clear();

        Some(SseEvent {
            event: self.event_name.take(),
            data,
            id: self.id.take(),
            retry: self.retry.take(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{IncrementalSseParser, SseEvent};

    #[test]
    fn parses_streaming_events() {
        // given
        let mut parser = IncrementalSseParser::new();

        // when
        let first = parser.push_chunk("event: message\ndata: hel");

        // then
        assert!(first.is_empty());

        let second = parser.push_chunk("lo\n\nid: 1\ndata: world\n\n");
        assert_eq!(
            second,
            vec![
                SseEvent {
                    event: Some(String::from("message")),
                    data: String::from("hello"),
                    id: None,
                    retry: None,
                },
                SseEvent {
                    event: None,
                    data: String::from("world"),
                    id: Some(String::from("1")),
                    retry: None,
                },
            ]
        );
    }

    #[test]
    fn finish_flushes_a_trailing_event_without_separator() {
        // given
        let mut parser = IncrementalSseParser::new();
        parser.push_chunk("event: message\ndata: trailing");

        // when
        let events = parser.finish();

        // then
        assert_eq!(
            events,
            vec![SseEvent {
                event: Some("message".to_string()),
                data: "trailing".to_string(),
                id: None,
                retry: None,
            }]
        );
    }
}
