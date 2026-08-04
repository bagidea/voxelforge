//! Steam Settings menu — egui overlay for Graphics / Display / Audio / Keybinds.
//!
//! Lives in `EguiPrimaryContextPass` so egui is initialised before we draw. ESC
//! toggles the menu in both Editor and Play; while the menu is open the game
//! pauses input capture and the cursor is freed.
//!
//! Persisted to `settings.json` using the same serde_json pattern as
//! `editor_config.rs`.

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, Window, WindowMode};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use serde::{Deserialize, Serialize};

use crate::audio::{AmbientEnts, AudioSettings};
use crate::editor::AppState;
use crate::editor_config::input_map::{EditorAction, Key, KeyBindings};
use crate::look::LookQuality;

// =============================================================================
// Config file format
// =============================================================================

const SETTINGS_PATH: &str = "settings.json";

/// Everything the Settings menu edits and persists.
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameSettings {
    /// Look-quality tier — mutating this re-applies the post stack next frame.
    pub graphics: LookQuality,
    /// Window mode + resolution.
    pub display: DisplaySettings,
    /// Master / SFX / ambient(=music) / UI volumes.
    pub audio: AudioSettings,
    /// Editor/play keybindings.
    pub keybindings: KeyBindings,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            graphics: LookQuality::default(),
            display: DisplaySettings::default(),
            audio: AudioSettings::default(),
            keybindings: KeyBindings::default(),
        }
    }
}

/// Window configuration the player can change from the menu.
#[derive(Resource, Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DisplaySettings {
    /// Fullscreen, borderless, or windowed.
    pub mode: WindowModeSetting,
    /// Logical window resolution in pixels.
    pub resolution: [u32; 2],
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            mode: WindowModeSetting::Windowed,
            resolution: [1280, 720],
        }
    }
}

/// A serialisable, simplified window mode. Bevy's `WindowMode` carries monitor
/// handles that do not round-trip through JSON, so we store this and map on apply.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum WindowModeSetting {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}

// =============================================================================
// Runtime menu state
// =============================================================================

/// Whether the settings menu is open and which tab is active.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SettingsMenuState {
    pub open: bool,
    pub tab: SettingsTab,
    /// When `Some(action)` the menu is waiting for the next physical key press
    /// to bind to that action.
    pub pending_rebind: Option<EditorAction>,
    /// True for exactly one frame after the user clicks "Bind". Prevents the
    /// same mouse-down event that clicked the button from being captured as the
    /// new binding.
    pub rebind_ignore_frame: bool,
}

/// System set for everything that handles settings-menu input. Editor state
/// transitions (e.g. `exit_play`) are ordered after this set so Escape is
/// consumed by the menu before the gameplay state machine can act on it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SettingsMenuSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    Graphics,
    Display,
    Audio,
    Keybinds,
}

// =============================================================================
// Plugin
// =============================================================================

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
        }

        let settings = load_settings();
        // Push loaded settings into the live resources the game already reads.
        // AudioPlugin and LookPlugin may have inserted their own defaults first;
        // these calls override them with the player's saved preferences.  The
        // standalone `KeyBindings` resource is what editor/gameplay systems query
        // through `action_just_pressed`, so we overwrite InputConfigPlugin's copy
        // with the settings-menu copy.
        app.insert_resource(settings.audio.clone())
            .insert_resource(settings.graphics)
            .insert_resource(settings.keybindings.clone())
            .insert_resource(settings)
            .init_resource::<SettingsMenuState>()
            .configure_sets(Update, SettingsMenuSet)
            .add_systems(
                Update,
                (settings_toggle, capture_rebind, apply_display_settings)
                    .chain()
                    .in_set(SettingsMenuSet),
            )
            .add_systems(Update, update_ambient_volumes.after(crate::audio::update_volumes))
            .add_systems(EguiPrimaryContextPass, settings_ui);
    }
}

// =============================================================================
// Voxelforge theme — warm walnut/espresso palette shared with the combat HUD
// spec (see docs/hud-design.md), instead of egui's default cool-grey theme.
// =============================================================================

mod theme {
    use bevy_egui::egui::{self, Color32, Stroke};

