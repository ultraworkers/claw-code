// Bevy UI Example - ECS-Based Game Interfaces
// Production-ready Bevy UI 0.15+ patterns for ECS-based game user interfaces
//
// Last Updated: 2026-03-14
// Bevy Version: 0.15+
// Rust Version: 1.85+

use bevy::prelude::*;
use bevy::ecs::system::SystemId;
use bevy::ui::UiSystem;

mod ecs_ui_state;
mod zero_copy_transfer;

use ecs_ui_state::{UiStateComponent, UiTheme, AccessibilityConfig, ui_state_system};
use zero_copy_transfer::{ZeroCopyUiState, Pod, Zeroable};

// Main Bevy app with UI camera
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(bevy::ui::ui_systems())
        .add_systems(ui_state_system)
        .add_systems(update_ui_component_system)
        .add_systems(accessibility_system)
        .insert_resource(UiStateComponent::default())
        .run();
}

// UI Camera setup for dedicated UI rendering
fn setup_ui_camera(mut commands: Commands) {
    // UI camera with 2D orthographic projection
    commands.spawn((
        Camera2dBundle::default(),
        CameraUiBundle::default(),
        Name::new("UI Camera"),
    ));
}

// UI State Component - ECS state storage
#[derive(Component, Clone, Debug, Default, Resource)]
struct PlayerUiState {
    health: u32,
    mana: u32,
    experience: u32,
    level: u32,
    focus_entity: Option<Entity>,
    theme: UiTheme,
    accessibility: AccessibilityConfig,
}

// UI Theme - High contrast accessibility
#[derive(Component, Clone, Debug, Default, Reflect)]
enum UiTheme {
    #[default]
    Default,
    HighContrast,
    Dark,
    Light,
    Colorblind,
}

// Accessibility Configuration
#[derive(Component, Clone, Debug, Default, Reflect)]
struct AccessibilityConfig {
    screen_reader_enabled: bool,
    high_contrast: bool,
    reduced_motion: bool,
    keyboard_navigation: bool,
    focus_indicators: bool,
}

// UI Component - Player HUD
#[derive(Component, Debug)]
struct PlayerHud {
    health_bar: Entity,
    mana_bar: Entity,
    xp_bar: Entity,
    level_text: Entity,
}

// UI Component - Inventory Panel
#[derive(Component, Debug)]
struct InventoryPanel {
    grid: Entity,
    slots: Vec<Entity>,
    selected_slot: usize,
}

// UI Component - Dialogue Box
#[derive(Component, Debug)]
struct DialogueBox {
    text_entity: Entity,
    speaker_entity: Entity,
    active: bool,
}

// Input Map - Keyboard navigation
#[derive(Resource, Default)]
struct InputMap {
    tab: bool,
    shift_tab: bool,
    enter: bool,
    escape: bool,
    arrow_up: bool,
    arrow_down: bool,
    arrow_left: bool,
    arrow_right: bool,
}

// UI Event - ECS event-driven updates
#[derive(Event, Clone, Debug)]
enum UiEvent {
    FocusChanged(Entity),
    ThemeChanged(UiTheme),
    AccessibilityToggle(String),
    StateUpdated(String, u32),
}

// System: Update UI component state
fn update_ui_component_system(
    mut ui_state: ResMut<PlayerUiState>,
    input: Res<InputMap>,
    mut commands: Commands,
    mut events: Events<UiEvent>,
) {
    // Keyboard navigation - focus management
    if input.tab {
        let next_focus = ui_state.next_focus_entity();
        ui_state.focus_entity = Some(next_focus);
        events.send(UiEvent::FocusChanged(next_focus));
    }

    // Theme toggle - high contrast accessibility
    if input.enter {
        ui_state.theme = match ui_state.theme {
            UiTheme::Default => UiTheme::HighContrast,
            UiTheme::HighContrast => UiTheme::Default,
            _ => UiTheme::Default,
        };
        events.send(UiEvent::ThemeChanged(ui_state.theme.clone()));
    }

    // State updates with validation
    if ui_state.health > 100 {
        ui_state.health = 100; // Cap validation
        events.send(UiEvent::StateUpdated("health capped", 100));
    }
}

// System: Accessibility system
fn accessibility_system(
    ui_state: Res<PlayerUiState>,
    mut commands: Commands,
    mut events: Events<UiEvent>,
) {
    // Screen reader toggle
    if ui_state.accessibility.screen_reader {
        // Emit screen reader event
        events.send(UiEvent::AccessibilityToggle("screen_reader"));
    }

    // High contrast mode
    if ui_state.accessibility.high_contrast {
        ui_state.theme = UiTheme::HighContrast;
    }

    // Focus indicators
    if ui_state.accessibility.focus_indicators {
        if let Some(focus) = ui_state.focus_entity {
            commands.entity(focus).insert(FocusIndicator);
        }
    }
}

// Focus Indicator Component
#[derive(Component, Debug)]
struct FocusIndicator;

