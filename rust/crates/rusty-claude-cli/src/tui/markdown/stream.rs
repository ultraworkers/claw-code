//! Streaming markdown state — placeholder for Phase 3.
//!
//! Incremental rendering: accumulates text deltas, finds safe boundaries,
//! parses/renders complete markdown blocks.

/// Streaming markdown state — stub for Phase 3.
pub struct StreamingMarkdownState {
    pending: String,
}

impl StreamingMarkdownState {
    pub fn new() -> Self {
        Self { pending: String::new() }
    }

    pub fn push_delta(&mut self, delta: &str) -> bool {
        self.pending.push_str(delta);
        true
    }

    pub fn flush(&mut self) -> String {
        std::mem::take(&mut self.pending)
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}

impl Default for StreamingMarkdownState {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulates() {
        let mut s = StreamingMarkdownState::new();
        s.push_delta("Hello ");
        s.push_delta("world");
        assert!(s.has_pending());
        assert_eq!(s.flush(), "Hello world");
        assert!(!s.has_pending());
    }
}