    pub const PANEL_BG: Color32 = Color32::from_rgb(30, 20, 15);
    pub const BORDER: Color32 = Color32::from_rgb(58, 39, 22);
    pub const WIDGET_IDLE: Color32 = Color32::from_rgb(58, 39, 22);
    pub const WIDGET_HOVER: Color32 = Color32::from_rgb(107, 74, 46);
    pub const WIDGET_ACTIVE: Color32 = Color32::from_rgb(200, 138, 74);
    pub const ACCENT_AMBER: Color32 = Color32::from_rgb(244, 184, 96);
    pub const TEXT_CREAM: Color32 = Color32::from_rgb(232, 216, 184);

    /// Frame for the outer `egui::Window` chrome. Set via `.frame(...)` on the
    /// `Window` builder so it never touches the shared context-level `Style`
    /// that the dialogue box and editor panels also draw with.
    pub fn window_frame(ctx: &egui::Context) -> egui::Frame {
        egui::Frame::window(&ctx.style_of(ctx.theme()))
            .fill(PANEL_BG)
            .stroke(Stroke::new(1.0, BORDER))
    }

    /// Warm widget palette, scoped to one `Ui` (and its children) via
    /// `ui.style_mut()` — this does NOT leak into sibling egui surfaces drawn
    /// later in the same `EguiPrimaryContextPass` frame, e.g. `dialogue_ui.rs`
    /// or `editor_ui.rs`, which is why this lives here rather than as a
    /// context-wide `ctx.set_visuals(..)` call.
    pub fn apply(ui: &mut egui::Ui) {
        let v = &mut ui.style_mut().visuals;
        v.override_text_color = Some(TEXT_CREAM);
        v.hyperlink_color = ACCENT_AMBER;
        v.selection.bg_fill = ACCENT_AMBER;
        v.selection.stroke.color = PANEL_BG;

        v.widgets.noninteractive.bg_fill = PANEL_BG;
        v.widgets.noninteractive.fg_stroke.color = TEXT_CREAM;

        v.widgets.inactive.bg_fill = WIDGET_IDLE;
        v.widgets.inactive.weak_bg_fill = WIDGET_IDLE;
        v.widgets.inactive.fg_stroke.color = TEXT_CREAM;

        v.widgets.hovered.bg_fill = WIDGET_HOVER;
        v.widgets.hovered.weak_bg_fill = WIDGET_HOVER;
        v.widgets.hovered.fg_stroke.color = TEXT_CREAM;

        v.widgets.active.bg_fill = WIDGET_ACTIVE;
        v.widgets.active.weak_bg_fill = WIDGET_ACTIVE;
        v.widgets.active.fg_stroke.color = PANEL_BG;
    }
}

// =============================================================================
// Load / save
// =============================================================================

fn load_settings() -> GameSettings {
    match std::fs::read_to_string(SETTINGS_PATH) {
        Ok(text) => match serde_json::from_str::<GameSettings>(&text) {
            Ok(cfg) => {
                println!("SETTINGS loaded {SETTINGS_PATH}");
                cfg
            }
            Err(e) => {
                eprintln!("SETTINGS parse error (using defaults): {e}");
                GameSettings::default()
            }
        },
        Err(_) => {
            let cfg = GameSettings::default();
            let _ = save_settings_file(&cfg);
            cfg
        }
    }
}

fn save_settings_file(settings: &GameSettings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(SETTINGS_PATH, &json).map_err(|e| e.to_string())?;
    println!("SETTINGS saved {SETTINGS_PATH}");
    Ok(())
}

/// Public save hook. Call after mutating `GameSettings` (or `KeyBindings` through
/// it) to persist the change.
pub fn save_settings(settings: &GameSettings) {
    if let Err(e) = save_settings_file(settings) {
        eprintln!("SETTINGS save failed: {e}");
    }
}

/// Run condition: the settings menu is closed. Use to keep gameplay systems
/// (camera lock, combat input) from firing while the menu is open.
pub fn settings_closed(menu: Res<SettingsMenuState>) -> bool {
    !menu.open
}

// =============================================================================
// Systems
// =============================================================================

