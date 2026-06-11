// Debug Overlay - egui Debug/Developer UI
// Production-ready debug overlay patterns for game engines
//
// Last Updated: 2026-03-14
// egui Version: 0.27+
// Rust Version: 1.85+

use egui::{Context, Color32, FontId, TextStyle, Align, Layout};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// DebugOverlay - Main debug overlay structure
/// Conditional compilation for production safety
#[cfg(debug)]
#[derive(Clone, Debug)]
pub struct DebugOverlay {
    /// egui context reference
    pub ctx: Arc<Context>,
    /// FPS counter
    pub fps: f32,
    /// Frame time
    pub frame_time_ms: f32,
    /// Entity inspector state
    pub inspector_open: bool,
    /// Console log buffer
    pub log_buffer: VecDeque<String>,
    /// Debug state
    pub debug_state: DebugState,
    /// Accessibility config
    pub accessibility: DebugAccessibility,
}

/// DebugState - Debug overlay state machine
#[cfg(debug)]
#[derive(Clone, Debug, Default)]
pub struct DebugState {
    /// Overlay enabled
    pub enabled: bool,
    /// FPS counter enabled
    pub fps_enabled: bool,
    /// Inspector enabled
    pub inspector_enabled: bool,
    /// Console enabled
    pub console_enabled: bool,
    /// Performance metrics enabled
    pub perf_metrics_enabled: bool,
    /// Memory tracking enabled
    pub memory_tracking_enabled: bool,
}

/// DebugAccessibility - Debug overlay accessibility
#[cfg(debug)]
#[derive(Clone, Debug, Default)]
pub struct DebugAccessibility {
    /// High contrast theme
    pub high_contrast: bool,
    /// Large text mode
    pub large_text: bool,
    /// Keyboard navigation
    pub keyboard_navigation: bool,
    /// Screen reader output
    pub screen_reader_output: bool,
}

/// Implementation: DebugOverlay
#[cfg(debug)]
impl DebugOverlay {
    /// Create new debug overlay
    pub fn new(ctx: Context) -> Self {
        Self {
            ctx: Arc::new(ctx),
            fps: 0.0,
            frame_time_ms: 0.0,
            inspector_open: false,
            log_buffer: VecDeque::with_capacity(100),
            debug_state: DebugState::default(),
            accessibility: DebugAccessibility::default(),
        }
    }

    /// Initialize debug overlay
    pub fn init(&mut self) {
        self.debug_state.enabled = true;
        self.debug_state.fps_enabled = true;
        self.debug_state.console_enabled = true;

        // Apply accessibility theme
        if self.accessibility.high_contrast {
            self.apply_high_contrast_theme();
        }

        log::info!("Debug overlay initialized");
    }

    /// Apply high contrast theme - accessibility
    fn apply_high_contrast_theme(&mut self) {
        let mut style = (*self.ctx.style()).clone();
        style.colors[egui::Spacing::Background] = Color32::BLACK;
        style.colors[egui::Spacing::Foreground] = Color32::WHITE;
        style.colors[egui::Spacing::PanelBackground] = Color32::BLACK;
        self.ctx.set_style(style);
    }

    /// Update FPS counter
    pub fn update_fps(&mut self, fps: f32, frame_time_ms: f32) {
        self.fps = fps;
        self.frame_time_ms = frame_time_ms;
    }

    /// Log message to console buffer
    pub fn log(&mut self, message: String) {
        self.log_buffer.push_front(message);
        if self.log_buffer.len() > 100 {
            self.log_buffer.pop_back();
        }
    }

