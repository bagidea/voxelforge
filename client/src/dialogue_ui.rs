//! Voxelforge dialogue UI — a native Bevy UI dialogue box (Node-based, not
//! egui) that wears the same walnut/amber HUD language as `hud.rs`: an
//! espresso plaque, a gold frame, and the `PANEL_BG`/`ACCENT_AMBER`/
//! `TEXT_CREAM` tokens reused verbatim (`docs/hud-design.md`).
//!
//! Renders: speaker nameplate + a per-character portrait swatch (canon
//! colours from `docs/character-design.md` §3 — no image assets needed, the
//! HUD itself is all geometric plaques, not skeuomorphic art), a
//! character-by-character typewriter reveal of the current line, a "Press E"
//! continue cue (blinking indicator + hint text), and clickable multi-choice
//! answer buttons.
//!
//! Reads `DialogueState` from quest.rs and writes `DialogueUiEvent` back so
//! the quest system applies the player's choices / advances / closes — same
//! contract the old egui version used. `DialogueState` mutation stays
//! quest.rs's job; the one deliberate exception is the debug proof-shot
//! seeder at the bottom of this file, clearly called out there.
//!
//! The objective tracker lives in a separate Bevy UI node (spawned by
//! quest.rs `spawn_objective_tracker`) — those are the only two HUD pieces
//! this module cares about.

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use bevy::ui::Interaction;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::hud::{ACCENT_AMBER, PANEL_BG, PANEL_BORDER, TEXT_CREAM, TEXT_DIM};
use crate::quest::{DialogueState, DialogueUiEvent, StoryDataRes};

// ===========================================================================
// Layout / pacing tokens
// ===========================================================================

const BOX_W: f32 = 640.0;
const BOX_BOTTOM: f32 = 28.0;
const PORTRAIT_SIZE: f32 = 48.0;
const CHARS_PER_SEC: f32 = 30.0;

pub struct DialogueUiPlugin;

impl Plugin for DialogueUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Typewriter>().add_systems(
            Update,
            (
                dialogue_debug_proof_seed,
                dialogue_typewriter_and_input,
                dialogue_box_sync.run_if(dialogue_proof_not_before),
            )
                .chain()
                .run_if(in_state(crate::editor::AppState::Play)),
        );
        // The pre-redesign egui dialogue box — the "before" reference for the
        // dialogue-UI redesign proof. Opt-in via `VOXELFORGE_DIALOGUE_PROOF=
        // before`; the native Node box above stays the default rendering path.
        app.add_systems(
            EguiPrimaryContextPass,
            dialogue_box_egui
                .run_if(in_state(crate::editor::AppState::Play))
                .run_if(dialogue_proof_before),
        );
    }
}

/// Run condition: the proof is rendering the pre-redesign (egui) box.
fn dialogue_proof_before() -> bool {
    std::env::var("VOXELFORGE_DIALOGUE_PROOF").as_deref() == Ok("before")
}

/// Inverse of [`dialogue_proof_before`] — the new native box draws unless the
/// `before` reference is requested.
fn dialogue_proof_not_before() -> bool {
    !dialogue_proof_before()
}

// ===========================================================================
// Components / resources
// ===========================================================================

#[derive(Component)]
struct DialogueRoot;
#[derive(Component)]
struct DialogueCloseButton;
#[derive(Component)]
struct DialogueChoiceButton(usize);

/// Per-line reveal progress, keyed to `(dialogue_id, current_line)` so a new
/// line (or a fresh dialogue) always starts its typewriter at zero. Lives
/// here, not on `DialogueState` — this is presentation-only state, quest.rs
/// doesn't need to know how far a line has typed.
#[derive(Resource, Default)]
struct Typewriter {
    key: (String, usize),
    revealed: f32,
}

// ===========================================================================
// Input — advance / skip-reveal. Choices are picked by number key (quest.rs's
// own `resolve_dialogue_actions` already reads Digit1-6 directly) or by
// clicking a choice button below; this system never auto-picks a choice on a
// "continue" press — that overload was the exact bug the old egui version's
// comment warned about (Enter advancing into choosing-mode picked choice 0
// on the same frame). E/Space stay advance-only, same fix, same key policy.
// ===========================================================================