// System: Build player HUD
fn build_player_hud(mut commands: Commands, ui_state: Res<PlayerUiState>) {
    // Spawn HUD root
    let hud = commands.spawn((
        NodeBundle {
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Vertical,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
            ..default()
        },
        PlayerHud {
            health_bar: Entity::PLACEHOLDER,
            mana_bar: Entity::PLACEHOLDER,
            xp_bar: Entity::PLACEHOLDER,
            level_text: Entity::PLACEHOLDER,
        },
        Name::new("Player HUD"),
    ));

    // Health bar
    let health = commands.spawn((
        TextBundle::from_section(
            format!("HP: {}", ui_state.health),
            TextStyle {
                font_size: 24.0,
                color: Color::srgb(1.0, 0.3, 0.3),
                ..default()
            },
        ),
        Name::new("Health Bar"),
    ));

    // Mana bar
    let mana = commands.spawn((
        TextBundle::from_section(
            format!("MP: {}", ui_state.mana),
            TextStyle {
                font_size: 24.0,
                color: Color::srgb(0.3, 0.6, 1.0),
                ..default()
            },
        ),
        Name::new("Mana Bar"),
    ));

    // XP bar
    let xp = commands.spawn((
        TextBundle::from_section(
            format!("XP: {}", ui_state.experience),
            TextStyle {
                font_size: 24.0,
                color: Color::srgb(1.0, 0.8, 0.2),
                ..default()
            },
        ),
        Name::new("XP Bar"),
    ));

    // Level text with ARIA-like label
    let level = commands.spawn((
        TextBundle::from_section(
            format!("Level: {}", ui_state.level),
            TextStyle {
                font_size: 28.0,
                color: Color::WHITE,
                ..default()
            },
        ),
        Name::new("Level Text"),
        aria_label("Player level indicator"),
    ));

    commands.entity(hud).add_children(&[health, mana, xp, level]);
}

// System: Build inventory panel
fn build_inventory_panel(mut commands: Commands) {
    let inventory = commands.spawn((
        NodeBundle {
            style: Style {
                display: Display::Grid,
                grid_template_rows: vec![Val::Px(50.0); 4],
                grid_template_columns: vec![Val::Px(50.0); 4],
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9)),
            visibility: Visibility::Hidden,
            ..default()
        },
        InventoryPanel {
            grid: Entity::PLACEHOLDER,
            slots: Vec::new(),
            selected_slot: 0,
        },
        Name::new("Inventory Panel"),
    ));

    // Spawn 16 inventory slots
    let mut slots = Vec::new();
    for i in 0..16 {
        let slot = commands.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(50.0),
                    height: Val::Px(50.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                border_color: BorderColor(Color::srgba(0.5, 0.5, 0.5, 1.0)),
                ..default()
            },
            Name::new(format!("Inventory Slot {}", i)),
        ));
        slots.push(slot);
    }

    commands.entity(inventory).add_children(&slots);
    commands.entity(inventory).set_slots(slots);
}

// System: Build dialogue box
fn build_dialogue_box(mut commands: Commands) {
    let dialogue = commands.spawn((
        NodeBundle {
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Vertical,
                padding: UiRect::all(Val::Px(20.0)),
                min_width: Val::Percent(50.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.95)),
            visibility: Visibility::Hidden,
            ..default()
        },
        DialogueBox {
            text_entity: Entity::PLACEHOLDER,
            speaker_entity: Entity::PLACEHOLDER,
            active: false,
        },
        Name::new("Dialogue Box"),
    ));

    // Speaker name
    let speaker = commands.spawn((
        TextBundle::from_section(
            "Speaker",
            TextStyle {
                font_size: 20.0,
                color: Color::srgb(0.8, 0.6, 0.2),
                ..default()
            },
        ),
        Name::new("Speaker Name"),
    ));

    // Dialogue text with ARIA label
    let text = commands.spawn((
        TextBundle::from_section(
            "Dialogue text goes here...",
            TextStyle {
                font_size: 18.0,
                color: Color::WHITE,
                ..default()
            },
        ),
        Name::new("Dialogue Text"),
        aria_label("Dialogue text for screen readers"),
    ));

    commands.entity(dialogue).add_children(&[speaker, text]);
}

// ARIA label component for accessibility
#[derive(Component, Debug)]
struct aria_label(pub String);

// Trait implementation for UI state navigation
impl PlayerUiState {
    fn next_focus_entity(&self) -> Entity {
        // Circular focus navigation
        Entity::from_bits((self.focus_entity.map_or(0, |e| e.to_bits()) + 1) % 1000)
    }
}

// Zero-copy UI state transfer demonstration
fn zero_copy_demo(
    ui_state: Res<PlayerUiState>,
    mut zero_copy: ResMut<ZeroCopyUiState>,
) {
    // Direct memory transfer - no allocation
    zero_copy.copy_from(&ui_state);
}