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

        let mut settings = load_settings();
        // `VOXELFORGE_LOOK_QUALITY` OUTRANKS THE SAVED FILE — 2026-08-09.
        //
        // It is the capture/profiling override every shoot script exports, and the
        // line below used to stomp it: `insert_resource` "overwrites any existing
        // resource of the same type", and main.rs adds this plugin AFTER LookPlugin,
        // so the saved tier always won. Every `--play` plate — the whole canonical
        // shotset — rendered at settings.json's "High" whatever the script asked for,
        // which is why `grade-vista` "at Ultra" is a PCSS-off frame. See
        // `look::quality_from_env` for the two measurements that pin it.
        //
        // Assigned into `settings` rather than inserted separately so the menu shows
        // the tier that is RUNNING; a player who then edits any setting persists it,
        // which is correct — at that point they are on that tier.
        if let Some(tier) = crate::look::quality_from_env() {
            settings.graphics = tier;
        }
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
    /// Dimmer cream for secondary/help text — section subtext, card
    /// descriptions — so the hierarchy reads without a second hue.
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(178, 160, 138);

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
// Motion — small hex-only helpers for hover/select animation.
//
// `egui::Context::animate_bool_with_time(id, target, duration)` is the one
// egui primitive this relies on: an animated 0..1 that eases toward `target`
// and is safe to call every frame keyed by a stable `Id`. Two shapes:
//   - When a widget's own `Response` already exists this frame (e.g.
//     `ui.selectable_label(..)`), animate straight off `response.hovered()` —
//     zero lag, see the tab underline in `settings_ui`.
//   - When a *custom* widget's fill colour must be decided before its
//     `Response` exists (a hand-built card `Frame`), there's no way around a
//     one-frame-stale hover read — a standard immediate-mode trick, invisible
//     at 1/60s. See `preset_card` below for the read/animate/write sequence.
// =============================================================================

mod motion {
    use bevy_egui::egui::Color32;

    fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
        (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)).round() as u8
    }

    /// Linear per-channel blend between two opaque colours.
    pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
        Color32::from_rgb(lerp_u8(a.r(), b.r(), t), lerp_u8(a.g(), b.g(), t), lerp_u8(a.b(), b.b(), t))
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

    // Fade + settle in on open (motion the CEO asked for isn't only
    // hover/select — the panel itself shouldn't just snap into existence).
    // `menu.open` is already true here, so this eases 0->1 once and holds;
    // there is no fade-*out* leg because `settings_ui` stops running the
    // instant `open` flips false (early return above), which is an
    // acceptable one-frame pop on close — only open/hover/select asked for
    // motion. Target alpha is 248/255, not fully transparent-capable: this is
    // a text- and slider-heavy modal the player paused the game to read, not
    // a combat HUD element, so legibility wins over the HUD's "translucent
    // plaque" rule (hud-design.md §0) — that rule is for elements sitting
    // over live gameplay, which this isn't (input capture pauses while open).
    let open_t = ctx.animate_bool_with_time(egui::Id::new("settings_menu_open"), true, 0.18);
    let fill = egui::Color32::from_rgba_unmultiplied(
        theme::PANEL_BG.r(),
        theme::PANEL_BG.g(),
        theme::PANEL_BG.b(),
        (248.0 * open_t) as u8,
    );
    let frame = theme::window_frame(ctx).fill(fill);
    let settle_offset = egui::vec2(0.0, (1.0 - open_t) * 10.0);

    let panel_width = 580.0;
    let panel_height = 460.0;

    egui::Window::new("Settings")
        .collapsible(false)
        .resizable(false)
        .title_bar(true)
        .fixed_size(egui::vec2(panel_width, panel_height))
        .anchor(egui::Align2::CENTER_CENTER, settle_offset)
        .frame(frame)
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
                    let resp = ui.selectable_label(selected, label);
                    if resp.clicked() {
                        menu.pending_rebind = None;
                        menu.tab = tab;
                    }
                    // Animated amber underline: grows in on hover, snaps full
                    // width when selected, retracts on hover-out. `resp` is
                    // this-frame-fresh (`selectable_label` interacts
                    // synchronously), so this reads current hover with no lag.
                    let ut = ui.ctx().animate_bool_with_time(resp.id, selected || resp.hovered(), 0.15);
                    if ut > 0.0 {
                        let r = resp.rect;
                        let y = r.bottom() + 2.0;
                        let x1 = r.left() + r.width() * ut;
                        ui.painter().line_segment(
                            [egui::pos2(r.left(), y), egui::pos2(x1, y)],
                            egui::Stroke::new(2.0, theme::ACCENT_AMBER),
                        );
                    }
                }
            });
            ui.add_space(4.0);
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

