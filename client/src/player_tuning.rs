//! Player controller tuning constants — single source of truth for movement,
//! camera, jump, and footstep feel. All values are in engine units (metres,
//! seconds, radians) unless noted.

// -----------------------------------------------------------------------------
// Movement
// -----------------------------------------------------------------------------

/// Maximum grounded walk speed (m/s).
pub const WALK_SPEED_MAX: f32 = 6.0;
/// Maximum grounded run speed (m/s). Hold Left-Ctrl to run.
pub const RUN_SPEED_MAX: f32 = 10.0;

/// Time to accelerate from standstill to [`WALK_SPEED_MAX`] (seconds).
pub const WALK_ACCEL_TIME: f32 = 0.25;
/// Time to decelerate from [`WALK_SPEED_MAX`] to standstill (seconds).
pub const WALK_DECEL_TIME: f32 = 0.20;
/// Time to accelerate from standstill to [`RUN_SPEED_MAX`] (seconds).
pub const RUN_ACCEL_TIME: f32 = 0.30;
/// Time to decelerate from [`RUN_SPEED_MAX`] to standstill (seconds).
pub const RUN_DECEL_TIME: f32 = 0.25;

/// Derived horizontal acceleration while walking (m/s²).
pub const WALK_ACCEL: f32 = WALK_SPEED_MAX / WALK_ACCEL_TIME;
/// Derived horizontal deceleration while walking (m/s²).
pub const WALK_DECEL: f32 = WALK_SPEED_MAX / WALK_DECEL_TIME;
/// Derived horizontal acceleration while running (m/s²).
pub const RUN_ACCEL: f32 = RUN_SPEED_MAX / RUN_ACCEL_TIME;
/// Derived horizontal deceleration while running (m/s²).
pub const RUN_DECEL: f32 = RUN_SPEED_MAX / RUN_DECEL_TIME;

/// Fraction of ground acceleration/deceleration available while airborne.
/// Lower = less air control, heavier feel.
pub const AIR_ACCEL_FACTOR: f32 = 0.3;

/// How fast the avatar's yaw turns to face the input direction (rad/s).
pub const AVATAR_TURN_RATE: f32 = 12.0;

// -----------------------------------------------------------------------------
// Jump
// -----------------------------------------------------------------------------

/// Initial upward velocity when jumping (m/s). With default gravity this is
/// roughly a 1.4-block hop.
pub const JUMP_SPEED: f32 = 9.0;
/// Gravity applied while rising (m/s²). Lower than fall gravity = floatier apex.
pub const GRAVITY_RISE: f32 = 28.0;
/// Gravity applied while falling (m/s²). Higher than rise gravity = snappier land.
pub const GRAVITY_FALL: f32 = 42.0;
/// Maximum fall speed (m/s).
pub const TERMINAL_VELOCITY: f32 = 55.0;
/// Coyote time: you can still jump this many seconds after leaving a ledge.
pub const COYOTE_TIME: f32 = 0.12;
/// Jump buffer: a jump press this many seconds before landing still triggers.
pub const JUMP_BUFFER_TIME: f32 = 0.15;

// -----------------------------------------------------------------------------
// Landing squash
// -----------------------------------------------------------------------------

/// How long the landing squash deformation lasts (seconds).
pub const LAND_SQUASH_DURATION: f32 = 0.12;
/// Y-axis compression during landing squash (unitless, 1.0 = no change).
pub const LAND_SQUASH_SCALE_Y: f32 = 0.85;
/// X/Z-axis expansion during landing squash (unitless). Slightly above 1.0 so
/// the silhouette keeps roughly the same volume while compressing vertically.
pub const LAND_SQUASH_SCALE_XZ: f32 = 1.05;

// -----------------------------------------------------------------------------
// Body dimensions
// -----------------------------------------------------------------------------

/// Half of the player's 0.6 m-wide footprint (m).
pub const PLAYER_HALF_W: f32 = 0.3;
/// Total player height from feet to crown (m).
pub const PLAYER_HEIGHT: f32 = 1.8;
/// Eye/camera height from feet (m). Head clearance = PLAYER_HEIGHT - EYE_HEIGHT.
pub const EYE_HEIGHT: f32 = 1.62;
/// Maximum ledge height the player auto-steps while walking (m).
pub const STEP_HEIGHT: f32 = 1.0;
/// Extra head-room probed above a ledge before the auto-step commits (m).
pub const STEP_CLEAR: f32 = 0.2;

// -----------------------------------------------------------------------------
// Third-person orbit camera
// -----------------------------------------------------------------------------

/// Maximum boom distance behind the avatar (m).
pub const BOOM_DIST: f32 = 6.5;
/// Keep the camera this far off a wall it pulls up to (m).
pub const BOOM_MARGIN: f32 = 0.9;
/// Treat the lens as a disc this wide so corners and parallel faces pull it in (m).
pub const BOOM_RADIUS: f32 = 0.7;
/// Lift the look-pivot above the eye for framing (m).
pub const PIVOT_UP: f32 = 0.35;
/// Clamp: do not roll the camera under the avatar (rad).
pub const PITCH_MIN: f32 = -1.35;
/// Clamp: do not roll the camera over the top (rad).
pub const PITCH_MAX: f32 = 1.20;

/// Camera spring frequency (Hz). Higher = tighter follow, lower = more lag.
pub const CAM_SPRING_FREQ: f32 = 3.0;
/// Camera spring damping ratio. 1.0 = critically damped; <1.0 = bouncy.
pub const CAM_SPRING_DAMP: f32 = 0.75;

/// Look-ahead lead time: how many seconds of travel-at-current-speed the follow
/// pivot is pushed ahead of the avatar, so the camera frames where you're going
/// instead of trailing dead-centre on where you are.
pub const CAM_LOOKAHEAD_TIME: f32 = 0.15;
/// Cap on the look-ahead offset regardless of speed (m).
pub const CAM_LOOKAHEAD_MAX: f32 = 1.4;
/// If the follow pivot jumps further than this in one frame (teleport, respawn,
/// scene warp), snap the spring instead of swooping the camera across the map.
pub const CAM_SNAP_DIST: f32 = 4.0;

/// Field of view while walking (degrees).
pub const CAM_FOV_WALK: f32 = 60.0;
/// Field of view while running (degrees).
pub const CAM_FOV_RUN: f32 = 65.0;
/// How quickly FOV transitions between walk and run (1/s).
pub const CAM_FOV_LERP_SPEED: f32 = 4.0;

// -----------------------------------------------------------------------------
// Footsteps
// -----------------------------------------------------------------------------

/// Seconds between footsteps while walking.
pub const FOOTSTEP_INTERVAL_WALK: f32 = 0.45;
/// Seconds between footsteps while running.
pub const FOOTSTEP_INTERVAL_RUN: f32 = 0.30;
/// Minimum horizontal speed to emit footsteps (m/s). Stops shuffle noise.
pub const FOOTSTEP_MIN_SPEED: f32 = 0.3;