fn dialogue_typewriter_and_input(
    time: Res<Time>,
    dialogue: Res<DialogueState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut tw: ResMut<Typewriter>,
    mut ui_events: MessageWriter<DialogueUiEvent>,
) {
    if !dialogue.open {
        return;
    }

    let key = (dialogue.dialogue_id.clone(), dialogue.current_line);
    if tw.key != key {
        tw.key = key;
        tw.revealed = 0.0;
    }

    let full_len = current_line(&dialogue).chars().count() as f32;
    let was_full = tw.revealed >= full_len;
    let continue_pressed = keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::Space);

    if dialogue.choosing {
        tw.revealed = full_len;
    } else if continue_pressed {
        if was_full {
            ui_events.write(DialogueUiEvent::Advance);
        } else {
            // First press just finishes the line instead of advancing past it.
            tw.revealed = full_len;
        }
    } else {
        tw.revealed = (tw.revealed + time.delta_secs() * CHARS_PER_SEC).min(full_len);
    }
}

fn current_line(dialogue: &DialogueState) -> &str {
    dialogue
        .lines
        .get(dialogue.current_line)
        .map(String::as_str)
        .unwrap_or("")
}

fn revealed_text(dialogue: &DialogueState, tw: &Typewriter) -> String {
    current_line(dialogue).chars().take(tw.revealed as usize).collect()
}

/// Canon portrait colours, `docs/character-design.md` §3: Maren's walnut robe
/// + warm-grey hair, Toma's lighter patched-overalls walnut, and `echo` — a
/// non-living remembered voice with no body — rendered as an empty amber-ring
/// plaque rather than invented flesh. Unknown speakers fall back to the plain
/// HUD panel tone so a missing entry never renders as an error state.
fn portrait_look(speaker: &str, speaker_display: &str) -> (Color, Color, char) {
    let glyph = speaker_display
        .chars()
        .find(|c| c.is_alphabetic())
        .unwrap_or('?')
        .to_ascii_uppercase();
    match speaker {
        "maren" => (Color::srgb_u8(0x6B, 0x4A, 0x2E), Color::srgb_u8(0xC9, 0xC0, 0xB4), glyph),
        "toma" => (Color::srgb_u8(0x8A, 0x63, 0x3E), ACCENT_AMBER, glyph),
        "echo" => (PANEL_BG, ACCENT_AMBER, '~'),
        _ => (PANEL_BG, PANEL_BORDER, glyph),
    }
}