/// ESC toggles the settings menu. When opening, free the cursor; when closing,
/// lock it again while in Play so the camera works immediately.
fn settings_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut menu: ResMut<SettingsMenuState>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    // While waiting for a rebind, Escape cancels the bind instead of toggling
    // the menu — `capture_rebind` handles it.
    if menu.pending_rebind.is_some() {
        return;
    }
    menu.open = !menu.open;
    menu.pending_rebind = None;

    let Ok(mut cursor) = cursors.single_mut() else {
        return;
    };
    if menu.open {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else if *state.get() == AppState::Play {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Capture the next physical key or mouse button for a pending rebind.
fn capture_rebind(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut menu: ResMut<SettingsMenuState>,
    mut settings: ResMut<GameSettings>,
    mut bindings: ResMut<KeyBindings>,
) {
    let Some(action) = menu.pending_rebind else { return };

    // Cancel on Escape so the menu doesn't close while binding.
    if keys.just_pressed(KeyCode::Escape) {
        menu.pending_rebind = None;
        menu.rebind_ignore_frame = false;
        return;
    }

    // Ignore the same frame that entered bind mode so the click on the Bind
    // button is not captured as the new key.
    if menu.rebind_ignore_frame {
        menu.rebind_ignore_frame = false;
        return;
    }

    let mut bound = None;
    for k in keys.get_just_pressed() {
        if let Some(key) = key_from_bevy(*k) {
            bound = Some(key);
            break;
        }
    }
    if bound.is_none() {
        for mb in mouse.get_just_pressed() {
            bound = Some(match mb {
                MouseButton::Left => Key::MouseLeft,
                MouseButton::Right => Key::MouseRight,
                MouseButton::Middle => Key::MouseMiddle,
                _ => continue,
            });
            break;
        }
    }

    if let Some(key) = bound {
        settings.keybindings.rebind(action, key);
        bindings.rebind(action, key);
        menu.pending_rebind = None;
        save_settings(&settings);
    }
}

/// Apply display settings to the primary window when they differ from the live
/// window state.
fn apply_display_settings(
    settings: Res<GameSettings>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !settings.is_changed() {
        return;
    }
    let Ok(mut window) = windows.single_mut() else { return };

    let target_mode = match settings.display.mode {
        WindowModeSetting::Windowed => WindowMode::Windowed,
        WindowModeSetting::Borderless => WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current),
        WindowModeSetting::Fullscreen => WindowMode::Fullscreen(
            bevy::window::MonitorSelection::Current,
            bevy::window::VideoModeSelection::Current,
        ),
    };

    if window.mode != target_mode {
        window.mode = target_mode;
    }

    let [w, h] = settings.display.resolution;
    if (window.resolution.width() as u32) != w || (window.resolution.height() as u32) != h {
        window.resolution.set(w as f32, h as f32);
    }
}

/// Keep the three ambient loop volumes in sync with the live master/ambient bus.
fn update_ambient_volumes(
    settings: Res<AudioSettings>,
    ambient: Res<AmbientEnts>,
    mut playback: Query<&mut bevy::audio::PlaybackSettings>,
) {
    if !settings.is_changed() {
        return;
    }
    let vol = (settings.master * settings.ambient) as f64;
    // Wind 0.7, campfire 0.8, village 0.5 — match the spawn mix in audio.rs.
    let gains = [0.7f32, 0.8, 0.5];
    for (opt, gain) in [ambient.wind, ambient.campfire, ambient.village].into_iter().zip(gains) {
        let Some(e) = opt else { continue };
        let Ok(mut pb) = playback.get_mut(e) else { continue };
        pb.volume = bevy::audio::Volume::Linear((vol * gain as f64) as f32);
    }
}

/// The settings window. Drawn in `EguiPrimaryContextPass` to avoid the
/// "No fonts available until first call to Context::run()" panic.
fn settings_ui(
    mut contexts: EguiContexts,
    mut menu: ResMut<SettingsMenuState>,
    mut settings: ResMut<GameSettings>,
    mut quality: ResMut<LookQuality>,
    mut audio: ResMut<AudioSettings>,
) {
    if !menu.open {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let panel_width = 540.0;
    let panel_height = 420.0;

    egui::Window::new("Settings")
        .collapsible(false)
        .resizable(false)
        .title_bar(true)
        .fixed_size(egui::vec2(panel_width, panel_height))
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .frame(theme::window_frame(ctx))
        .show(ctx, |ui| {
            theme::apply(ui);
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (SettingsTab::Graphics, "Graphics"),
                    (SettingsTab::Display, "Display"),
                    (SettingsTab::Audio, "Audio"),
                    (SettingsTab::Keybinds, "Keybinds"),
                ] {
                    let selected = menu.tab == tab;
                    if ui.selectable_label(selected, label).clicked() {
                        menu.pending_rebind = None;
                        menu.tab = tab;
                    }
                }
            });
            ui.separator();

            match menu.tab {
                SettingsTab::Graphics => graphics_tab(ui, &mut settings, &mut quality),
                SettingsTab::Display => display_tab(ui, &mut settings),
                SettingsTab::Audio => audio_tab(ui, &mut settings, &mut audio),
                SettingsTab::Keybinds => keybinds_tab(ui, &mut menu, &mut settings),
            }

            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    menu.open = false;
                    menu.pending_rebind = None;
                }
            });
        });
}

