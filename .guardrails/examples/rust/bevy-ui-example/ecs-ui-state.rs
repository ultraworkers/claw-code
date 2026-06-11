// ECS UI State Management - Bevy UI 0.15+
// Production-ready ECS UI state management patterns
//
// Last Updated: 2026-03-14
// Bevy Version: 0.15+
// Rust Version: 1.85+

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use std::collections::HashMap;

/// ECS UI State Component - Central state storage
/// Derives Component for entity attachment, Resource for global access
#[derive(Component, Resource, Clone, Debug, Default, Reflect)]
pub struct UiStateComponent {
    /// Current UI panel type
    pub active_panel: PanelType,
    /// UI theme for accessibility
    pub theme: UiTheme,
    /// Accessibility configuration
    pub accessibility: AccessibilityConfig,
    /// Focus tracking for keyboard navigation
    pub focus_stack: Vec<Entity>,
    /// State machine validation
    pub state_machine: UiStateMachine,
    /// Event queue for reactive updates
    pub event_queue: Vec<UiStateEvent>,
}

/// Panel Type - ECS UI state machine
#[derive(Component, Clone, Debug, Default, Reflect, PartialEq, Eq)]
pub enum PanelType {
    #[default]
    None,
    PlayerHud,
    Inventory,
    Dialogue,
    Settings,
    PauseMenu,
    QuestLog,
    Map,
    CharacterSheet,
    SkillTree,
}

/// UI Theme - Accessibility themes
#[derive(Component, Clone, Debug, Default, Reflect, PartialEq, Eq)]
pub enum UiTheme {
    #[default]
    Default,
    HighContrast,
    Dark,
    Light,
    ColorblindProtanopia,
    ColorblindDeuteranopia,
    ColorblindTritanopia,
}

/// Accessibility Configuration - WCAG compliance
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct AccessibilityConfig {
    /// Screen reader enablement
    pub screen_reader_enabled: bool,
    /// High contrast mode (WCAG AA)
    pub high_contrast: bool,
    /// Reduced motion (motion sensitivity)
    pub reduced_motion: bool,
    /// Keyboard navigation enablement
    pub keyboard_navigation: bool,
    /// Focus indicator visibility
    pub focus_indicators: bool,
    /// Text size multiplier
    pub text_size_multiplier: f32,
    /// Color correction type
    pub color_correction: Option<ColorCorrectionType>,
}

/// Color Correction Type - Colorblind accessibility
#[derive(Component, Clone, Debug, Default, Reflect, PartialEq, Eq)]
pub enum ColorCorrectionType {
    #[default]
    None,
    Protanopia,    // Red-green
    Deuteranopia,  // Green-red
    Tritanopia,    // Blue-yellow
}

/// UI State Machine - Validation rules
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct UiStateMachine {
    /// Current state
    pub current: PanelType,
    /// Valid transitions
    pub transitions: HashMap<PanelType, Vec<PanelType>>,
    /// Validation enabled
    pub validation_enabled: bool,
}

/// UI State Event - ECS event-driven updates
#[derive(Event, Clone, Debug, Reflect)]
pub enum UiStateEvent {
    /// Panel transition request
    PanelTransition(PanelType, PanelType),
    /// Theme change request
    ThemeChange(UiTheme),
    /// Accessibility toggle
    AccessibilityToggle(String, bool),
    /// Focus change
    FocusChanged(Entity),
    /// State validation failure
    ValidationFailure(String),
    /// State sync request
    StateSync(PanelType),
}

/// Focus Component - Keyboard navigation
#[derive(Component, Debug, Default)]
pub struct FocusComponent {
    /// Focus order index
    pub order: usize,
    /// Focusable entity
    pub entity: Entity,
    /// Focus indicator visible
    pub indicator_visible: bool,
}

/// Label Component - ARIA-like accessibility
#[derive(Component, Debug, Default, Reflect)]
pub struct LabelComponent {
    /// Accessible label text
    pub label: String,
    /// Label type (heading, button, etc.)
    pub label_type: LabelType,
    /// Screen reader description
    pub description: Option<String>,
}

