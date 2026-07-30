//! Voxelforge level-editor UI — four egui panels, wired through bevy_egui.
//!
//! Plugin: `app.add_plugins(EditorUiPlugin)`
//!
//! Panels (in layout order, no central panel consumed):
//!   top    — toolbar: Save / Load / Play-Stop
//!   bottom — block palette / hotbar  →  writes `SelectedBlock`
//!   left   — entity list              →  writes `EditorState::selected_entity`
//!   right  — transform inspector      →  edits `Transform` of selected entity
//!
//! ── Wiring notes for teammates ───────────────────────────────────────────
//!   Poppy  — `SelectedBlock` resource (declared below as placeholder).
//!            If you define it in your own module, delete the struct here
//!            and re-export / import yours — the system reads it via
//!            `ResMut<SelectedBlock>`.
//!   Rose   — `MessageReader<SaveRequest>` → write map file to disk.
//!            `MessageReader<LoadRequest>` → read map file from disk.
//!   Shiba  — `Res<EditorState>::selected_entity` → drive your gizmo /
//!            highlight system. The entity is the Bevy ECS Entity handle.

use bevy::prelude::*;
// bevy_egui 0.41 moved egui drawing into its own schedule. Any system that borrows
// `EguiContexts` MUST run in `EguiPrimaryContextPass` (the pass where the plugin
// first calls `Context::run`), or it panics: "No fonts available until first call
// to Context::run()". So the five UI systems below live in that pass, not `Update`.
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use voxelforge_sim::block::BlockId;

use crate::import::{ModelCatalog, ModelKind, SpawnModel};
use crate::editor::AppState;
use crate::FlyCam;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Unified during Round-2 assembly (Shino): the palette writes the SAME resource
/// Poppy's raycast-place loop reads, so clicking a tile actually changes the block
/// you place. Was a duplicate placeholder type here; now re-exported from `editor`.
pub use crate::editor::SelectedBlock;

/// Editor state shared with other systems (read-only for gizmos / highlights).
///
/// Shiba: read `selected_entity` to anchor your gizmo transform.
#[derive(Resource, Default)]
pub struct EditorState {
    /// True while the editor toolbar is in "play" mode.
    pub play_mode: bool,
    /// Entity currently selected in the entity list panel.
    pub selected_entity: Option<Entity>,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Fired when the user clicks the Save button in the toolbar.
/// Rose: subscribe with `MessageReader<SaveRequest>` to write the map file.
#[derive(Message)]
pub struct SaveRequest;

/// Fired when the user clicks the Load button in the toolbar.
/// Rose: subscribe with `MessageReader<LoadRequest>` to read the map file.
#[derive(Message)]
pub struct LoadRequest;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
        }
        app.init_resource::<EditorState>()
            .init_resource::<SelectedBlock>()
            .add_message::<SaveRequest>()
            .add_message::<LoadRequest>()
            .add_systems(
                EguiPrimaryContextPass,
                (toolbar_ui, block_palette_ui, model_browser_ui, entity_list_ui, inspector_ui)
                    .chain()
                    .run_if(in_state(AppState::Editor)),
            );
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Top toolbar: Save / Load / Play-Stop.
fn toolbar_ui(
    mut contexts: EguiContexts,
    mut state: ResMut<EditorState>,
    mut save_ev: MessageWriter<SaveRequest>,
    mut load_ev: MessageWriter<LoadRequest>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return; };
    egui::Window::new("vf_toolbar")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .current_pos([8.0, 8.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    save_ev.write(SaveRequest);
                }
                if ui.button("Load").clicked() {
                    load_ev.write(LoadRequest);
                }
                ui.separator();
                let play_lbl = if state.play_mode { "Stop" } else { "Play" };
                if ui.button(play_lbl).clicked() {
                    state.play_mode = !state.play_mode;
                }
            });
        });
}

/// Bottom block palette / hotbar — clicking a tile sets `SelectedBlock`.
fn block_palette_ui(
    mut contexts: EguiContexts,
    mut selected: ResMut<SelectedBlock>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return; };
    egui::Window::new("vf_palette")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .current_pos([8.0, 680.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
            ui.label("Block:");
            for &block in BlockId::ALL_PLACEABLE {
                let active = selected.0 == block;
                let btn = egui::Button::new(block.name()).fill(if active {
                    egui::Color32::from_rgb(60, 120, 200)
                } else {
                    egui::Color32::from_rgb(45, 45, 58)
                });
                if ui.add(btn).clicked() {
                    selected.0 = block;
                }
            }
        });
    });
}