// ===========================================================================
// Render — despawns and rebuilds the box every frame it's open. The tree is
// small (a dozen-odd nodes) and content changes every frame anyway while
// typing, so this mirrors the same despawn/respawn-per-frame pattern
// `quest.rs`'s own `[E] Talk to <name>` interaction prompt already uses,
// rather than inventing a second incremental-mutation style in this file.
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn dialogue_box_sync(
    mut commands: Commands,
    dialogue: Res<DialogueState>,
    tw: Res<Typewriter>,
    time: Res<Time>,
    mut ui_events: MessageWriter<DialogueUiEvent>,
    old_root_q: Query<Entity, With<DialogueRoot>>,
    close_q: Query<&Interaction, With<DialogueCloseButton>>,
    choice_q: Query<(&Interaction, &DialogueChoiceButton)>,
) {
    // Read last frame's clicks before tearing that frame's tree down.
    if let Ok(interaction) = close_q.single() {
        if *interaction == Interaction::Pressed {
            ui_events.write(DialogueUiEvent::Close);
        }
    }
    for (interaction, choice) in choice_q.iter() {
        if *interaction == Interaction::Pressed {
            ui_events.write(DialogueUiEvent::Choose(choice.0));
        }
    }

    for e in old_root_q.iter() {
        commands.entity(e).despawn();
    }

    if !dialogue.open {
        return;
    }

    let (portrait_fill, portrait_ring, glyph) = portrait_look(&dialogue.speaker, &dialogue.speaker_display);
    let full_len = current_line(&dialogue).chars().count() as f32;
    let fully_revealed = tw.revealed >= full_len;
    let revealed = revealed_text(&dialogue, &tw);
    let is_last_line = dialogue.current_line + 1 >= dialogue.lines.len();

    let hint = if dialogue.choosing {
        "Choose an answer -- click, or press 1-9"
    } else if !fully_revealed {
        "Press E to skip"
    } else if is_last_line && dialogue.choices.is_empty() {
        "Press E to close"
    } else {
        "Press E to continue"
    };

    // Slow amber pulse on the continue cue, same idle-motion language as
    // `hud.rs`'s low-HP pulse (a sine wave, not a linear blink).
    let blink_alpha = ((time.elapsed_secs() * 3.0).sin() * 0.5 + 0.5) * 0.7 + 0.3;

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(BOX_BOTTOM),
                left: Val::Percent(50.0),
                width: Val::Px(BOX_W),
                margin: UiRect { left: Val::Px(-BOX_W / 2.0), ..default() },
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            // Gold frame (vs. the HUD's usual espresso `PANEL_BORDER`) — the
            // one deliberate accent break that marks this as a focus surface,
            // same read the old egui box's gold stroke was going for.
            BorderColor::all(ACCENT_AMBER),
            DialogueRoot,
        ))
        .with_children(|box_| {
            // ---- header: portrait + speaker name + close --------------------
            box_.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|header| {
                header
                    .spawn((
                        Node {
                            width: Val::Px(PORTRAIT_SIZE),
                            height: Val::Px(PORTRAIT_SIZE),
                            border: UiRect::all(Val::Px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BackgroundColor(portrait_fill),
                        BorderColor::all(portrait_ring),
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Text::new(glyph.to_string()),
                            TextFont { font_size: bevy::text::FontSize::from(22.0), ..default() },
                            TextColor(TEXT_CREAM),
                        ));
                    });

                header
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_grow: 1.0,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|nc| {
                        nc.spawn((
                            Text::new(dialogue.speaker_display.clone()),
                            TextFont { font_size: bevy::text::FontSize::from(18.0), ..default() },
                            TextColor(ACCENT_AMBER),
                        ));
                        nc.spawn((
                            Node { padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)), ..default() },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                            Interaction::default(),
                            DialogueCloseButton,
                        ))
                        .with_children(|c| {
                            c.spawn((
                                Text::new("X"),
                                TextFont { font_size: bevy::text::FontSize::from(14.0), ..default() },
                                TextColor(TEXT_DIM),
                            ));
                        });
                    });
            });

            box_.spawn((Node { height: Val::Px(1.0), ..default() }, BackgroundColor(PANEL_BORDER)));

            // ---- body: current line (typing) or the choice list -------------
            if dialogue.choosing {
                box_.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|list| {
                    for (i, choice) in dialogue.choices.iter().enumerate() {
                        list.spawn((
                            Node {
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                border_radius: BorderRadius::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(PANEL_BORDER.with_alpha(0.35)),
                            BorderColor::all(PANEL_BORDER),
                            Interaction::default(),
                            DialogueChoiceButton(i),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new(format!("{}. {}", i + 1, choice.label)),
                                TextFont { font_size: bevy::text::FontSize::from(14.0), ..default() },
                                TextColor(TEXT_CREAM),
                            ));
                        });
                    }
                });
            } else {
                box_.spawn((
                    Text::new(revealed),
                    TextFont { font_size: bevy::text::FontSize::from(15.0), ..default() },
                    TextColor(TEXT_CREAM),
                    Node { width: Val::Percent(100.0), ..default() },
                ));
            }

            // ---- footer: blinking continue cue + hint ------------------------
            box_.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|h| {
                if !dialogue.choosing && fully_revealed {
                    h.spawn((
                        Node { width: Val::Px(7.0), height: Val::Px(7.0), ..default() },
                        BackgroundColor(ACCENT_AMBER.with_alpha(blink_alpha)),
                    ));
                }
                h.spawn((
                    Text::new(hint),
                    TextFont { font_size: bevy::text::FontSize::from(11.0), ..default() },
                    TextColor(TEXT_DIM),
                ));
            });
        });
}

