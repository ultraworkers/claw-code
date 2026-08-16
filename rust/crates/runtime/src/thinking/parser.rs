//! Stream-aware `<think>…</think>` tag parser used to separate
//! chain-of-thought reasoning from visible assistant text in streaming
//! LLM responses.
//!
//! Some reasoning models (DeepSeek-R1 distilled, GLM-Z1, some Qwen
//! reasoning variants) emit thinking content as inline `<think>…</think>`
//! tags within text deltas instead of (or in addition to) using the
//! provider's native thinking-block content type. This parser detects
//! those tags and splits each chunk into a `(visible, reasoning)` pair.
//!
//! The parser is stateful and chunk-aware: a tag that straddles two
//! `push` calls is correctly handled by retaining a small suffix buffer.

/// Stream-aware `<think>…</think>` tag parser.
///
/// Maintains an internal buffer to handle tag boundaries that split
/// across chunks. Call [`ThinkParser::push`] for each incoming text
/// delta and [`ThinkParser::finish`] when the stream ends.
#[derive(Clone, Debug, Default)]
pub struct ThinkParser {
    in_think: bool,
    buffer: String,
}

impl ThinkParser {
    /// Construct a new empty parser.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a chunk of streaming text and return `(visible, reasoning)`
    /// for that chunk. Both strings contain only the *delta* produced by
    /// this call — call again to receive the next delta.
    ///
    /// Empty input returns `(String::new(), String::new())`.
    pub fn push(&mut self, text: &str) -> (String, String) {
        if text.is_empty() {
            return (String::new(), String::new());
        }
        self.buffer.push_str(text);

        let mut visible = String::new();
        let mut reasoning = String::new();

        loop {
            if self.in_think {
                if let Some(end) = self.buffer.find("</think>") {
                    reasoning.push_str(&self.buffer[..end]);
                    self.buffer.drain(..end + "</think>".len());
                    self.in_think = false;
                    continue;
                }

                let keep = think_tag_suffix_len(&self.buffer);
                let split = self.buffer.len().saturating_sub(keep);
                reasoning.push_str(&self.buffer[..split]);
                self.buffer.drain(..split);
                break;
            }

            if let Some(start) = self.buffer.find("<think>") {
                visible.push_str(&self.buffer[..start]);
                self.buffer.drain(..start + "<think>".len());
                self.in_think = true;
                continue;
            }

            let keep = think_tag_suffix_len(&self.buffer);
            let split = self.buffer.len().saturating_sub(keep);
            visible.push_str(&self.buffer[..split]);
            self.buffer.drain(..split);
            break;
        }

        (visible, reasoning)
    }

    /// Drain any remaining buffered text. Call this once after the stream
    /// has ended. An unterminated think-block (i.e. `<think>` with no
    /// matching `</think>`) is flushed as reasoning.
    pub fn finish(&mut self) -> (String, String) {
        let mut visible = String::new();
        let mut reasoning = String::new();

        if self.in_think {
            reasoning.push_str(&self.buffer);
        } else {
            visible.push_str(&self.buffer);
        }

        self.buffer.clear();
        (visible, reasoning)
    }
}

/// Returns the maximum suffix of `text` that could be a partial
/// `<think>` or `</think>` tag. This prevents splitting a multi-chunk
/// tag boundary: the parser retains that many trailing characters in its
/// buffer instead of emitting them.
fn think_tag_suffix_len(text: &str) -> usize {
    const TAGS: [&str; 2] = ["</think>", "<think>"];

    for tag in TAGS {
        let max = tag.len().saturating_sub(1);
        for keep in (1..=max).rev() {
            if text.ends_with(&tag[..keep]) {
                return keep;
            }
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::ThinkParser;

    #[test]
    fn think_parser_single_chunk() {
        let mut p = ThinkParser::new();
        let (v, r) = p.push("<think>hidden</think>visible");
        assert_eq!(v, "visible");
        assert_eq!(r, "hidden");
    }

    #[test]
    fn think_parser_split_across_chunks() {
        let mut p = ThinkParser::new();
        // first push: <think> is complete → enter think mode; "hid" has
        // no potential partial-tag suffix, so it is emitted as reasoning
        let (v1, r1) = p.push("<think>hid");
        assert_eq!(v1, "");
        assert_eq!(r1, "hid");
        // second push: </think> closes the think block; remaining text
        // is visible
        let (v2, r2) = p.push("den</think>visible");
        assert_eq!(v2, "visible");
        assert_eq!(r2, "den");
        let (vf, rf) = p.finish();
        assert_eq!(vf, "");
        assert_eq!(rf, "");
    }

    #[test]
    fn think_parser_unterminated() {
        let mut p = ThinkParser::new();
        // "<think>no close" — <think> is recognized, "no close" is
        // emitted as reasoning on the same push (no partial-tag suffix
        // to retain). finish() then has nothing left to drain.
        let (v, r) = p.push("<think>no close");
        assert_eq!(v, "");
        assert_eq!(r, "no close");
        let (vf, rf) = p.finish();
        assert_eq!(vf, "");
        assert_eq!(rf, "");
    }

    #[test]
    fn think_parser_no_think_tag() {
        let mut p = ThinkParser::new();
        let (v, r) = p.push("just visible text");
        assert_eq!(v, "just visible text");
        assert_eq!(r, "");
        let (vf, rf) = p.finish();
        assert_eq!(vf, "");
        assert_eq!(rf, "");
    }

    #[test]
    fn think_parser_partial_tag_at_end() {
        // ensure trailing "<thi" is retained, not emitted
        let mut p = ThinkParser::new();
        let (v, r) = p.push("hello <thi");
        assert_eq!(v, "hello ");
        assert_eq!(r, "");
        let (v2, r2) = p.push("nk>hidden</think>ok");
        assert_eq!(v2, "ok");
        assert_eq!(r2, "hidden");
    }
}
