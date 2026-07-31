//! Voxelforge dialogue UI — egui overlay that renders the current dialogue
//! text, speaker name, choice buttons, and a close control. Reads
//! `DialogueState` from quest.rs and writes `DialogueUiEvent` back so the
//! quest system can apply the player's choices / advances / closes.
//!
//! Runs in `EguiPrimaryContextPass` so egui is initialised before we draw.
//! The objective tracker lives in a separate Bevy UI node (spawned by
//! quest.rs `spawn_objective_tracker`) — those are the only two HUD pieces
//! this module cares about.

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::quest::{DialogueState, DialogueUiEvent};

pub struct DialogueUiPlugin;

impl Plugin for DialogueUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            EguiPrimaryContextPass,
            dialogue_box.run_if(in_state(crate::editor::AppState::Play)),
        );
    }
}

fn dialogue_box(
    mut contexts: EguiContexts,
    dialogue: Res<DialogueState>,
    mut ui_events: MessageWriter<DialogueUiEvent>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !dialogue.open {
        return;
    }

    // ---- Keyboard fallback: Escape to close, Space to advance ----------------
    if keys.just_pressed(KeyCode::Escape) {
        ui_events.write(DialogueUiEvent::Close);
    }
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) {
        if dialogue.choosing {
            // Space during choices picks the first one.
            ui_events.write(DialogueUiEvent::Choose(0));
        } else {
            ui_events.write(DialogueUiEvent::Advance);
        }
    }
    for i in 0..4usize {
        let key = match i {
            0 => KeyCode::Digit1,
            1 => KeyCode::Digit2,
            2 => KeyCode::Digit3,
            _ => KeyCode::Digit4,
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