/// Quality tier, card title, and a two-line description of what that tier
/// adds over the one below it. Mirrors the doc comments on
/// [`LookQuality`] and the tier `match` in `look.rs::insert_stack` — that
/// `match` is the single source of truth (rose's lane, read-only from here);
/// if it's retuned, these strings need updating by hand to match.
const QUALITY_PRESETS: [(LookQuality, &str, &str); 4] = [
    (LookQuality::Low, "Low", "Contact AO + soft\nshadows only."),
    (LookQuality::Medium, "Medium", "+ TAA, temporal\nsoft shadows."),
    (LookQuality::High, "High", "+ High AO, god rays\n(32-step). Default."),
    (LookQuality::Ultra, "Ultra", "+ Ultra AO, PCSS,\nfull 96-step rays."),
];

/// One clickable quality-preset card. Returns `true` if it was clicked.
///
/// Colour must be picked *before* the card's own `Response` exists (it's a
/// builder argument to `Frame`), so hover can't be read live off this frame's
/// response the way the tab underline does. Instead: read last frame's hover
/// out of egui's per-id temp memory, animate off that, draw, then stash this
/// frame's real hover back for next frame. One frame of lag on the hover
/// edge, imperceptible at 60fps — a standard immediate-mode trick.
fn preset_card(ui: &mut egui::Ui, width: f32, title: &str, desc: &str, selected: bool) -> bool {
    let id = ui.id().with(("quality_preset", title));
    let last_hovered = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    let t = ui.ctx().animate_bool_with_time(id, selected || last_hovered, 0.15);

    let fill = motion::lerp_color(theme::WIDGET_IDLE, theme::WIDGET_ACTIVE, if selected { 1.0 } else { t });
    let border = motion::lerp_color(theme::BORDER, theme::ACCENT_AMBER, if selected { 1.0 } else { t });
    let text_col = if selected { theme::PANEL_BG } else { theme::TEXT_CREAM };
    let desc_col = if selected { theme::PANEL_BG } else { theme::TEXT_MUTED };

    let inner = egui::Frame::group(ui.style())
        .fill(fill)
        .stroke(egui::Stroke::new(if selected { 2.0 } else { 1.0 }, border))
        .inner_margin(egui::Margin::symmetric(8, 8))
        .show(ui, |ui| {
            ui.set_width((width - 20.0).max(60.0));
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(title).strong().size(15.0).color(text_col));
                ui.add_space(3.0);
                ui.label(egui::RichText::new(desc).size(10.5).color(desc_col));
            });
        });

    let resp = inner.response.interact(egui::Sense::click());
    ui.ctx().data_mut(|d| d.insert_temp(id, resp.hovered()));
    resp.clicked()
}

fn graphics_tab(ui: &mut egui::Ui, settings: &mut GameSettings, quality: &mut LookQuality) {
    ui.heading("Graphics");
    ui.label(egui::RichText::new("Quality preset — retunes the post-processing stack live.").color(theme::TEXT_MUTED));
    ui.add_space(10.0);

    let current = *quality;
    let spacing = 8.0;
    let card_w = (ui.available_width() - spacing * (QUALITY_PRESETS.len() as f32 - 1.0)) / QUALITY_PRESETS.len() as f32;
    let mut picked = current;

    ui.horizontal(|ui| {
        for (i, &(tier, title, desc)) in QUALITY_PRESETS.iter().enumerate() {
            if preset_card(ui, card_w, title, desc, current == tier) {
                picked = tier;
            }
            if i + 1 < QUALITY_PRESETS.len() {
                ui.add_space(spacing);
            }
        }
    });

    if picked != current {
        *quality = picked;
        settings.graphics = picked;
        save_settings(settings);
    }

    ui.add_space(10.0);
    egui::CollapsingHeader::new("Advanced — exact effects per tier")
        .default_open(false)
        .show(ui, |ui| {
            for &(_, title, desc) in QUALITY_PRESETS.iter() {
                ui.label(
                    egui::RichText::new(format!("{title}: {}", desc.replace('\n', " ")))
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                );
            }
        });

    ui.add_space(10.0);
    ui.label("Changes apply to the live camera and sun immediately.");
}

fn display_tab(ui: &mut egui::Ui, settings: &mut GameSettings) {
    ui.heading("Display");
    ui.label(egui::RichText::new("Window mode and resolution.").color(theme::TEXT_MUTED));
    ui.add_space(10.0);

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
    ui.label(egui::RichText::new("Master, SFX, music/ambient, and UI volumes.").color(theme::TEXT_MUTED));
    ui.add_space(10.0);

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
    ui.label(egui::RichText::new("Click Bind, then press a key or mouse button.").color(theme::TEXT_MUTED));
    ui.add_space(10.0);

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