/// Label Type - ARIA role mapping
#[derive(Component, Clone, Debug, Default, Reflect, PartialEq, Eq)]
pub enum LabelType {
    #[default]
    None,
    Heading,
    Button,
    Input,
    Checkbox,
    Radio,
    Link,
    Image,
    Region,
    Dialog,
    Alert,
    Status,
}

/// Implementation: UI State Component
impl UiStateComponent {
    /// Create new UI state component
    pub fn new() -> Self {
        Self {
            active_panel: PanelType::None,
            theme: UiTheme::Default,
            accessibility: AccessibilityConfig::default(),
            focus_stack: Vec::new(),
            state_machine: UiStateMachine::default(),
            event_queue: Vec::new(),
        }
    }

    /// Initialize state machine transitions
    pub fn init_transitions(&mut self) {
        // Valid panel transitions - systemic consistency
        self.state_machine.transitions = HashMap::from([
            (PanelType::None, vec![PanelType::PlayerHud]),
            (PanelType::PlayerHud, vec![PanelType::Inventory, PanelType::PauseMenu]),
            (PanelType::Inventory, vec![PanelType::PlayerHud, PanelType::Settings]),
            (PanelType::Dialogue, vec![PanelType::PlayerHud, PanelType::None]),
            (PanelType::Settings, vec![PanelType::Inventory, PanelType::PauseMenu]),
            (PanelType::PauseMenu, vec![PanelType::PlayerHud, PanelType::Settings]),
            (PanelType::QuestLog, vec![PanelType::PlayerHud, PanelType::Map]),
            (PanelType::Map, vec![PanelType::QuestLog, PanelType::PlayerHud]),
            (PanelType::CharacterSheet, vec![PanelType::PlayerHud, PanelType::SkillTree]),
            (PanelType::SkillTree, vec![PanelType::CharacterSheet, PanelType::PlayerHud]),
        ]);
        self.state_machine.validation_enabled = true;
    }

    /// Validate panel transition - systemic consistency
    pub fn validate_transition(&self, from: PanelType, to: PanelType) -> bool {
        if !self.state_machine.validation_enabled {
            return true;
        }

        self.state_machine
            .transitions
            .get(&from)
            .map_or(false, |targets| targets.contains(&to))
    }

    /// Request panel transition with validation
    pub fn request_transition(&mut self, to: PanelType) -> Result<(), String> {
        let from = self.active_panel.clone();

        if !self.validate_transition(from.clone(), to.clone()) {
            let error = format!(
                "Invalid transition: {} -> {}",
                from, to
            );
            self.event_queue.push(UiStateEvent::ValidationFailure(error));
            return Err(error);
        }

        self.active_panel = to;
        self.event_queue.push(UiStateEvent::PanelTransition(from, to));
        Ok(())
    }

    /// Apply theme - accessibility
    pub fn set_theme(&mut self, theme: UiTheme) {
        self.theme = theme.clone();
        self.event_queue.push(UiStateEvent::ThemeChange(theme));
    }

    /// Toggle accessibility setting
    pub fn toggle_accessibility(&mut self, setting: String, value: bool) {
        match setting.as_str() {
            "screen_reader" => self.accessibility.screen_reader_enabled = value,
            "high_contrast" => self.accessibility.high_contrast = value,
            "reduced_motion" => self.accessibility.reduced_motion = value,
            "keyboard_navigation" => self.accessibility.keyboard_navigation = value,
            "focus_indicators" => self.accessibility.focus_indicators = value,
            _ => {
                self.event_queue.push(UiStateEvent::ValidationFailure(
                    format!("Unknown setting: {}", setting)
                ));
            }
        }
        self.event_queue.push(UiStateEvent::AccessibilityToggle(setting, value));
    }

    /// Push focus entity - focus management
    pub fn push_focus(&mut self, entity: Entity) {
        self.focus_stack.push(entity);
        self.event_queue.push(UiStateEvent::FocusChanged(entity));
    }

    /// Pop focus entity - focus management
    pub fn pop_focus(&mut self) -> Option<Entity> {
        let entity = self.focus_stack.pop();
        if let Some(e) = &entity {
            self.event_queue.push(UiStateEvent::FocusChanged(*e));
        }
        entity
    }

