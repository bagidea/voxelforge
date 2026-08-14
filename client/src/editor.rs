//! Editor shell — the Roblox-Studio-style build/play split for Voxelforge.
//!
//! `EditorPlugin` owns the top-level [`AppState`] (`Editor` ↔ `Play`) and the
//! interactive voxel edit loop you get while building: aim the orbit camera at a
//! block, left-click to break it, right-click to place the [`SelectedBlock`]. Both
//! reuse the raycast + single-chunk remesh path the headless `edit_demo` proves
//! (`crate::raycast_voxel` → `crate::set_world_voxel`), so hand-building and the
//! scripted proof edit the world through exactly the same code.
//!
//! This is round-1 skeleton: it defines the state machine + the one resource the
//! rest of the team hooks into (`SelectedBlock`). Round-2 assembly plugs the other
//! editor crates in beside it (see the plug points in `main.rs`): Rose's scene,
//! Yamamoto's UI (which writes `SelectedBlock`), Kevin's import, Shiba's editor
//! camera, Sun's input map.

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};
use bevy_egui::EguiContexts;

use voxelforge_sim::block::BlockId;

use crate::gizmo::GizmoState;
use crate::settings_menu::{settings_closed, SettingsMenuSet};
use crate::{paint_at_cursor, OrbitCam, PaintOp, World};

/// Top-level mode. `MainMenu` is the game entrance (New Game / Continue / Settings /
/// Quit) shown on a plain launch; `Editor` builds the world (combat/physics AI held
/// off); `Play` drops you into the scene with the hero + combat live. Enter → Play,
/// Esc → Editor. Defaults to `MainMenu` so the product opens on the menu, not the
/// dev build tool — the scripted/bench/shot lanes still land in `Editor` explicitly
/// (see `boot_state` in main.rs).
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
    Editor,
    Play,
}

/// The block a right-click places while in [`AppState::Editor`]. Defaults to STONE;
/// Yamamoto's `EditorUiPlugin` writes this from the block-pick UI. It is the single
/// interface the editor exposes for "what block am I holding" — any system may read
/// or set it (`SelectedBlock(BlockId)`, tuple field `.0`).
#[derive(Resource, Clone, Copy, Debug)]
pub struct SelectedBlock(pub BlockId);

impl Default for SelectedBlock {
    fn default() -> Self {
        Self(BlockId::STONE)
    }
}

/// How far the editor's aim ray reaches, in voxels (matches the play-mode reach).
pub(crate) const EDIT_REACH: f32 = 200.0;

/// Wires the editor state machine + the interactive build loop into the app.
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .init_resource::<SelectedBlock>()
            // Shared editor contract other tools read: what's picked (Selection),
            // grid-snap settings (SnapGrid), and whether this is a scripted run
            // (Scripted — see the note on `Scripted`).
            //
            // The interactive editor camera + transform gizmo were split back out of
            // this plugin (Director trimmed them off Poppy's scope — handed to another
            // owner next round). The selection/snap/scripted contract below stays so
            // whoever builds them just re-adds their sub-plugins here.
            .init_resource::<Selection>()
            .init_resource::<SnapGrid>()
            .init_resource::<Scripted>()
            .add_systems(
                Update,
                (
                    // editor_edit runs only for a human in the editor (the
                    // scripted proofs drive the world directly). It runs after
                    // the gizmo's begin-drag so a handle grab this frame is
                    // already reflected in `GizmoState`.
                    editor_edit
                        .after(crate::gizmo::gizmo_begin_drag)
                        .run_if(in_interactive_editor),
                    enter_play.run_if(in_state(AppState::Editor)),
                    exit_play
                        .run_if(in_state(AppState::Play))
                        .run_if(settings_closed)
                        .after(SettingsMenuSet),
                ),
            )
            .add_systems(OnEnter(AppState::Play), on_enter_play)
            .add_systems(OnEnter(AppState::Editor), on_enter_editor);
    }
}