    /// Render debug overlay
    pub fn render(&self) {
        let ctx = &self.ctx;

        // Top panel - FPS counter
        egui::TopBottomPanel::top("debug_top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.debug_state.fps_enabled {
                    ui.colored_label(
                        Color32::GREEN,
                        format!("FPS: {:.1} | Frame: {:.2}ms", self.fps, self.frame_time_ms),
                    );
                }

                if self.debug_state.perf_metrics_enabled {
                    ui.label(format!("Memory: {} MB", self.get_memory_usage()));
                }
            });
        });

        // Left panel - Entity inspector
        if self.inspector_open && self.debug_state.inspector_enabled {
            egui::SidePanel::left("debug_inspector").show(ctx, |ui| {
                ui.heading("Entity Inspector");
                ui.label("Entity count: {}", self.get_entity_count());
                ui.label("Active systems: {}", self.get_system_count());

                // Entity list with accessibility
                ui.vertical(|ui| {
                    ui.set_accessibility(self.accessibility.keyboard_navigation);
                    for i in 0..self.get_entity_count() {
                        ui.label(format("Entity {}", i));
                    }
                });
            });
        }

        // Bottom panel - Console
        if self.debug_state.console_enabled {
            egui::TopBottomPanel::bottom("debug_console").show(ctx, |ui| {
                ui.set_min_height(150.0);
                ui.heading("Debug Console");

                // Log buffer with scrolling
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for log in &self.log_buffer {
                        ui.label(log);
                    }
                });

                // Input field
                ui.horizontal(|ui| {
                    ui.label("> ");
                    ui.add(egui::TextEdit::singleline(""));
                });
            });
        }

        // Window - Performance metrics
        if self.debug_state.perf_metrics_enabled {
            egui::Window::new("Performance").show(ctx, |ui| {
                ui.heading("Performance Metrics");
                ui.label(format!("FPS: {:.1}", self.fps));
                ui.label(format!("Frame time: {:.2}ms", self.frame_time_ms));
                ui.label(format!("Memory: {} MB", self.get_memory_usage()));
                ui.label(format!("Entities: {}", self.get_entity_count()));
                ui.label(format!("Systems: {}", self.get_system_count()));
            });
        }

        // Window - Memory tracking
        if self.debug_state.memory_tracking_enabled {
            egui::Window::new("Memory").show(ctx, |ui| {
                ui.heading("Memory Tracking");
                ui.label(format("Total: {} MB", self.get_memory_usage()));
                ui.label(format("Allocations: {}", self.get_allocation_count()));
                ui.label(format("Dealocations: {}", self.get_deallocation_count()));
            });
        }
    }

    /// Get memory usage - mock implementation
    fn get_memory_usage(&self) -> u32 {
        50 // Mock value
    }

    /// Get entity count - mock implementation
    fn get_entity_count(&self) -> usize {
        100 // Mock value
    }

    /// Get system count - mock implementation
    fn get_system_count(&self) -> usize {
        10 // Mock value
    }

    /// Get allocation count - mock implementation
    fn get_allocation_count(&self) -> usize {
        1000 // Mock value
    }

    /// Get deallocation count - mock implementation
    fn get_deallocation_count(&self) -> usize {
        950 // Mock value
    }

    /// Toggle inspector
    pub fn toggle_inspector(&mut self) {
        self.inspector_open = !self.inspector_open;
        self.log(format!("Inspector: {}", self.inspector_open));
    }

    /// Toggle console
    pub fn toggle_console(&mut self) {
        self.debug_state.console_enabled = !self.debug_state.console_enabled;
        self.log(format!("Console: {}", self.debug_state.console_enabled));
    }

    /// Toggle FPS counter
    pub fn toggle_fps(&mut self) {
        self.debug_state.fps_enabled = !self.debug_state.fps_enabled;
        self.log(format!("FPS counter: {}", self.debug_state.fps_enabled));
    }

    /// Enable accessibility
    pub fn enable_accessibility(&mut self, enabled: bool) {
        self.accessibility.high_contrast = enabled;
        self.accessibility.keyboard_navigation = enabled;
        self.accessibility.screen_reader_output = enabled;

        if enabled {
            self.apply_high_contrast_theme();
        }

        self.log(format!("Accessibility: {}", enabled));
    }
}

/// Debug overlay system - Main render loop
#[cfg(debug)]
pub fn debug_overlay_system(
    mut overlay: ResMut<DebugOverlay>,
    fps: Res<FpsCounter>,
    frame_time: Res<FrameTime>,
) {
    // Update FPS
    overlay.update_fps(fps.value, frame_time.value_ms);

    // Render overlay
    overlay.render();
}

/// FPS Counter - Resource
#[derive(Resource, Debug)]
pub struct FpsCounter {
    pub value: f32,
}

/// Frame Time - Resource
#[derive(Resource, Debug)]
pub struct FrameTime {
    pub value_ms: f32,
}

/// Resource marker for debug overlay
#[derive(Resource, Debug)]
pub struct DebugOverlayMarker;

/// Conditional compilation for production builds
#[cfg(not(debug))]
pub struct DebugOverlay; // Empty stub

#[cfg(not(debug))]
impl DebugOverlay {
    pub fn new(_ctx: Context) -> Self { Self }
    pub fn init(&mut self) {}
    pub fn render(&self) {}
    pub fn log(&mut self, _message: String) {}
}