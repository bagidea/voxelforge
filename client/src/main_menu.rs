//! Main menu — the game entrance.
//!
//! A plain launch (`voxelforge.exe`, no flags) boots to [`AppState::MainMenu`] with
//! the play scene already pre-loaded behind it (see `Cfg::menu` in main.rs), so
//! **New Game** / **Continue** drop straight into the world instead of loading.
//!
//! * **New Game** — wipes saves, re-seeds a fresh quest journal, resets the player
//!   to the campsite, enters Play.
//! * **Continue** — applies `savegame.json` (position/heading/HP/stamina; quest
//!   progress auto-loads through `quest::try_load_saved_journal` at boot), enters
//!   Play. Disabled while no save exists.
//! * **Settings** — opens the existing [`SettingsMenuState`] overlay (`settings_menu.rs`).
//! * **Quit** — exits the app.
//!
//! The buttons, keyboard nav (↑/↓ + Enter) and the scripted proof all route through
//! one [`MenuAction`] message, so the demo exercises the same code path a click does.
//!
//! The visual palette matches `_main_menu_mockup.py` (espresso panel / cream text /
//! amber accent).

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

use crate::combat::{Health, Stamina};
use crate::editor::AppState;
use crate::inventory::Inventory;
use crate::quest::QuestJournal;
use crate::save_game;
use crate::scene::Campsite;
use crate::settings_menu::{settings_closed, SettingsMenuState};
use crate::{Cfg, FlyCam, OrbitCam};

// ---------------------------------------------------------------------------
// Palette (from _main_menu_mockup.py)
// ---------------------------------------------------------------------------

const PANEL_ESPRESSO: egui::Color32 = egui::Color32::from_rgb(26, 18, 13);
const BORDER: egui::Color32 = egui::Color32::from_rgb(58, 39, 22);
const TEXT_CREAM: egui::Color32 = egui::Color32::from_rgb(232, 216, 184);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(178, 160, 138);
const ACCENT_AMBER: egui::Color32 = egui::Color32::from_rgb(244, 184, 96);
const SCREEN_DIM: egui::Color32 = egui::Color32::from_rgb(12, 9, 7);

// ---------------------------------------------------------------------------
// Selection + action
// ---------------------------------------------------------------------------

/// Which menu row the keyboard has highlighted (and which row Enter activates).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum MenuSelection {
    NewGame,
    Continue,
    Settings,
    Quit,
}

impl Default for MenuSelection {
    fn default() -> Self {
        Self::NewGame
    }
}

impl MenuSelection {
    fn prev(self) -> Self {
        match self {
            Self::NewGame => Self::Quit,
            Self::Continue => Self::NewGame,
            Self::Settings => Self::Continue,
            Self::Quit => Self::Settings,
        }
    }
    fn next(self) -> Self {
        match self {
            Self::NewGame => Self::Continue,
            Self::Continue => Self::Settings,
            Self::Settings => Self::Quit,
            Self::Quit => Self::NewGame,
        }
    }
    fn action(self) -> MenuAction {
        match self {
            Self::NewGame => MenuAction::NewGame,
            Self::Continue => MenuAction::Continue,
            Self::Settings => MenuAction::Settings,
            Self::Quit => MenuAction::Quit,
        }
    }
}

/// What a menu interaction asks the app to do. Written by the UI (click), the
/// keyboard nav (Enter) and the scripted proof; read by [`handle_menu_action`].
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    NewGame,
    Continue,
    Settings,
    Quit,
}

/// Run condition: the main menu is NOT on screen (used by main.rs to hold off
/// `fly_camera`, `hud`, `highlight_target` etc. behind the menu).
pub fn main_menu_closed(state: Res<State<AppState>>) -> bool {
    !matches!(state.get(), AppState::MainMenu)
}

/// Run condition: the main menu is showing.
pub fn in_main_menu(state: Res<State<AppState>>) -> bool {
    matches!(state.get(), AppState::MainMenu)
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        // Same guard as settings_menu::SettingsPlugin — self-sufficient so the menu
        // works regardless of plugin order (SettingsPlugin adds it too, idempotently).
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
        }
        app.init_resource::<MenuSelection>()
            .add_message::<MenuAction>()
            .add_systems(OnEnter(AppState::MainMenu), on_enter_main_menu)
            .add_systems(
                EguiPrimaryContextPass,
                menu_ui.run_if(in_main_menu).run_if(settings_closed),
            )
            .add_systems(
                Update,
                (
                    menu_keys.run_if(in_main_menu).run_if(settings_closed),
                    handle_menu_action.run_if(in_main_menu),
                    save_demo.run_if(save_demo_active),
                    menushot.run_if(menushot_active),
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                demo_state_overlay
                    .run_if(save_demo_active)
                    .run_if(in_state(AppState::Play)),
            );
    }
}

