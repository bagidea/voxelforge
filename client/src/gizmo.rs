//! Transform gizmo (Poppy — took over Shiba's gizmo socket).
//!
//! A dependency-free move/rotate/scale gizmo drawn with Bevy's immediate-mode
//! [`Gizmos`] and dragged with the mouse — no external gizmo crate, so it builds
//! green against Bevy 0.19 with no version-matching risk. It operates on whatever
//! [`Selection`] holds (written by `editor_camera::pick_selectable`).
//!
//! Keys (interactive Editor only):
//!   * **1 / 2 / 3** — Translate / Rotate / Scale mode.
//!   * **X** — toggle grid snap ([`SnapGrid`]).
//!
//! Interaction: left-press near an axis handle grabs that axis; drag transforms
//! the selected entity along/around it; release drops it. Grabbing a handle wins
//! over re-picking (the picker checks [`GizmoState::is_dragging`]).
//!
//! Draw/pick use the entity's `Transform.translation` as the pivot, which equals
//! its world position for the un-parented scene objects the editor manipulates.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::editor::{in_interactive_editor, Selection, SnapGrid};
use crate::OrbitCam;

/// Which transform the gizmo edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

/// A live drag: which axis, the entity, and the transform + hit-point captured at
/// grab time so the delta is measured from a stable origin.
#[derive(Clone, Copy, Debug)]
struct Drag {
    /// 0 = X, 1 = Y, 2 = Z.
    axis: usize,
    entity: Entity,
    start: Transform,
    /// World point where the grab ray met the drag plane (translate).
    start_hit: Vec3,
    /// Cursor at grab time (rotate/scale measure pixel deltas from here).
    start_cursor: Vec2,
}

/// Gizmo mode + the active drag. `is_dragging()` lets the picker yield to a handle
/// grab.
#[derive(Resource, Debug)]
pub struct GizmoState {
    pub mode: GizmoMode,
    drag: Option<Drag>,
}

impl Default for GizmoState {
    fn default() -> Self {
        Self {
            mode: GizmoMode::Translate,
            drag: None,
        }
    }
}

impl GizmoState {
    /// True while an axis handle is held — the picker skips re-selection then.
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }
}

pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut App) {
        // GizmoState is owned here; Selection + SnapGrid come from EditorPlugin.
        app.init_resource::<GizmoState>().add_systems(
            Update,
            (
                gizmo_hotkeys,
                gizmo_begin_drag,
                // After the picker so a fresh selection this frame can be dragged
                // immediately, and the begin→pick→update order is deterministic.
                gizmo_update_drag.after(crate::editor_camera::pick_selectable),
                draw_gizmo,
            )
                .run_if(in_interactive_editor),
        );
    }
}

/// Screen pixels within which a click counts as grabbing an axis handle.
const HANDLE_PX: f32 = 14.0;
/// Radians per pixel of horizontal drag in Rotate mode.
const ROTATE_SENS: f32 = 0.01;
/// Scale units per pixel of vertical drag in Scale mode.
const SCALE_SENS: f32 = 0.01;

/// 1/2/3 pick the mode; X toggles snap.
pub fn gizmo_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut gizmo: ResMut<GizmoState>,
    mut snap: ResMut<SnapGrid>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        gizmo.mode = GizmoMode::Translate;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        gizmo.mode = GizmoMode::Rotate;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        gizmo.mode = GizmoMode::Scale;
    }
    if keys.just_pressed(KeyCode::KeyX) {
        snap.enabled = !snap.enabled;
    }
}

/// Left-press: if the cursor is over an axis handle of the selected entity, begin
/// dragging that axis.
pub fn gizmo_begin_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
    selection: Res<Selection>,
    q_tf: Query<&Transform>,
    mut gizmo: ResMut<GizmoState>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    gizmo.drag = None;
    let Some(sel) = selection.entity else {
        return;
    };
    let Ok(tf) = q_tf.get(sel) else {
        return;
    };
    let origin = tf.translation;
    let Ok((cam, cam_gt)) = cam_q.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let len = gizmo_len(cam_gt.translation(), origin);
    let Ok(o_screen) = cam.world_to_viewport(cam_gt, origin) else {
        return;
    };

    // Nearest axis handle to the cursor, if any is within HANDLE_PX.
    let mut best: Option<(f32, usize)> = None;
    for axis in 0..3 {
        let tip = origin + axis_unit(axis) * len;
        if let Ok(tip_screen) = cam.world_to_viewport(cam_gt, tip) {
            let d = point_seg_dist(cursor, o_screen, tip_screen);
            if d < HANDLE_PX && best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, axis));
            }
        }
    }

    if let Some((_, axis)) = best {
        let view = cam_gt.affine().transform_vector3(Vec3::NEG_Z).normalize_or_zero();
        let normal = plane_normal(axis_unit(axis), view);
        let start_hit = cam
            .viewport_to_world(cam_gt, cursor)
            .ok()
            .and_then(|ray| ray_plane(ray.origin, ray.direction.as_vec3(), origin, normal))
            .unwrap_or(origin);
        gizmo.drag = Some(Drag {
            axis,
            entity: sel,
            start: *tf,
            start_hit,
            start_cursor: cursor,
        });
    }
}

