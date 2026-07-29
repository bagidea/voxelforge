//! Editor camera + raycast picking (Poppy — took over Shiba's `editor_cam` socket).
//!
//! [`EditorCameraPlugin`] gives the build mode a free camera that never fights
//! the gameplay `fly_camera`: it drives the same `OrbitCam` entity, but only in
//! an *interactive* editor session ([`crate::editor::in_interactive_editor`]),
//! while `main` suppresses `fly_camera` for exactly that case. Scripted / headless
//! runs stay on `fly_camera`, so the walk/edit/map-save screenshot proofs are
//! untouched.
//!
//! Controls (Editor state):
//!   * **Middle-drag** — orbit yaw/pitch around the focus point.
//!   * **Shift + Middle-drag** — pan the focus in the camera plane.
//!   * **Wheel** — zoom (orbit) / adjust fly speed (fly).
//!   * **F** — toggle orbit ⇆ fly. In fly, `WASD` + `Space`/`Ctrl` move, hold
//!     middle-drag to look, `Shift` sprints.
//!
//! Left and right clicks are deliberately left free for voxel editing
//! (`editor::editor_edit`: left = place the selected block, right = break), which
//! is why the camera orbits on the **middle** button instead of the usual
//! right-drag. Left-click also raycasts into the scene and writes the nearest
//! [`Selectable`] to [`Selection`] (skipped while a gizmo handle is being
//! dragged, so grabbing a handle never re-selects). The cursor stays free (no
//! grab) so screen-space picking + gizmo interaction work.

use bevy::camera::primitives::Aabb;
use bevy::ecs::message::MessageReader;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::editor::{in_interactive_editor, not_scripted, ray_aabb, AppState, Selectable, Selection};
use crate::gizmo::GizmoState;
use crate::OrbitCam;

/// The editor camera's state. Orbit/fly both keep `focus` up to date so toggling
/// between them is seamless. Persisted as a resource; the controller drives the
/// `OrbitCam` `Transform` from it each frame.
#[derive(Resource, Debug, Clone)]
pub struct EditorCamera {
    /// Point the orbit spins around / the fly camera keeps ahead of it.
    pub focus: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    /// Orbit boom length (also the fly camera's focus lead distance).
    pub distance: f32,
    /// false = orbit around `focus`; true = free fly (`WASD` + look).
    pub fly: bool,
    /// Units/sec in fly mode (wheel adjusts it).
    pub fly_speed: f32,
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            yaw: 0.6,
            pitch: -0.5,
            distance: 40.0,
            fly: false,
            fly_speed: 30.0,
        }
    }
}

pub struct EditorCameraPlugin;

impl Plugin for EditorCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorCamera>()
            // Seed the camera from wherever the view currently is, and free the
            // cursor — but only for interactive boots (OnEnter also fires for the
            // default Editor state on a scripted launch).
            .add_systems(
                OnEnter(AppState::Editor),
                enter_editor_camera.run_if(not_scripted),
            )
            .add_systems(
                Update,
                (
                    editor_camera_control,
                    // Runs after the gizmo decides whether a handle was grabbed,
                    // so clicking a handle never also re-picks (see `pick_selectable`).
                    pick_selectable.after(crate::gizmo::gizmo_begin_drag),
                )
                    .run_if(in_interactive_editor),
            );
    }
}

/// yaw (about world +Y) then pitch (about local +X) — matches the play-mode
/// orbit convention so the handoff into the editor is continuous.
fn cam_rot(yaw: f32, pitch: f32) -> Quat {
    Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch)
}