/// Interactive build loop (interactive Editor only): aim with the free cursor,
/// **left-click** to place the [`SelectedBlock`] on the face under the pointer,
/// **right-click** to break the voxel under the pointer. Both route through the
/// shared [`crate::paint_at_cursor`] (screen→world raycast → `set_world_voxel`) —
/// the same function the headless `editor_paint_demo` proof drives — so a click
/// and the proof edit the world through one code path, and the touched chunk
/// re-meshes in place.
///
/// It yields to the other left-click consumers: it does nothing while the cursor
/// is over an egui panel, while a gizmo handle is being dragged, or (for place)
/// when a [`Selectable`] entity is under the pointer — so selecting an entity for
/// the gizmo never also stamps a block on it.
fn editor_edit(
    mut contexts: EguiContexts,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
    selectables: Query<(Entity, &GlobalTransform, &Aabb), With<Selectable>>,
    gizmo: Res<GizmoState>,
    selected: Res<SelectedBlock>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<World>,
) {
    // Don't edit through the UI. (`ctx_mut` — bevy_egui gates the immutable
    // `ctx()` behind its `immutable_ctx` feature, which we don't enable.)
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.is_pointer_over_egui() {
            return;
        }
    }
    // Don't edit while dragging a transform-gizmo handle.
    if gizmo.is_dragging() {
        return;
    }

    let place = mouse.just_pressed(MouseButton::Left);
    let brk = mouse.just_pressed(MouseButton::Right);
    if !place && !brk {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((cam, cam_gt)) = cam_q.single() else {
        return;
    };

    // A place click that lands on a Selectable entity yields to the picker.
    if place && selectable_under_cursor(&selectables, cam, cam_gt, cursor) {
        return;
    }

    let op = if brk { PaintOp::Break } else { PaintOp::Place };
    let _ = paint_at_cursor(
        &mut world,
        &mut meshes,
        &mut commands,
        cam,
        cam_gt,
        cursor,
        op,
        selected.0,
    );
}

/// True if a [`Selectable`] entity's mesh volume is under the screen `cursor` —
/// the same slab test `pick_selectable` uses, factored out so a place-click can
/// tell "I'm over an entity" from "I'm over open terrain".
fn selectable_under_cursor(
    selectables: &Query<(Entity, &GlobalTransform, &Aabb), With<Selectable>>,
    cam: &Camera,
    cam_gt: &GlobalTransform,
    cursor: Vec2,
) -> bool {
    let Ok(ray) = cam.viewport_to_world(cam_gt, cursor) else {
        return false;
    };
    let ro = ray.origin;
    let rd = ray.direction.as_vec3();
    for (_e, gt, aabb) in selectables.iter() {
        let inv = gt.affine().inverse();
        let lo = inv.transform_point3(ro);
        let ld = inv.transform_vector3(rd);
        let c = Vec3::from(aabb.center);
        let h = Vec3::from(aabb.half_extents);
        if ray_aabb(lo, ld, c - h, c + h).is_some() {
            return true;
        }
    }
    false
}

/// Enter → leave build mode and drop into Play (hero + combat come alive).
fn enter_play(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Enter) {
        next.set(AppState::Play);
    }
}

/// Esc → back to the editor. (`fly_camera` also reads Esc to release the cursor, so
/// the one press both frees the mouse and returns to build mode.)
fn exit_play(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(AppState::Editor);
    }
}

/// Crossing into Play. The avatar + orbit camera + first Guard Husk are already in the
/// scene (spawned at startup) and the combat/husk-AI systems are gated to run only in
/// this state — so entering Play is what actually brings the fight to life.
///
/// ROUND-2 PLUG POINT: once Rose's `ScenePlugin` owns scene/hero spawning, spawn a
/// fresh hero + encounter here instead of relying on the startup avatar.
fn on_enter_play() {
    println!("ENTER_PLAY");
}