    /// Get current focus - focus management
    pub fn current_focus(&self) -> Option<Entity> {
        self.focus_stack.last().copied()
    }

    /// Clear event queue
    pub fn clear_events(&mut self) {
        self.event_queue.clear();
    }
}

/// Implementation: UI State Machine
impl UiStateMachine {
    /// Create new state machine
    pub fn new() -> Self {
        Self {
            current: PanelType::None,
            transitions: HashMap::new(),
            validation_enabled: true,
        }
    }

    /// Add valid transition
    pub fn add_transition(&mut self, from: PanelType, to: PanelType) {
        self.transitions
            .entry(from)
            .or_insert(Vec::new())
            .push(to);
    }

    /// Get valid transitions from state
    pub fn get_valid_transitions(&self, from: PanelType) -> Vec<PanelType> {
        self.transitions.get(&from).cloned().unwrap_or_default()
    }

    /// Is state valid?
    pub fn is_valid_state(&self, state: PanelType) -> bool {
        self.transitions.contains_key(&state) || state == PanelType::None
    }
}

/// System: UI state system - Processes events, updates state
pub fn ui_state_system(
    mut ui_state: ResMut<UiStateComponent>,
    mut events: Events<UiStateEvent>,
    mut commands: Commands,
) {
    // Process event queue
    for event in ui_state.event_queue.iter() {
        match event {
            UiStateEvent::PanelTransition(from, to) => {
                log::info!("Panel transition: {} -> {}", from, to);
                commands.spawn(Name::new(format("Transition: {} -> {}", from, to)));
            }
            UiStateEvent::ThemeChange(theme) => {
                log::info!("Theme changed: {}", theme);
            }
            UiStateEvent::AccessibilityToggle(setting, value) => {
                log::info!("Accessibility: {} = {}", setting, value);
            }
            UiStateEvent::FocusChanged(entity) => {
                log::info!("Focus changed to: {:?}", entity);
            }
            UiStateEvent::ValidationFailure(error) => {
                log::error!("Validation failure: {}", error);
            }
            UiStateEvent::StateSync(panel) => {
                log::info!("State sync request: {}", panel);
            }
        }
    }

    // Clear processed events
    ui_state.clear_events();
}

/// System: Focus management system
pub fn focus_management_system(
    ui_state: Res<UiStateComponent>,
    keyboard_input: Res<InputMap>,
    mut commands: Commands,
) {
    // Tab: push next focus
    if keyboard_input.tab {
        if let Some(current) = ui_state.current_focus() {
            let next = Entity::from_bits(current.to_bits() + 1);
            ui_state.push_focus(next);
        }
    }

    // Shift+Tab: pop focus
    if keyboard_input.shift_tab {
        ui_state.pop_focus();
    }

    // Escape: close panel
    if keyboard_input.escape {
        ui_state.request_transition(PanelType::None).ok();
    }
}

/// System: Accessibility system
pub fn accessibility_system(
    ui_state: Res<UiStateComponent>,
    mut commands: Commands,
) {
    // Apply high contrast theme
    if ui_state.accessibility.high_contrast {
        ui_state.set_theme(UiTheme::HighContrast);
    }

    // Apply color correction
    if let Some(correction) = &ui_state.accessibility.color_correction {
        match correction {
            ColorCorrectionType::Protanopia => {
                log::info!("Applying protanopia color correction");
            }
            ColorCorrectionType::Deuteranopia => {
                log::info!("Applying deuteranopia color correction");
            }
            ColorCorrectionType::Tritanopia => {
                log::info!("Applying tritanopia color correction");
            }
            _ => {}
        }
    }
}

/// Input Map - Keyboard navigation state
#[derive(Resource, Default)]
pub struct InputMap {
    pub tab: bool,
    pub shift_tab: bool,
    pub enter: bool,
    pub escape: bool,
    pub arrow_up: bool,
    pub arrow_down: bool,
    pub arrow_left: bool,
    pub arrow_right: bool,
}