// =============================================================================
// Tabs
// =============================================================================

fn graphics_tab(ui: &mut egui::Ui, settings: &mut GameSettings, quality: &mut LookQuality) {
    ui.heading("Graphics");
    ui.add_space(8.0);

    let current = *quality;
    let mut selected = current;
    egui::ComboBox::from_label("Quality")
        .selected_text(format!("{:?}", selected))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut selected, LookQuality::Low, "Low");
            ui.selectable_value(&mut selected, LookQuality::Medium, "Medium");
            ui.selectable_value(&mut selected, LookQuality::High, "High");
            ui.selectable_value(&mut selected, LookQuality::Ultra, "Ultra");
        });
    if selected != current {
        *quality = selected;
        settings.graphics = selected;
        save_settings(settings);
    }

    ui.add_space(8.0);
    ui.label("Changes apply to the live camera and sun immediately.");
}

fn display_tab(ui: &mut egui::Ui, settings: &mut GameSettings) {
    ui.heading("Display");
    ui.add_space(8.0);

    let mut mode = settings.display.mode;
    egui::ComboBox::from_label("Window mode")
        .selected_text(format!("{:?}", mode))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut mode, WindowModeSetting::Windowed, "Windowed");
            ui.selectable_value(&mut mode, WindowModeSetting::Borderless, "Borderless");
            ui.selectable_value(&mut mode, WindowModeSetting::Fullscreen, "Fullscreen");
        });
    if mode != settings.display.mode {
        settings.display.mode = mode;
        save_settings(settings);
    }

    let mut width = settings.display.resolution[0] as f32;
    let mut height = settings.display.resolution[1] as f32;
    ui.horizontal(|ui| {
        ui.label("Resolution");
        ui.add(egui::DragValue::new(&mut width).speed(1.0).range(640.0..=7680.0).prefix("W "));
        ui.add(egui::DragValue::new(&mut height).speed(1.0).range(480.0..=4320.0).prefix("H "));
    });
    let new_w = width as u32;
    let new_h = height as u32;
    if new_w != settings.display.resolution[0] || new_h != settings.display.resolution[1] {
        settings.display.resolution = [new_w, new_h];
        save_settings(settings);
    }
}

fn audio_tab(
    ui: &mut egui::Ui,
    settings: &mut GameSettings,
    audio: &mut AudioSettings,
) {
    ui.heading("Audio");
    ui.add_space(8.0);

    fn volume_slider(ui: &mut egui::Ui, label: &str, value: &mut f32) -> bool {
        let before = *value;
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::Slider::new(value, 0.0..=1.0).show_value(true));
        });
        *value != before
    }

    let mut dirty = false;
    dirty |= volume_slider(ui, "Master", &mut audio.master);
    dirty |= volume_slider(ui, "SFX", &mut audio.sfx);
    dirty |= volume_slider(ui, "Music / Ambient", &mut audio.ambient);
    dirty |= volume_slider(ui, "UI", &mut audio.ui);

    if dirty {
        settings.audio = audio.clone();
        save_settings(settings);
    }
}