/// While an axis is held, transform the entity; release ends the drag.
pub fn gizmo_update_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
    snap: Res<SnapGrid>,
    mut gizmo: ResMut<GizmoState>,
    mut q_tf: Query<&mut Transform>,
) {
    let Some(drag) = gizmo.drag else {
        return;
    };
    if !mouse.pressed(MouseButton::Left) {
        gizmo.drag = None;
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
    let Ok(mut tf) = q_tf.get_mut(drag.entity) else {
        gizmo.drag = None;
        return;
    };
    let axis_w = axis_unit(drag.axis);

    match gizmo.mode {
        GizmoMode::Translate => {
            let view = cam_gt.affine().transform_vector3(Vec3::NEG_Z).normalize_or_zero();
            let normal = plane_normal(axis_w, view);
            if let Ok(ray) = cam.viewport_to_world(cam_gt, cursor) {
                if let Some(hit) =
                    ray_plane(ray.origin, ray.direction.as_vec3(), drag.start.translation, normal)
                {
                    let mut d = (hit - drag.start_hit).dot(axis_w);
                    if snap.enabled {
                        d = snap_scalar(d, snap.translate_step);
                    }
                    tf.translation = drag.start.translation + axis_w * d;
                }
            }
        }
        GizmoMode::Rotate => {
            let mut ang = (cursor.x - drag.start_cursor.x) * ROTATE_SENS;
            if snap.enabled {
                ang = snap_scalar(ang, snap.rotate_step);
            }
            tf.rotation = drag.start.rotation * Quat::from_axis_angle(axis_w, ang);
            tf.translation = drag.start.translation;
            tf.scale = drag.start.scale;
        }
        GizmoMode::Scale => {
            // Drag up = grow. Scale the grabbed axis only; clamp so it stays positive.
            let mut ds = (drag.start_cursor.y - cursor.y) * SCALE_SENS;
            if snap.enabled {
                ds = snap_scalar(ds, snap.scale_step);
            }
            let base = get_comp(drag.start.scale, drag.axis);
            tf.scale = with_comp(drag.start.scale, drag.axis, (base + ds).max(0.05));
            tf.translation = drag.start.translation;
            tf.rotation = drag.start.rotation;
        }
    }
}

/// Draw the three axis handles at the selected entity (X red, Y green, Z blue),
/// sized to stay roughly constant on screen. Scale mode nudges them longer as a
/// mode cue.
pub fn draw_gizmo(
    mut gizmos: Gizmos,
    selection: Res<Selection>,
    state: Res<GizmoState>,
    cam_q: Query<&GlobalTransform, With<OrbitCam>>,
    q_tf: Query<&Transform>,
) {
    let Some(sel) = selection.entity else {
        return;
    };
    let Ok(tf) = q_tf.get(sel) else {
        return;
    };
    let Ok(cam_gt) = cam_q.single() else {
        return;
    };
    let origin = tf.translation;
    let hint = if state.mode == GizmoMode::Scale { 1.15 } else { 1.0 };
    let len = gizmo_len(cam_gt.translation(), origin) * hint;
    let cols = [
        Color::srgb(0.90, 0.25, 0.25),
        Color::srgb(0.35, 0.85, 0.35),
        Color::srgb(0.30, 0.55, 0.95),
    ];
    for axis in 0..3 {
        gizmos.arrow(origin, origin + axis_unit(axis) * len, cols[axis]);
    }
}

// ---------------------------------------------------------------------------
// Pure helpers — unit-tested below, no GPU needed.
// ---------------------------------------------------------------------------

/// World-space unit vector for an axis index (0=X, 1=Y, 2=Z).
pub(crate) fn axis_unit(axis: usize) -> Vec3 {
    match axis {
        0 => Vec3::X,
        1 => Vec3::Y,
        _ => Vec3::Z,
    }
}

fn get_comp(v: Vec3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn with_comp(v: Vec3, axis: usize, val: f32) -> Vec3 {
    match axis {
        0 => Vec3::new(val, v.y, v.z),
        1 => Vec3::new(v.x, val, v.z),
        _ => Vec3::new(v.x, v.y, val),
    }
}

/// Round `v` to the nearest multiple of `step` (no-op when `step <= 0`).
pub(crate) fn snap_scalar(v: f32, step: f32) -> f32 {
    if step <= 0.0 {
        v
    } else {
        (v / step).round() * step
    }
}

/// Ray vs plane; `None` if parallel or the hit is behind the ray.
pub(crate) fn ray_plane(ro: Vec3, rd: Vec3, plane_p: Vec3, plane_n: Vec3) -> Option<Vec3> {
    let denom = rd.dot(plane_n);
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = (plane_p - ro).dot(plane_n) / denom;
    if t < 0.0 {
        return None;
    }
    Some(ro + rd * t)
}

/// The plane that contains `axis` and most faces the camera — the surface a
/// translate drag slides along. Falls back to any perpendicular when the axis
/// points nearly straight at/away from the camera.
pub(crate) fn plane_normal(axis: Vec3, view: Vec3) -> Vec3 {
    let n = axis.cross(view).cross(axis);
    if n.length_squared() < 1e-6 {
        axis.any_orthonormal_vector()
    } else {
        n.normalize()
    }
}

/// Shortest distance from point `p` to segment `a`–`b` (all 2D, screen space).
pub(crate) fn point_seg_dist(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-6 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Handle length in world units, scaled by camera distance so the gizmo keeps a
/// roughly constant on-screen size.
fn gizmo_len(cam_pos: Vec3, origin: Vec3) -> f32 {
    ((cam_pos - origin).length() * 0.15).max(0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_rounds_to_step_and_passes_through_when_off() {
        assert_eq!(snap_scalar(1.2, 1.0), 1.0);
        assert_eq!(snap_scalar(1.6, 1.0), 2.0);
        assert_eq!(snap_scalar(2.3, 0.5), 2.5);
        assert_eq!(snap_scalar(1.234, 0.0), 1.234); // step<=0 → no snap
    }

    #[test]
    fn ray_plane_hits_xy_plane_from_z() {
        // Ray from +Z toward -Z hits the z=0 plane at the origin.
        let hit = ray_plane(Vec3::new(0.0, 0.0, 5.0), Vec3::NEG_Z, Vec3::ZERO, Vec3::Z);
        assert!(hit.is_some());
        assert!(hit.unwrap().abs_diff_eq(Vec3::ZERO, 1e-4));
    }

    #[test]
    fn ray_plane_parallel_misses() {
        // Ray travelling along the plane never meets it.
        let hit = ray_plane(Vec3::new(0.0, 0.0, 5.0), Vec3::X, Vec3::ZERO, Vec3::Z);
        assert!(hit.is_none());
    }

    #[test]
    fn plane_normal_is_perpendicular_to_axis() {
        let n = plane_normal(Vec3::X, Vec3::new(0.0, 0.0, -1.0));
        assert!(n.dot(Vec3::X).abs() < 1e-5, "normal should be ⟂ to the axis");
        assert!((n.length() - 1.0).abs() < 1e-4, "normal should be unit length");
    }

    #[test]
    fn plane_normal_degenerate_axis_toward_camera_still_unit() {
        // Axis pointing straight down the view direction → fallback perpendicular.
        let n = plane_normal(Vec3::Z, Vec3::new(0.0, 0.0, -1.0));
        assert!((n.length() - 1.0).abs() < 1e-4);
        assert!(n.dot(Vec3::Z).abs() < 1e-4);
    }

    #[test]
    fn point_segment_distance_basics() {
        // Point directly above the segment midpoint.
        let d = point_seg_dist(Vec2::new(1.0, 2.0), Vec2::new(0.0, 0.0), Vec2::new(2.0, 0.0));
        assert!((d - 2.0).abs() < 1e-4);
        // Point past an endpoint clamps to that endpoint.
        let d2 = point_seg_dist(Vec2::new(5.0, 0.0), Vec2::new(0.0, 0.0), Vec2::new(2.0, 0.0));
        assert!((d2 - 3.0).abs() < 1e-4);
    }

    #[test]
    fn axis_component_helpers_round_trip() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(get_comp(v, 1), 2.0);
        assert_eq!(with_comp(v, 2, 9.0), Vec3::new(1.0, 2.0, 9.0));
        assert_eq!(axis_unit(0), Vec3::X);
    }
}
