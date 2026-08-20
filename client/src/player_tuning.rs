//! Third-person player controller tuning — single source of truth for feel.
//!
//! Everything that makes the avatar and the orbit camera "smooth" lives here:
//! movement ramps, turn curves, camera spring/lag, coyote time, jump buffer and
//! the small running FOV bump.  Systems read these constants and helpers instead
//! of hard-coding numbers in `main.rs` so tuning rounds are one-file edits.

use bevy::prelude::*;

// -----------------------------------------------------------------------------
// Movement
// -----------------------------------------------------------------------------

/// Normal walk speed (blocks/s).
pub(crate) const WALK_SPEED: f32 = 6.0;
/// Sprint speed when Ctrl is held (blocks/s).
pub(crate) const SPRINT_SPEED: f32 = 10.0;

/// How fast the avatar reaches its target horizontal speed on the ground.
/// Lower = heavier/smoother; too low feels muddy.
pub(crate) const ACCEL_GROUND: f32 = 60.0;
/// How fast the avatar sheds speed when input is released on the ground.
pub(crate) const DECEL_GROUND: f32 = 50.0;
/// Air acceleration is tiny — you can nudge trajectory, not whip it around.
pub(crate) const AIR_ACCEL: f32 = 8.0;
/// Air deceleration/friction (mostly to cap residual drift).
pub(crate) const AIR_DECEL: f32 = 2.0;

// -----------------------------------------------------------------------------
// Turning
// -----------------------------------------------------------------------------

/// Max rotation speed when turning toward the movement direction (rad/s).
pub(crate) const TURN_RATE: f32 = 8.0;
/// Rotation speed when standing still and turning to face the camera (rad/s).
pub(crate) const TURN_RATE_IDLE: f32 = 4.5;
/// Exponent on the turn ease curve (>1 = gentle start, sharp finish).
pub(crate) const TURN_EASE: f32 = 2.0;

// -----------------------------------------------------------------------------
// Camera boom
// -----------------------------------------------------------------------------

pub(crate) const BOOM_DIST: f32 = 6.5; // how far the camera sits behind the avatar (max)
pub(crate) const BOOM_MARGIN: f32 = 0.9; // keep the camera this far off a wall it pulls up to
pub(crate) const BOOM_RADIUS: f32 = 0.7; // lens disc radius for side-collision tests
pub(crate) const PIVOT_UP: f32 = 0.35; // lift the look-pivot a touch above the eye for framing
pub(crate) const PITCH_MIN: f32 = -1.35; // clamp: don't roll under the avatar
pub(crate) const PITCH_MAX: f32 = 1.20; // clamp: don't roll over the top

// -----------------------------------------------------------------------------
// Camera spring / smoothing
// -----------------------------------------------------------------------------

/// Stiffness of the damped spring that drives the camera to its target position.
pub(crate) const CAM_SPRING_STIFFNESS: f32 = 120.0;
/// Damping of the same spring.  Around 2*sqrt(stiffness) is near-critical;
/// we keep it slightly under-damped so it settles with a soft whisper, not a dead stop.
pub(crate) const CAM_SPRING_DAMP: f32 = 18.0;
/// How fast the camera rotation catches the orbit orientation (exponential decay, 1/s).
pub(crate) const CAM_ROT_CATCHUP: f32 = 6.0;

// -----------------------------------------------------------------------------
// Running feel
// -----------------------------------------------------------------------------

/// Extra vertical FOV (radians) at full sprint.
pub(crate) const RUN_FOV_BUMP: f32 = 0.10472; // 6 degrees
/// Speed at which the FOV bump begins to ramp in.
pub(crate) const RUN_FOV_SPEED_LO: f32 = 4.0;
/// Speed at which the FOV bump reaches full strength.
pub(crate) const RUN_FOV_SPEED_HI: f32 = 8.0;
/// Base vertical FOV (radians) for the gameplay camera.
pub(crate) const CAM_BASE_FOV: f32 = std::f32::consts::FRAC_PI_3; // 60 degrees

/// How far the camera lags behind the avatar at full sprint (blocks).
pub(crate) const RUN_CAM_LAG_DIST: f32 = 0.55;
/// Speed at which the lag begins to ramp in.
pub(crate) const RUN_CAM_LAG_SPEED_LO: f32 = 3.5;
/// Speed at which the lag reaches full strength.
pub(crate) const RUN_CAM_LAG_SPEED_HI: f32 = 8.0;

// -----------------------------------------------------------------------------
// Coyote time + jump buffer
// -----------------------------------------------------------------------------

/// Grace period after leaving ground where a jump still counts.
pub(crate) const COYOTE_TIME: f32 = 0.10;
/// Grace period before landing where an early Space press is remembered.
pub(crate) const JUMP_BUFFER_TIME: f32 = 0.12;

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

/// Rotate `cur` toward `target` by at most `max_step` radians, taking the short
/// way round the circle.  This is the same logic the old `main::turn_toward`
/// used, hoisted here so both controller and camera lanes can share it.
pub(crate) fn turn_toward(cur: f32, target: f32, max_step: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut d = (target - cur).rem_euclid(TAU);
    if d > PI {
        d -= TAU;
    }
    cur + d.clamp(-max_step, max_step)
}

/// Same as `turn_toward` but with an ease curve on the step size so the turn
/// starts gentle and tightens as it closes.
pub(crate) fn smooth_turn_toward(cur: f32, target: f32, rate: f32, dt: f32, ease: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut d = (target - cur).rem_euclid(TAU);
    if d > PI {
        d -= TAU;
    }
    let step = (rate * dt).min(PI);
    let t = (d / step).clamp(-1.0, 1.0);
    // apply ease to the normalized progress, then map back to the same step size
    let eased_t = t.signum() * t.abs().powf(ease);
    cur + step * eased_t
}

/// Exponential decay from `a` to `b` with time-constant `decay` (1/s).
pub(crate) fn exp_decay(a: f32, b: f32, decay: f32, dt: f32) -> f32 {
    b + (a - b) * (-decay * dt).exp()
}

/// Integrate a damped spring for one frame.  `velocity` is carried across frames.
pub(crate) fn damped_spring(
    current: Vec3,
    target: Vec3,
    velocity: &mut Vec3,
    stiffness: f32,
    damping: f32,
    dt: f32,
) -> Vec3 {
    let dt = dt.clamp(0.0, 1.0 / 15.0);
    let force = (target - current) * stiffness;
    *velocity += (force - *velocity * damping) * dt;
    current + *velocity * dt
}

/// Normalised smoothstep from edge0 to edge1.
pub(crate) fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Accelerate `current` horizontal velocity toward `target` using `accel` and
/// `dt`.  The acceleration is applied along the direction to the target, never
/// overshooting, so releasing a key doesn't reverse velocity.
pub(crate) fn accel_toward(current: Vec3, target: Vec3, accel: f32, dt: f32) -> Vec3 {
    let diff = target - current;
    let max_step = accel * dt;
    if diff.length_squared() <= max_step * max_step {
        target
    } else {
        current + diff.normalize_or_zero() * max_step
    }
}

/// Apply friction/decay to horizontal velocity when there is no input.
pub(crate) fn apply_friction(current: Vec3, decel: f32, dt: f32) -> Vec3 {
    let max_step = decel * dt;
    if current.length_squared() <= max_step * max_step {
        Vec3::ZERO
    } else {
        current - current.normalize_or_zero() * max_step
    }
}