// ===========================================================================
// Debug proof-shot seeder — the ONE place in this file that writes
// `DialogueState` directly instead of going through `DialogueUiEvent`.
//
// Off by default; opt-in via `VOXELFORGE_DIALOGUE_PROOF` (same "env var
// lever, no rebuild to disable" pattern as `VOXELFORGE_QUEST_DEMO` /
// `GATE3_SKIP_WAIT` elsewhere in this repo). It exists so a screenshot proof
// doesn't depend on `--quest-demo`'s walk-to-gate timing lining up with
// `screenshot_once`'s fixed t=3.2s capture (`main.rs`) — instead it seeds the
// REAL `dlg_maren_gate` node (Elder Maren's actual 4 lines + 5 choices, from
// `assets/story/act1.json`, Rose's lane) straight into `DialogueState`, so
// what's on screen is real shipped story content rendered by the real UI,
// not placeholder text.
//
//   VOXELFORGE_DIALOGUE_PROOF=before   seeds immediately, first line full; the
//                                      pre-redesign egui box draws instead of
//                                      the native Node box (see `dialogue_box_egui`).
//   VOXELFORGE_DIALOGUE_PROOF=typing   seeds at t=2.9s, 0.3s before the shot
//                                      fires, so the capture genuinely lands
//                                      mid-reveal instead of a frozen fake.
//   VOXELFORGE_DIALOGUE_PROOF=choices  seeds immediately with `choosing=true`
//                                      so the 5-choice list is fully laid out
//                                      well before the t=3.2s shot.
//
// Never touches quest.rs; only reads its public `StoryDataRes` and writes the
// public `DialogueState` fields, same access any system in this crate has.
fn dialogue_debug_proof_seed(
    time: Res<Time>,
    mut seeded: Local<bool>,
    mut dialogue: ResMut<DialogueState>,
    story: Res<StoryDataRes>,
) {
    if *seeded {
        return;
    }
    let Ok(mode) = std::env::var("VOXELFORGE_DIALOGUE_PROOF") else { return };
    let seed_at = if mode == "typing" { 2.9 } else { 0.05 };
    if time.elapsed_secs() < seed_at {
        return;
    }
    *seeded = true;

    let Some(dlg) = story.data.dialogue.iter().find(|d| d.id == "dlg_maren_gate") else {
        println!("DIALOGUE_PROOF_SEED dlg_maren_gate not found in story data");
        return;
    };
    dialogue.speaker = dlg.speaker.clone();
    dialogue.speaker_display = dlg.speaker_display.clone();
    dialogue.lines = dlg.lines.clone();
    dialogue.choices = dlg.choices.clone().unwrap_or_default();
    dialogue.dialogue_id = dlg.id.clone();
    dialogue.quest_id = dlg.quest.clone();
    dialogue.open = true;
    dialogue.pending_action = None;
    dialogue.current_line = if mode == "choices" { dialogue.lines.len() } else { 0 };
    dialogue.choosing = mode == "choices";

    println!(
        "DIALOGUE_PROOF_SEED mode={mode} t={:.2} lines={} choices={}",
        time.elapsed_secs(),
        dialogue.lines.len(),
        dialogue.choices.len()
    );
}