/// Model browser: one "Import <name>" button per `.vox` the startup scan found in
/// `assets/models/`. Clicking emits a `SpawnModel` the import plugin handles
/// (parse → spawn voxel cubes), dropped a few blocks in front of the player so it
/// lands in view. This is the "Import .vox" entry point the editor spec calls for.
fn model_browser_ui(
    mut contexts: EguiContexts,
    catalog: Res<ModelCatalog>,
    player: Query<&Transform, With<FlyCam>>,
    mut spawn: MessageWriter<SpawnModel>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::Window::new("vf_models")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .current_pos([1000.0, 60.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Import .vox:");
                let mut shown = 0;
                for entry in catalog.entries.iter().filter(|e| e.kind == ModelKind::Voxel) {
                    if ui.button(&entry.name).clicked() {
                        // Drop the model a few blocks in front of the player so it
                        // appears in view; the import plugin realises the voxels.
                        let tf = player.single().copied().unwrap_or(Transform::IDENTITY);
                        let fwd = tf.forward();
                        let flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
                        let pos = tf.translation + flat * 4.0;
                        spawn.write(SpawnModel {
                            path: entry.path.clone(),
                            transform: Transform::from_translation(pos),
                        });
                    }
                    shown += 1;
                }
                if shown == 0 {
                    ui.label("(drop a .vox in assets/models/)");
                }
            });
        });
}

/// Left entity list — all entities in the scene; click to select.
fn entity_list_ui(
    mut contexts: EguiContexts,
    mut state: ResMut<EditorState>,
    entities: Query<(Entity, Option<&Name>)>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return; };
    egui::Window::new("vf_entity_list")
        .resizable(true)
        .collapsible(false)
        .default_width(180.0)
        .current_pos([8.0, 90.0])
        .show(ctx, |ui| {
            ui.heading("Entities");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (entity, name) in &entities {
                    let label = match name {
                        Some(n) => format!("{} ({:?})", n.as_str(), entity),
                        None => format!("{:?}", entity),
                    };
                    let is_sel = state.selected_entity == Some(entity);
                    if ui.selectable_label(is_sel, &label).clicked() {
                        state.selected_entity = Some(entity);
                    }
                }
            });
        });
}

/// Right inspector — shows and edits Transform of the selected entity.
///
/// Pattern: snapshot transform before the panel (releasing the read borrow),
/// accumulate edits in local vars inside the closure, write back after the
/// panel if anything changed.  This keeps the mutable query borrow out of
/// the closure so Rust's borrow checker is happy.
fn inspector_ui(
    mut contexts: EguiContexts,
    state: Res<EditorState>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(entity) = state.selected_entity else { return; };
    let Ok(ctx) = contexts.ctx_mut() else { return; };

    // Snapshot current values — borrow ends at the closing `}`.
    let (mut t, rot, mut s) = {
        let Ok(tf) = transforms.get(entity) else { return; };
        (tf.translation, tf.rotation, tf.scale)
    };
    let (ex, ey, ez) = rot.to_euler(EulerRot::XYZ);
    let (mut rx, mut ry, mut rz) = (ex.to_degrees(), ey.to_degrees(), ez.to_degrees());
    let mut dirty = false;

    egui::Window::new("vf_inspector")
        .resizable(true)
        .collapsible(false)
        .default_width(240.0)
        .current_pos([720.0, 8.0])
        .show(ctx, |ui| {
            ui.heading("Inspector");
            ui.separator();
            ui.label(format!("Entity: {entity:?}"));
            ui.separator();

            egui::Grid::new("vf_tf_grid")
                .num_columns(4)
                .spacing([4.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Pos");
                    dirty |= ui.add(egui::DragValue::new(&mut t.x).speed(0.1).prefix("X ")).changed();
                    dirty |= ui.add(egui::DragValue::new(&mut t.y).speed(0.1).prefix("Y ")).changed();
                    dirty |= ui.add(egui::DragValue::new(&mut t.z).speed(0.1).prefix("Z ")).changed();
                    ui.end_row();

                    ui.label("Rot°");
                    dirty |= ui.add(egui::DragValue::new(&mut rx).speed(1.0).prefix("X ")).changed();
                    dirty |= ui.add(egui::DragValue::new(&mut ry).speed(1.0).prefix("Y ")).changed();
                    dirty |= ui.add(egui::DragValue::new(&mut rz).speed(1.0).prefix("Z ")).changed();
                    ui.end_row();

                    ui.label("Scale");
                    dirty |= ui.add(egui::DragValue::new(&mut s.x).speed(0.01).prefix("X ")).changed();
                    dirty |= ui.add(egui::DragValue::new(&mut s.y).speed(0.01).prefix("Y ")).changed();
                    dirty |= ui.add(egui::DragValue::new(&mut s.z).speed(0.01).prefix("Z ")).changed();
                    ui.end_row();
                });
        });

    if dirty {
        if let Ok(mut tf) = transforms.get_mut(entity) {
            tf.translation = t;
            tf.rotation = Quat::from_euler(
                EulerRot::XYZ,
                rx.to_radians(),
                ry.to_radians(),
                rz.to_radians(),
            );
            tf.scale = s;
        }
    }
}
