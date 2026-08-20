// _sun_combo_probe.pending.rs — the combo_probe edit for client/src/combat.rs.
// Sun's scratch file: paste these four hunks into combat.rs the moment the
// target-sun build goes green. Do NOT edit combat.rs while a build is running.
//
// Purpose: drive a CLEAN L1→L2→L3→finisher chain through the real
// ButtonInput<KeyCode>, so the combo video shows the actual 3-hit + finisher.
// The existing feel_probe mash (Light/Dodge/Heavy on step%4) can never chain
// because Dodge resets combo to 0.
//
// Gated behind VOXELFORGE_COMBO_PROBE=1; normal --play and feel_probe are
// untouched.

// ===========================================================================
// HUNK 1 — constants. Insert after `const PROBE_HEAVY_HOLD: f32 = 0.30;`
//          (currently combat.rs:3369).
// ===========================================================================
/// One scripted light step per this many seconds — past the cancel point
/// (`LIGHT_TIME * COMBO_CANCEL_FROM` ≈ 0.175s) but inside the swing (0.35s), so
/// each press cancels the previous into the next chain step.
const COMBO_LIGHT_GAP: f32 = 0.28;
/// How long the finisher heavy tap is held (< `CHARGE_HOLD` 0.60s, so it reads
/// as a finisher tap, never a charge).
const COMBO_HEAVY_TAP: f32 = 0.08;
/// Wall-clock after the finisher step before the next chain — long enough for
/// the finisher (`FINISHER_TIME` 0.95s) + combo cooldown (`COMBO_CD` 0.35s).
const COMBO_FINISHER_REST: f32 = 1.6;

// ===========================================================================
// HUNK 2 — FeelProbe struct: add `combo_mode` after `pub enabled: bool,`
//          (currently combat.rs:3374).
// ===========================================================================
    /// Drive a clean L1→L2→L3→finisher chain (VOXELFORGE_COMBO_PROBE=1) instead
    /// of the light/dodge/heavy mash.
    pub combo_mode: bool,

// ===========================================================================
// HUNK 3 — plugin build (CombatFeelPlugin::build).
//          At combat.rs:4095 add the combo_probe env read; at :4127 wire it.
// ===========================================================================
    // AFTER:  let probe = std::env::var("VOXELFORGE_FEEL_PROBE").is_ok();
    let combo_probe = std::env::var("VOXELFORGE_COMBO_PROBE").is_ok();
    // AND change the log line so a combo run turns the FEEL log on too:
    //   let log = probe || combo_probe || std::env::var("VOXELFORGE_FEEL_LOG").is_ok();

    // REPLACE:  .insert_resource(FeelProbe { enabled: probe, ..default() })
    // WITH:
    .insert_resource(FeelProbe { enabled: probe || combo_probe, combo_mode: combo_probe, ..default() })

// ===========================================================================
// HUNK 4 — feel_probe cadence. Replace the block from
//          `if t < probe.next_t { return; }` through the closing of the
//          `match probe.step % 4 { ... }` (currently combat.rs:3484-3503).
// ===========================================================================
    if t < probe.next_t {
        return;
    }
    probe.step += 1;
    if probe.combo_mode {
        // Clean showcase: L1 → L2 → L3 → finisher → rest → repeat. Each light
        // press lands inside the cancel window (0.175s..0.35s) so it chains; the
        // finisher is a tap (heavy_pressed rising edge, < CHARGE_HOLD) that
        // commits the 4th step. The rest lets finisher (0.95s) + cooldown
        // (0.35s) close before the next chain.
        match probe.step % 5 {
            1 | 2 | 3 => {
                keys.reset(KeyCode::KeyX);
                keys.press(KeyCode::KeyX);
                probe.next_t = t + COMBO_LIGHT_GAP;
            }
            4 => {
                keys.reset(KeyCode::KeyC);
                keys.press(KeyCode::KeyC);
                probe.heavy_until = t + COMBO_HEAVY_TAP;
                probe.next_t = t + COMBO_FINISHER_REST;
            }
            _ => {
                probe.next_t = t + COMBO_LIGHT_GAP;
            }
        }
    } else {
        probe.next_t = t + PROBE_STEP;
        match probe.step % 4 {
            1 | 0 => {
                // Light: reset-then-press so `just_pressed` fires a fresh rising edge.
                keys.reset(KeyCode::KeyX);
                keys.press(KeyCode::KeyX);
            }
            2 => {
                keys.reset(KeyCode::Space);
                keys.press(KeyCode::Space);
            }
            _ => {
                keys.press(KeyCode::KeyC);
                probe.heavy_until = t + PROBE_HEAVY_HOLD;
            }
        }
    }