/// On entering an interactive editor session: derive yaw/pitch/focus from the
/// camera's current transform (so it doesn't snap) and free the OS cursor.
fn enter_editor_camera(
    mut ec: ResMut<EditorCamera>,
    cam_q: Query<&Transform, With<OrbitCam>>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(tf) = cam_q.single() {
        let fwd: Vec3 = *tf.forward();
        // Inverse of `cam_rot`: fwd = (-cosθ·sinψ, sinθ, -cosθ·cosψ).
        ec.pitch = fwd.y.clamp(-1.0, 1.0).asin();
        ec.yaw = (-fwd.x).atan2(-fwd.z);
        ec.focus = tf.translation + fwd * ec.distance;
    }
    if let Ok(mut cursor) = cursors.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Orbit / pan / zoom / fly. Reads accumulated mouse motion + wheel, writes the
/// `OrbitCam` transform from [`EditorCamera`].
fn editor_camera_control(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut ec: ResMut<EditorCamera>,
    mut cam_q: Query<&mut Transform, With<OrbitCam>>,
) {
    let Ok(mut tf) = cam_q.single_mut() else {
        motion.clear();
        wheel.clear();
        return;
    };

    let dt = time.delta_secs();
    let mut md = Vec2::ZERO;
    for ev in motion.read() {
        md += ev.delta;
    }
    let mut scroll = 0.0f32;
    for ev in wheel.read() {
        scroll += ev.y;
    }

    if keys.just_pressed(KeyCode::KeyF) {
        ec.fly = !ec.fly;
    }

    const ORBIT_SENS: f32 = 0.005;

    if ec.fly {
        // ---- FLY: look on middle-drag, WASD move, wheel = speed -------------
        // (right-click is reserved for voxel break — see editor::editor_edit)
        if mouse.pressed(MouseButton::Middle) {
            ec.yaw -= md.x * ORBIT_SENS;
            ec.pitch = (ec.pitch - md.y * ORBIT_SENS).clamp(-1.54, 1.54);
        }
        let rot = cam_rot(ec.yaw, ec.pitch);
        let fwd = rot * Vec3::NEG_Z;
        let right = rot * Vec3::X;
        if scroll != 0.0 {
            ec.fly_speed = (ec.fly_speed * (1.0 + scroll * 0.1)).clamp(2.0, 500.0);
        }
        let mut wish = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            wish += fwd;
        }
        if keys.pressed(KeyCode::KeyS) {
            wish -= fwd;
        }
        if keys.pressed(KeyCode::KeyD) {
            wish += right;
        }
        if keys.pressed(KeyCode::KeyA) {
            wish -= right;
        }
        if keys.pressed(KeyCode::Space) {
            wish += Vec3::Y;
        }
        if keys.pressed(KeyCode::ControlLeft) {
            wish -= Vec3::Y;
        }
        let speed = ec.fly_speed * if keys.pressed(KeyCode::ShiftLeft) { 3.0 } else { 1.0 };
        if wish != Vec3::ZERO {
            tf.translation += wish.normalize() * speed * dt;
        }
        tf.rotation = rot;
        ec.focus = tf.translation + fwd * ec.distance;
    } else {
        // ---- ORBIT: middle = orbit, shift+middle = pan, wheel = zoom --------
        // (left/right are reserved for voxel place/break — see editor::editor_edit)
        if mouse.pressed(MouseButton::Middle) {
            let rot = cam_rot(ec.yaw, ec.pitch);
            let right = rot * Vec3::X;
            let up = rot * Vec3::Y;
            if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                // Pan scales with distance so it feels the same near or far.
                // Snapshot `distance` into a local first: `ec.focus += … ec.distance
                // …` would otherwise borrow `ec` shared and mutable at once.
                let dist = ec.distance;
                ec.focus += (-right * md.x + up * md.y) * (dist * 0.0015);
            } else {
                ec.yaw -= md.x * ORBIT_SENS;
                ec.pitch = (ec.pitch - md.y * ORBIT_SENS).clamp(-1.54, 1.54);
            }
        }
        let rot = cam_rot(ec.yaw, ec.pitch);
        let fwd = rot * Vec3::NEG_Z;
        if scroll != 0.0 {
            ec.distance = (ec.distance * (1.0 - scroll * 0.1)).clamp(1.0, 800.0);
        }
        tf.translation = ec.focus - fwd * ec.distance;
        tf.rotation = rot;
    }
}

/// Left-click raycast: pick the nearest [`Selectable`] under the cursor and write
/// it to [`Selection`]. A click on empty space clears the selection. Skipped
/// while a gizmo handle is being dragged so the handle grab wins over re-picking.
pub fn pick_selectable(
    windows: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
    mouse: Res<ButtonInput<MouseButton>>,
    gizmo: Res<GizmoState>,
    selectables: Query<(Entity, &GlobalTransform, &Aabb), With<Selectable>>,
    mut selection: ResMut<Selection>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if gizmo.is_dragging() {
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
    let Ok(ray) = cam.viewport_to_world(cam_gt, cursor) else {
        return;
    };
    let ro = ray.origin;
    let rd = ray.direction.as_vec3();

    let mut best: Option<(f32, Entity)> = None;
    for (entity, gt, aabb) in &selectables {
        // Transform the ray into the entity's local space (affine inverse) and
        // test against its local mesh Aabb. `t` is preserved under the affine,
        // so the smallest `t` across entities is the nearest hit.
        let inv = gt.affine().inverse();
        let local_o = inv.transform_point3(ro);
        let local_d = inv.transform_vector3(rd);
        let c = Vec3::from(aabb.center);
        let h = Vec3::from(aabb.half_extents);
        if let Some(t) = ray_aabb(local_o, local_d, c - h, c + h) {
            if best.map_or(true, |(bt, _)| t < bt) {
                best = Some((t, entity));
            }
        }
    }
    selection.entity = best.map(|(_, e)| e);
}