fn keybinds_tab(
    ui: &mut egui::Ui,
    menu: &mut SettingsMenuState,
    settings: &mut GameSettings,
) {
    ui.heading("Keybinds");
    ui.add_space(8.0);

    if let Some(action) = menu.pending_rebind {
        ui.label(format!("Press a key or mouse button for '{}'…", action.label()));
        ui.label("(Esc to cancel)");
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("keybind_grid")
            .num_columns(3)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                for action in EditorAction::ALL {
                    let key = settings.keybindings.key(*action);
                    ui.label(action.label());
                    ui.label(format!("{:?}", key));
                    if ui.button("Bind").clicked() {
                        menu.pending_rebind = Some(*action);
                        menu.rebind_ignore_frame = true;
                    }
                    ui.end_row();
                }
            });
    });
}

// =============================================================================
// Helpers
// =============================================================================

fn key_from_bevy(kc: KeyCode) -> Option<Key> {
    use bevy::input::keyboard::KeyCode as K;
    use Key::*;
    Some(match kc {
        K::KeyA => KeyA,
        K::KeyB => KeyB,
        K::KeyC => KeyC,
        K::KeyD => KeyD,
        K::KeyE => KeyE,
        K::KeyF => KeyF,
        K::KeyG => KeyG,
        K::KeyH => KeyH,
        K::KeyI => KeyI,
        K::KeyJ => KeyJ,
        K::KeyK => KeyK,
        K::KeyL => KeyL,
        K::KeyM => KeyM,
        K::KeyN => KeyN,
        K::KeyO => KeyO,
        K::KeyP => KeyP,
        K::KeyQ => KeyQ,
        K::KeyR => KeyR,
        K::KeyS => KeyS,
        K::KeyT => KeyT,
        K::KeyU => KeyU,
        K::KeyV => KeyV,
        K::KeyW => KeyW,
        K::KeyX => KeyX,
        K::KeyY => KeyY,
        K::KeyZ => KeyZ,
        K::Digit0 => Digit0,
        K::Digit1 => Digit1,
        K::Digit2 => Digit2,
        K::Digit3 => Digit3,
        K::Digit4 => Digit4,
        K::Digit5 => Digit5,
        K::Digit6 => Digit6,
        K::Digit7 => Digit7,
        K::Digit8 => Digit8,
        K::Digit9 => Digit9,
        K::F1 => F1,
        K::F2 => F2,
        K::F3 => F3,
        K::F4 => F4,
        K::F5 => F5,
        K::F6 => F6,
        K::F7 => F7,
        K::F8 => F8,
        K::F9 => F9,
        K::F10 => F10,
        K::F11 => F11,
        K::F12 => F12,
        K::Space => Space,
        K::Enter => Enter,
        K::Escape => Escape,
        K::Tab => Tab,
        K::Backspace => Backspace,
        K::Delete => Delete,
        K::ShiftLeft => ShiftLeft,
        K::ShiftRight => ShiftRight,
        K::ControlLeft => ControlLeft,
        K::ControlRight => ControlRight,
        K::AltLeft => AltLeft,
        K::AltRight => AltRight,
        K::ArrowUp => ArrowUp,
        K::ArrowDown => ArrowDown,
        K::ArrowLeft => ArrowLeft,
        K::ArrowRight => ArrowRight,
        _ => return None,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_serialises() {
        let s = GameSettings::default();
        let json = serde_json::to_string_pretty(&s).expect("serialize");
        assert!(json.contains("\"graphics\""));
        assert!(json.contains("\"display\""));
        assert!(json.contains("\"audio\""));
        assert!(json.contains("\"keybindings\""));
    }

    #[test]
    fn window_mode_round_trips() {
        for mode in [
            WindowModeSetting::Windowed,
            WindowModeSetting::Borderless,
            WindowModeSetting::Fullscreen,
        ] {
            let json = serde_json::to_string(&mode).expect("serialize");
            let parsed: WindowModeSetting = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(mode, parsed);
        }
    }
}