/// The cursor starts free + visible on the menu (and again if we ever return to it).
fn on_enter_main_menu(mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut c) = cursors.single_mut() {
        c.grab_mode = CursorGrabMode::None;
        c.visible = true;
    }
}

/// Warm walnut/espresso widget palette, scoped to this one `Ui` so it never leaks
/// into the settings window or dialogue box (same pattern as `settings_menu::theme`).
fn apply_menu_theme(ui: &mut egui::Ui) {
    let v = &mut ui.style_mut().visuals;
    v.override_text_color = Some(TEXT_CREAM);
    v.widgets.noninteractive.bg_fill = PANEL_ESPRESSO;
    v.widgets.noninteractive.fg_stroke.color = TEXT_CREAM;
    v.widgets.inactive.bg_fill = PANEL_ESPRESSO;
    v.widgets.inactive.weak_bg_fill = PANEL_ESPRESSO;
    v.widgets.inactive.fg_stroke.color = TEXT_CREAM;
    v.widgets.hovered.bg_fill = BORDER;
    v.widgets.hovered.weak_bg_fill = BORDER;
    v.widgets.hovered.fg_stroke.color = TEXT_CREAM;
    v.widgets.active.bg_fill = ACCENT_AMBER;
    v.widgets.active.weak_bg_fill = ACCENT_AMBER;
    v.widgets.active.fg_stroke.color = PANEL_ESPRESSO;
    v.selection.bg_fill = ACCENT_AMBER;
    v.selection.stroke.color = PANEL_ESPRESSO;
}

/// The menu itself: a dimmed screen over the pre-loaded play scene, a centered
/// panel with the title and the four rows. Row clicks (and the keyboard selection)
/// both emit the corresponding [`MenuAction`].
fn menu_ui(
    mut contexts: EguiContexts,
    mut selection: ResMut<MenuSelection>,
    mut writer: MessageWriter<MenuAction>,
) {
    // `ctx_mut()` is fallible in bevy_egui 0.41 (the primary context is a queried
    // entity) — bail the frame rather than panic, same as `settings_menu::settings_ui`.
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let can_continue = save_game::save_exists();
    // Slot data shown under Continue once a save exists — this is the "slot data"
    // the after shot has to prove is live (pos + hp straight from `savegame.json`).
    let slot = if can_continue {
        save_game::read_save()
    } else {
        None
    };

    // The screen dim. egui 0.35's `CentralPanel::show` takes a `&mut Ui`, not a
    // `&Context` — there is no context-level full-screen panel any more — so the
    // dim is painted straight onto the background layer and the menu itself is an
    // `Area` on top of it. Same two visual elements, one layer lower.
    // `Context::screen_rect` is gone in egui 0.35; `content_rect` is the whole
    // drawable area of the viewport, which is what the dim + the centering want.
    let screen = ctx.content_rect();
    ctx.layer_painter(egui::LayerId::background())
        .rect_filled(screen, 0.0, SCREEN_DIM);

    let panel_w = 380.0;
    let panel_h = 470.0;
    let rect = egui::Rect::from_min_size(
        egui::pos2(
            (screen.width() - panel_w) * 0.5,
            (screen.height() - panel_h) * 0.5,
        ),
        egui::vec2(panel_w, panel_h),
    );

    egui::Area::new(egui::Id::new("voxelforge_main_menu"))
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            apply_menu_theme(ui);
            ui.set_width(panel_w);
            {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("VOXELFORGE")
                            .size(46.0)
                            .color(TEXT_CREAM)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("A world built one block at a time")
                            .size(14.0)
                            .color(TEXT_DIM),
                    );
                });
                ui.add_space(30.0);

                let rows = [
                    ("New Game", MenuSelection::NewGame, true),
                    ("Continue", MenuSelection::Continue, can_continue),
                    ("Settings", MenuSelection::Settings, true),
                    ("Quit", MenuSelection::Quit, true),
                ];
                for (label, sel, enabled) in rows {
                    let text = if enabled {
                        egui::RichText::new(label).size(20.0).color(TEXT_CREAM)
                    } else {
                        egui::RichText::new(label).size(20.0).color(TEXT_DIM)
                    };
                    let active = *selection == sel;
                    let resp = ui.add_sized(
                        [panel_w - 48.0, 48.0],
                        // `SelectableLabel` is gone in egui 0.35 — `Button::selectable`
                        // is the drop-in with the same selected/unselected fills.
                        egui::Button::selectable(active, text),
                    );
                    if enabled && resp.clicked() {
                        *selection = sel;
                        writer.write(sel.action());
                    }
                    ui.add_space(10.0);
                }

                if let Some(s) = slot {
                    ui.add_space(4.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Saved  pos ({:.0},{:.0},{:.0})  hp {:.0}",
                                s.position[0], s.position[1], s.position[2], s.hp
                            ))
                            .size(12.0)
                            .color(TEXT_DIM),
                        );
                    });
                }

                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("↑ / ↓ to move  ·  Enter to choose")
                            .size(12.0)
                            .color(TEXT_DIM),
                    );
                });
            }
        });
}