/// Returning to Editor.
///
/// ROUND-2 PLUG POINT: tear down transient Play-only entities and restore the build
/// camera (Shiba's `EditorCamPlugin`) here.
fn on_enter_editor() {
    println!("ENTER_EDITOR");
}

// ===========================================================================
// Selection + gizmo shell contract. The camera/gizmo *implementation* is a
// separate round (handed off); this contract stays here for its next owner.
// ===========================================================================

/// The editor's current selection — the single source of truth for "what is
/// picked". The camera's raycast picker (`editor_camera::pick_selectable`) is
/// the only writer during normal use; other editor tools **read** it:
///
/// ```ignore
/// // Yamamoto's inspector:
/// fn inspector(sel: Res<Selection>, q: Query<&Transform>) {
///     if let Some(e) = sel.entity { if let Ok(tf) = q.get(e) { /* show tf */ } }
/// }
/// // Rose's scene panel — highlight the selected row:
/// fn scene_list(sel: Res<Selection>) { /* row.selected = sel.is(row_entity) */ }
/// ```
///
/// A tool that wants to drive selection itself (e.g. "select from the scene
/// tree") may write it via [`Selection::set`] / [`Selection::clear`].
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Selection {
    /// The currently-selected entity, or `None` when nothing is picked.
    pub entity: Option<Entity>,
}

impl Selection {
    /// True when `e` is the selected entity.
    pub fn is(&self, e: Entity) -> bool {
        self.entity == Some(e)
    }
    /// Select `e`.
    pub fn set(&mut self, e: Entity) {
        self.entity = Some(e);
    }
    /// Clear the selection.
    pub fn clear(&mut self) {
        self.entity = None;
    }
}

/// Marks an entity as pickable by the editor's left-click raycast. Rose's scene
/// puts this on anything it wants selectable; the picker ignores every entity
/// that lacks it, so terrain chunks / the player / HUD are never grabbed. The
/// entity also needs a mesh (hence a `bevy::camera::primitives::Aabb`, which
/// Bevy computes for meshed entities) for the ray to have a volume to hit.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Selectable;

/// Grid-snap settings for the transform gizmo. Toggle with `X`; steps are in
/// world units (translate/scale) and radians (rotate).
#[derive(Resource, Debug, Clone, Copy)]
pub struct SnapGrid {
    pub enabled: bool,
    /// World units a move snaps to when enabled.
    pub translate_step: f32,
    /// Radians a rotation snaps to when enabled (default 15°).
    pub rotate_step: f32,
    /// Scale increment a scale snaps to when enabled.
    pub scale_step: f32,
}

impl Default for SnapGrid {
    fn default() -> Self {
        Self {
            enabled: false,
            translate_step: 1.0,
            rotate_step: std::f32::consts::PI / 12.0, // 15°
            scale_step: 0.25,
        }
    }
}

/// True when this launch is a scripted/headless run (a demo, bench, or shot) as
/// opposed to a human sitting at the editor. `main` sets it from `Cfg`.
///
/// It exists because the headless walk/edit/map-save proofs run in the *default*
/// `Editor` state and drive the camera through the play-mode `fly_camera`. The
/// editor camera + gizmo therefore gate on [`in_interactive_editor`] (Editor
/// **and** not scripted) so they never hijack those screenshots, and the
/// play-mode `fly_camera` is only suppressed in an interactive editor session
/// (see [`not_interactive_editor`]).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct Scripted(pub bool);

/// Run condition: a human is in the editor (Editor state, not a scripted run).
/// The editor camera, picker and gizmo all gate on this.
pub fn in_interactive_editor(state: Res<State<AppState>>, scripted: Res<Scripted>) -> bool {
    matches!(state.get(), AppState::Editor) && !scripted.0
}

