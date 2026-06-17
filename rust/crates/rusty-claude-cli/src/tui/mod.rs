//! TUI module — terminal user interface for Claw Code.
//!
//! This is a directory module (`tui/mod.rs`) that organizes the TUI into
//! clean sub-modules. The legacy god-struct lives in `legacy.rs` and is
//! re-exported here so all existing `tui::` paths continue to work.

// Legacy — the current TuiApp, untouched during migration.
// All existing tui::* paths resolve through `pub use legacy::*`.
pub mod legacy;
pub use legacy::*;

// New sub-modules — built incrementally alongside the legacy code.
pub mod app;
pub mod capture;
pub mod component;
pub mod components;
pub mod event;
pub mod markdown;
pub mod slash_commands;