/// Keyboard navigation: ↑/↓ (and W/S) move the selection; Enter/Space activate it.
fn menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<MenuSelection>,
    mut writer: MessageWriter<MenuAction>,
) {
    if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
        *selection = selection.prev();
    }
    if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
        *selection = selection.next();
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        // "Continue" only fires when a save actually exists (the row is disabled
        // otherwise, so the keyboard path must match the click path).
        if *selection == MenuSelection::Continue && !save_game::save_exists() {
            return;
        }
        writer.write(selection.action());
    }
}

/// Execute a menu action. New Game / Continue write through `save_game` and flip to
/// Play; Settings opens the existing overlay; Quit exits.
fn handle_menu_action(
    mut reader: MessageReader<MenuAction>,
    mut next: ResMut<NextState<AppState>>,
    mut journal: ResMut<QuestJournal>,
    mut settings: ResMut<SettingsMenuState>,
    mut exit: MessageWriter<AppExit>,
    mut player: Query<
        (&mut Transform, &mut FlyCam, &mut Health, &mut Stamina),
        Without<OrbitCam>,
    >,
    mut cam: Query<(&mut Transform, &mut OrbitCam)>,
    mut inv: ResMut<Inventory>,
    camp: Option<Res<Campsite>>,
) {
    for action in reader.read() {
        match action {
            MenuAction::NewGame => {
                save_game::start_new_game(
                    &mut player,
                    &mut cam,
                    camp.as_deref(),
                    &mut journal,
                    &mut inv,
                );
                next.set(AppState::Play);
            }
            MenuAction::Continue => {
                if save_game::continue_game(&mut player, &mut cam, &mut journal, &mut inv) {
                    next.set(AppState::Play);
                }
            }
            MenuAction::Settings => {
                settings.open = true;
            }
            MenuAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Scripted proof — the "real play" the definition of done asks for, captured.
//
// Two runs of the same binary drive the real menu action path + the real
// `fly_camera` walk + the real save file, shooting in-engine PNGs at each step:
//
//   VOXELFORGE_SAVE_DEMO=save      → menu → New Game → walk → save → exit
//   VOXELFORGE_SAVE_DEMO=continue  → menu → Continue → back at the saved spot
//
// The saved position printed by the `save` run and the restored position printed by
// the `continue` run must match, and the `walked.png` / `continue.png` frames must
// show the same place. A human does the same thing with the mouse + F6.
// ---------------------------------------------------------------------------

fn save_demo_active(cfg: Res<Cfg>) -> bool {
    cfg.save_demo.is_some()
}

fn shoot(commands: &mut Commands, path: &str) {
    // `save_to_disk` won't create the parent folder; make it so the PNGs always land.
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.to_string()));
    println!("DEMO_SHOT {path}");
}

fn save_demo(
    time: Res<Time>,
    cfg: Res<Cfg>,
    mut commands: Commands,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<MenuAction>,
    player: Query<(&Transform, &FlyCam, &Health, &Stamina), Without<OrbitCam>>,
    mut journal: ResMut<QuestJournal>,
    mut last: ResMut<save_game::LastSaveSnapshot>,
    inventory: Res<Inventory>,
    mut exit: MessageWriter<AppExit>,
    mut phase: Local<u8>,
) {
    let Some(mode) = cfg.save_demo.as_deref() else {
        return;
    };
    let t = time.elapsed_secs();

    if mode == "save" {
        match *phase {
            0 if t > 0.7 => {
                shoot(&mut commands, "menu.png");
                *phase = 1;
            }
            1 if t > 1.0 => {
                writer.write(MenuAction::NewGame);
                println!("DEMO_ACTION NewGame");
                *phase = 2;
            }
            2 if t > 2.0 => {
                keys.press(KeyCode::KeyW);
                println!("DEMO_WALK start");
                *phase = 3;
            }
            3 if t > 3.5 => {
                keys.release(KeyCode::KeyW);
                shoot(&mut commands, "docs/assets/menu/walk.png");
                println!("DEMO_WALK end");
                *phase = 4;
            }
            4 if t > 4.0 => {
                // The "picked something up" beat: plant a quest flag so the save
                // round-trips quest state too. The inventory rides the same
                // `write_save` (empty here — the scripted proof doesn't dig); its
                // own round-trip is covered by the inventory.rs test suite.
                journal.flags.insert("demo_kevin_item".to_string());
                match save_game::write_save(&player, &journal, &mut last, &inventory) {
                    Ok(s) => println!(
                        "DEMO_SAVE pos=({:.2},{:.2},{:.2}) yaw={:.2} hp={:.0} stamina={:.0}",
                        s.position[0], s.position[1], s.position[2], s.yaw, s.hp, s.stamina
                    ),
                    Err(e) => eprintln!("DEMO_SAVE FAIL {e}"),
                }
                *phase = 5;
            }
            5 if t > 5.2 => {
                shoot(&mut commands, "docs/assets/menu/save-before-load.png");
                *phase = 6;
            }
            6 if t > 6.6 => {
                println!("DEMO_DONE save");
                exit.write(AppExit::Success);
                *phase = 7;
            }
            _ => {}
        }
    } else if mode == "continue" {
        match *phase {
            0 if t > 0.7 => {
                shoot(&mut commands, "menu-continue.png");
                *phase = 1;
            }
            1 if t > 1.0 => {
                writer.write(MenuAction::Continue);
                println!("DEMO_ACTION Continue");
                *phase = 2;
            }
            2 if t > 2.5 => {
                shoot(&mut commands, "docs/assets/menu/save-after-load.png");
                *phase = 3;
            }
            3 if t > 4.0 => {
                println!("DEMO_DONE continue");
                exit.write(AppExit::Success);
                *phase = 4;
            }
            _ => {}
        }
    }
}

/// Paints the session values (position / HP / stamina / quest flags) into the
/// top-left corner during the demo, so the captured frames prove the save/load
/// round-trip with numbers readable *from the image*, not just the log. Prefers
/// the exact values `write_save` snapshotted (the bytes on disk) so the "before
/// load" frame matches what Continue restores; falls back to the live session
/// before any save exists.
fn demo_state_overlay(
    player: Query<(&Transform, &Health, &Stamina), Without<OrbitCam>>,
    journal: Res<QuestJournal>,
    last: Res<save_game::LastSaveSnapshot>,
    mut contexts: EguiContexts,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let (pos, hp, stamina) = if let Some(s) = &last.save {
        (
            format!(
                "{:.1},{:.1},{:.1}",
                s.position[0], s.position[1], s.position[2]
            ),
            format!("{:.0}", s.hp),
            format!("{:.0}", s.stamina),
        )
    } else if let Ok((tf, hp, stam)) = player.single() {
        (
            format!(
                "{:.1},{:.1},{:.1}",
                tf.translation.x, tf.translation.y, tf.translation.z
            ),
            format!("{:.0}", hp.cur),
            format!("{:.0}", stam.cur),
        )
    } else {
        return;
    };

    let flags: Vec<String> = if last.save.is_some() {
        last.flags.clone()
    } else {
        let mut f: Vec<String> = journal.flags.iter().cloned().collect();
        f.sort();
        f
    };
    let flags = if flags.is_empty() {
        "-".to_string()
    } else {
        flags.join(",")
    };

    let text = format!("pos=({pos})  hp={hp}  stamina={stamina}  quest_flags={flags}");

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("demo_state_overlay"),
    ));
    painter.rect_filled(
        egui::Rect::from_min_size(egui::pos2(8.0, 8.0), egui::vec2(880.0, 34.0)),
        4.0,
        egui::Color32::from_black_alpha(190),
    );
    painter.text(
        egui::pos2(14.0, 14.0),
        egui::Align2::LEFT_TOP,
        text,
        egui::FontId::monospace(18.0),
        egui::Color32::WHITE,
    );
}