// ===========================================================================
// Pre-redesign dialogue box (egui) — the `VOXELFORGE_DIALOGUE_PROOF=before`
// reference. Pulled verbatim from `git show HEAD:client/src/dialogue_ui.rs`
// (the egui overlay that preceded the native Node box) so the before shot is a
// faithful render of the old look, not a re-imagining. Registered in
// `EguiPrimaryContextPass` and gated to the `before` proof mode, so it never
// runs (and never double-draws over the native box) in normal play.
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn dialogue_box_egui(
    mut contexts: EguiContexts,
    dialogue: Res<DialogueState>,
    mut ui_events: MessageWriter<DialogueUiEvent>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !dialogue.open {
        return;
    }

    // ---- Keyboard fallback: Escape to close, Space/Enter to advance ----------
    // Enter never picks a choice — that is the bug that sent the quest demo into
    // the wrong dialogue branch every time the last line of dlg_maren_gate
    // advanced into choosing mode on the same frame Enter was still held.
    if keys.just_pressed(KeyCode::Escape) {
        ui_events.write(DialogueUiEvent::Close);
        return; // close wins — don't also send an advance/choice this frame
    }
    if keys.just_pressed(KeyCode::Space) {
        if dialogue.choosing {
            ui_events.write(DialogueUiEvent::Choose(0));
        } else {
            ui_events.write(DialogueUiEvent::Advance);
        }
    } else if keys.just_pressed(KeyCode::Enter) && !dialogue.choosing {
        ui_events.write(DialogueUiEvent::Advance);
    }
    // Digit keys 1–6 → choice indices 0–5 (dlg_maren_gate has 5 choices).
    for i in 0..6usize {
        let key = match i {
            0 => KeyCode::Digit1,
            1 => KeyCode::Digit2,
            2 => KeyCode::Digit3,
            3 => KeyCode::Digit4,
            4 => KeyCode::Digit5,
            _ => KeyCode::Digit6,
        };
        if keys.just_pressed(key) && dialogue.choosing {
            ui_events.write(DialogueUiEvent::Choose(i));
        }
    }

    // ---- egui window ----------------------------------------------------------
    let Ok(ctx) = contexts.ctx_mut() else { return; };
    let max_w = ctx.input(|i| i.viewport_rect().width() * 0.7);
    let text_color = egui::Color32::from_rgb(230, 225, 215);
    let gold = egui::Color32::from_rgb(244, 184, 96);
    let dim = egui::Color32::from_rgb(140, 135, 125);
    let bg = egui::Color32::from_rgba_premultiplied(10, 10, 18, 238);

    egui::Area::new("dialogue_box".into())
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
        .movable(false)
        .interactable(true)
        .show(ctx, |ui| {
            egui::Frame::window(ctx.global_style().as_ref())
                .fill(bg)
                .stroke(egui::Stroke::new(1.5, gold))
                .inner_margin(egui::Margin::symmetric(18, 14))
                .show(ui, |ui| {
                    ui.set_max_width(max_w);
                    ui.set_min_width(max_w.min(640.0));

                    // ---- Header: speaker name + close button ------------------
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            gold,
                            egui::RichText::new(&dialogue.speaker_display)
                                .size(16.0)
                                .strong(),
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .small_button(egui::RichText::new("✕").size(14.0))
                                    .clicked()
                                {
                                    ui_events.write(DialogueUiEvent::Close);
                                }
                            },
                        );
                    });
                    ui.separator();
                    ui.add_space(4.0);

                    // ---- Current line text ------------------------------------
                    if dialogue.current_line < dialogue.lines.len() {
                        ui.label(
                            egui::RichText::new(&dialogue.lines[dialogue.current_line])
                                .size(15.0)
                                .color(text_color),
                        );
                    }

                    // ---- Choice buttons ---------------------------------------
                    if dialogue.choosing && !dialogue.choices.is_empty() {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                        for (i, choice) in dialogue.choices.iter().enumerate() {
                            let label = format!("{}. {}", i + 1, choice.label);
                            let btn = egui::Button::new(
                                egui::RichText::new(&label).size(14.0),
                            )
                            .min_size(egui::vec2(max_w.min(500.0), 0.0));
                            if ui.add(btn).clicked() {
                                ui_events.write(DialogueUiEvent::Choose(i));
                            }
                        }
                    }

                    // ---- Hint text -------------------------------------------
                    ui.add_space(4.0);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let hint = if dialogue.choosing {
                                "Pick a choice — press 1–4 or click a button"
                            } else if dialogue.current_line + 1 >= dialogue.lines.len() {
                                "Press Space or click ✕ to close"
                            } else {
                                "Press Space to continue"
                            };
                            ui.label(
                                egui::RichText::new(hint).size(11.0).color(dim),
                            );
                        },
                    );
                });
        });
}
