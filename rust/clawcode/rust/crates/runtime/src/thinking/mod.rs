//! Thinking-block primitives: parser, embedded-tool extractor, and renderer.
//!
//! This module consolidates all the code that deals with reasoning /
//! chain-of-thought text in model responses. It is a leaf module — it has
//! no upward dependencies and is depended on by `agents` and
//! `claw-cli`.
//!
//! Sub-modules:
//! - [`parser`]: `ThinkParser` — streams `<think>…</think>` tag boundaries
//!   across chunks and splits `(visible, reasoning)` deltas.
//! - [`extract`]: `extract_embedded_tools` — parses `<tool_call>` XML
//!   blocks that models emit inside their thinking and converts them into
//!   `(id, name, input)` tuples.
//! - [`render`]: `render_reasoning` — formats the accumulated reasoning
//!   text as an ANSI-styled terminal string with a `┃` gutter and a
//!   `Thinking:` / `Thought:` label.

pub mod extract;
pub mod parser;
pub mod render;

pub use extract::extract_embedded_tools;
pub use parser::ThinkParser;
pub use render::{render_reasoning, ReasoningTheme};