// ---------------------------------------------------------------------------
// Headless menu capture — `VOXELFORGE_MENUSHOT=before|after`.
//
// `before` is the plain-entrance truth: no save on disk, so the Continue row is
// dimmed. `after` runs once a save exists, so Continue is lit and the slot line
// shows the saved session. Both boot to MainMenu and exit after the frame lands.
// ---------------------------------------------------------------------------

fn menushot_active(cfg: Res<Cfg>) -> bool {
    cfg.menushot.is_some()
}

fn menushot(
    time: Res<Time>,
    cfg: Res<Cfg>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
    mut phase: Local<u8>,
) {
    let Some(mode) = cfg.menushot.as_deref() else {
        return;
    };
    let t = time.elapsed_secs();
    match *phase {
        0 if t > 0.9 => {
            let path = match mode {
                "before" => "docs/assets/menu/menu-a-before.png",
                "after" => "docs/assets/menu/menu-b-after.png",
                other => {
                    eprintln!("MENUSHOT unknown mode {other:?} (want before|after)");
                    "docs/assets/menu/menu-unknown.png"
                }
            };
            shoot(&mut commands, path);
            println!("MENUSHOT {mode} -> {path}");
            *phase = 1;
        }
        1 if t > 2.6 => {
            println!("MENUSHOT_DONE {mode}");
            exit.write(AppExit::Success);
            *phase = 2;
        }
        _ => {}
    }
}