/// Run condition: the inverse of [`in_interactive_editor`]. The play-mode
/// `fly_camera` runs here — i.e. always in Play, and in Editor only for scripted
/// proofs — so the editor camera owns the view only when a human is building.
pub fn not_interactive_editor(state: Res<State<AppState>>, scripted: Res<Scripted>) -> bool {
    !(matches!(state.get(), AppState::Editor) && !scripted.0)
}

/// Run condition: this is not a scripted run (used to skip editor-camera setup
/// on headless boots, where OnEnter(Editor) still fires for the default state).
pub fn not_scripted(scripted: Res<Scripted>) -> bool {
    !scripted.0
}

// ---------------------------------------------------------------------------
// Shared geometry — used by the picker (unit-tested here, no GPU needed).
// ---------------------------------------------------------------------------

/// Slab-method ray vs axis-aligned box, all in the **same** space (the picker
/// calls it in the entity's local space, where the box is its mesh `Aabb`).
/// Returns the entry distance `t` (>= 0) along the ray, or `None` on a miss.
///
/// The caller deliberately leaves the ray direction un-normalised: after an
/// affine transform into local space the parameter `t` is preserved between
/// world and local space, so comparing `t` across entities stays valid and the
/// nearest hit is the smallest `t`.
pub(crate) fn ray_aabb(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let inv = Vec3::new(safe_inv(dir.x), safe_inv(dir.y), safe_inv(dir.z));
    let t1 = (min - origin) * inv;
    let t2 = (max - origin) * inv;
    let t_near = t1.min(t2).max_element();
    let t_far = t1.max(t2).min_element();
    if t_near <= t_far && t_far >= 0.0 {
        Some(t_near.max(0.0))
    } else {
        None
    }
}

/// Reciprocal that keeps the slab test well-behaved: a zero component maps to a
/// large finite value so a downstream `0 * inf` never produces a NaN.
fn safe_inv(v: f32) -> f32 {
    if v.abs() < 1e-8 {
        1e8 * if v < 0.0 { -1.0 } else { 1.0 }
    } else {
        1.0 / v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hits_box_dead_centre() {
        // Ray from -Z toward +Z, straight at a unit box at the origin.
        let t = ray_aabb(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::Z,
            Vec3::splat(-1.0),
            Vec3::splat(1.0),
        );
        // Enters at z = -1, i.e. 4 units along the ray.
        assert!((t.unwrap() - 4.0).abs() < 1e-4, "t = {:?}", t);
    }

    #[test]
    fn ray_misses_box_beside_it() {
        let t = ray_aabb(
            Vec3::new(10.0, 0.0, -5.0),
            Vec3::Z,
            Vec3::splat(-1.0),
            Vec3::splat(1.0),
        );
        assert!(t.is_none());
    }

    #[test]
    fn nearer_box_wins_by_smaller_t() {
        // The picker keeps the smaller t; prove the two boxes order correctly.
        let near = ray_aabb(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::Z,
            Vec3::splat(-1.0),
            Vec3::splat(1.0),
        )
        .unwrap();
        let far = ray_aabb(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::Z,
            Vec3::new(-1.0, -1.0, 9.0),
            Vec3::new(1.0, 1.0, 11.0),
        )
        .unwrap();
        assert!(near < far, "near {near} should be < far {far}");
    }

    #[test]
    fn box_entirely_behind_the_ray_is_not_hit() {
        let t = ray_aabb(
            Vec3::ZERO,
            Vec3::Z,
            Vec3::new(-1.0, -1.0, -20.0),
            Vec3::new(1.0, 1.0, -10.0),
        );
        assert!(t.is_none());
    }

    #[test]
    fn selection_helpers_round_trip() {
        let mut s = Selection::default();
        assert!(s.entity.is_none());
        let e = Entity::from_raw_u32(7).unwrap();
        s.set(e);
        assert!(s.is(e));
        s.clear();
        assert!(s.entity.is_none());
    }
}
