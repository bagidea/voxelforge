//! Out-of-order walk-throughs of the Act-1 loop — pure, no Bevy.
//!
//! `quest_demo` walks Act 1 the way the script imagines it: campfire, gate,
//! Maren, east, Garren, build, lore, sigil, gate, out. Every objective is
//! completed in list order, so it exercises exactly the case that never broke.
//! The two ways the loop actually dead-ended (`fb71e4c`) both need the player to
//! do something *early*, and neither is reachable on that route:
//!
//! * **`zone_early`** — stand in `guard_post_east` before Maren hands out q3.
//!   `check_area_triggers` used to evaluate `reach_zone` objectives inside a
//!   once-only `Local<HashSet>` "entered" branch, so the region was spent on that
//!   first visit and `o1_east` could never be scored afterwards — q3, q4 and q5
//!   with it.
//! * **`kill_early`** — put Garren down while `o1_east` is still owed.
//!   `check_kill_triggers` used to credit only `objectives[current_objective]`,
//!   which is `o1_east` (a `reach_zone`) at that moment, so the kill was dropped
//!   — and a corpse despawns the frame after it dies, so there is no second one
//!   to offer.
//!
//! Selected with `VOXELFORGE_QUEST_CHAOS=zone_early|kill_early`; `quest_demo`
//! prints `QUEST_CHAOS mode=<label>` on its first frame so a log can never be
//! mistaken for the ordinary run.
//!
//! Everything here is geometry and a mode flag — no Bevy — so the claims the
//! routes rest on (the ambush stand really is *outside* the region; the detour
//! really steps *inside* it) are checked headlessly against the shipped
//! `assets/story/act1.json`:
//!
//! ```text
//! rustc --edition 2021 --test client/src/quest_chaos.rs \
//!   --extern serde_json=target-flamingo/release/deps/libserde_json-<hash>.rlib \
//!   -L dependency=target-flamingo/release/deps -o _probe/quest_chaos_test.exe
//! ./_probe/quest_chaos_test.exe --nocapture
//! ```

/// Which out-of-order walk-through the scripted demo should take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Chaos {
    /// The ordinary in-order route (`GATE_ROUTE` in `quest.rs`).
    #[default]
    None,
    /// Step into `guard_post_east` before q3 exists.
    ZoneEarly,
    /// Kill Garren while `o1_east` is still outstanding.
    KillEarly,
}

impl Chaos {
    /// Parse the `VOXELFORGE_QUEST_CHAOS` value. Anything unrecognised is
    /// [`Chaos::None`] — a typo must degrade to the ordinary run, never to a
    /// half-selected scenario that grades itself against the wrong contract.
    pub fn parse(raw: Option<&str>) -> Chaos {
        match raw.map(str::trim).unwrap_or("") {
            "zone_early" | "zone" => Chaos::ZoneEarly,
            "kill_early" | "kill" => Chaos::KillEarly,
            _ => Chaos::None,
        }
    }

    pub fn from_env() -> Chaos {
        Chaos::parse(std::env::var(ENV_VAR).ok().as_deref())
    }

    /// What `quest_demo` prints in its `QUEST_CHAOS mode=` banner, and what the
    /// runtime driver greps for to prove the run really was the scenario it
    /// grades.
    pub fn label(self) -> &'static str {
        match self {
            Chaos::None => "none",
            Chaos::ZoneEarly => "zone_early",
            Chaos::KillEarly => "kill_early",
        }
    }

    pub fn zone_early(self) -> bool { self == Chaos::ZoneEarly }
    pub fn kill_early(self) -> bool { self == Chaos::KillEarly }
}

/// The env lever. Named here so the driver script and the engine cannot drift.
pub const ENV_VAR: &str = "VOXELFORGE_QUEST_CHAOS";

/// The lane east of the village that is clear ground (top y=2) from x=30 to
/// x=52 — the one `quest_demo` phase 2 already walks. `quest.rs`'s
/// `POST_LANE_Z` is defined as this constant, so there is one number.
pub const LANE_Z: f32 = 8.5;

/// The x the demo walks to on the gate square, and the foot of the x≈32 ramp.
/// The gate square is a two-block plateau whose only climbable side is that
/// ramp (z 12-15), so every return to Maren has to come back through it —
/// dropping south off the plateau is free, climbing back north is not.
const GATE_X: f32 = 32.5;
const GATE_SQUARE_Z: f32 = 6.4;
const RAMP_FOOT_Z: f32 = 16.5;

/// How far east the detour walks before turning around. Inside
/// `guard_post_east` (x 48-56) with margin, and the same stop `quest_demo`
/// phase 2 uses.
const DETOUR_EAST_X: f32 = 49.5;

/// `zone_early`: the ordinary gate route with a there-and-back leg through
/// `guard_post_east` spliced onto the end, walked while q3 is still Locked.
///
/// The first five legs are `quest.rs`'s `GATE_ROUTE` verbatim — the demo has to
/// reach the gate square either way, and q2's `o1_gate` fires there. The
/// detour then drops south off the plateau onto [`LANE_Z`], runs east into the
/// region, comes back along the same lane, and climbs the ramp to end up beside
/// Maren exactly where the ordinary route leaves the body.
///
/// `scripts/act1_loop_audit.py` asserts the shared prefix still matches
/// `GATE_ROUTE` in `quest.rs`; a route that drifts is a route that walks into a
/// wall and blames the fix.
pub const GATE_ROUTE_ZONE_EARLY: [(f32, f32); 10] = [
    (GATE_X, 22.0),          // north — clear the shelter posts (x=29/35)
    (45.0, 22.0),            // east along clear ground south of the longhouse
    (45.0, RAMP_FOOT_Z),     // north along the open east field
    (GATE_X, RAMP_FOOT_Z),   // west onto the ramp column
    (GATE_X, GATE_SQUARE_Z), // up the ramp onto the gate square (q2 o1_gate)
    (GATE_X, LANE_Z),        // drop south off the plateau onto the clear lane
    (DETOUR_EAST_X, LANE_Z), // EAST into guard_post_east — q3 is still Locked
    (GATE_X, LANE_Z),        // back west along the same proven lane
    (GATE_X, RAMP_FOOT_Z),   // south to the foot of the ramp
    (GATE_X, GATE_SQUARE_Z), // up the ramp again, back beside Maren
];

/// How many legs of [`GATE_ROUTE_ZONE_EARLY`] are `GATE_ROUTE` verbatim.
pub const GATE_ROUTE_SHARED_LEGS: usize = 5;

/// `kill_early`: where the demo stands to fight Garren.
///
/// WEST of `guard_post_east.x0` — that is the entire point. The kill has to
/// land while the player is outside the region, so `o1_east` (and `o2_observe`
/// behind it) are still owed when `check_kill_triggers` runs.
///
/// Garren spawns at x=53.5 and patrols ±6 along x, so his western turn is
/// x=47.5 — 1.5 blocks from this stand, well inside the 6-block leash that
/// decides whether a chase sticks. He walks into melee on his own; the demo
/// never steps east to meet him.
pub const AMBUSH_X: f32 = 46.0;
pub const AMBUSH_Z: f32 = LANE_Z;

/// Mirrors of the numbers the ambush depends on. They live in `quest.rs`
/// (`GARREN_POS`) and `combat.rs` (`husk_leash`, the patrol turn), which both
/// pull in Bevy; `scripts/act1_loop_audit.py` re-reads them from those files and
/// fails if these copies drift.
pub const GARREN_X: f32 = 53.5;
pub const HUSK_PATROL_SPAN: f32 = 6.0;
pub const HUSK_LEASH: f32 = 6.0;

/// Garren's western patrol turn — the closest he comes without aggro.
pub fn garren_west_reach() -> f32 { GARREN_X - HUSK_PATROL_SPAN }

// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped story data. Tests run from the repo root (`rustc --test`) or
    /// from `client/` (`cargo test`), so try both.
    fn act1() -> serde_json::Value {
        for path in ["assets/story/act1.json", "../assets/story/act1.json"] {
            if let Ok(text) = std::fs::read_to_string(path) {
                return serde_json::from_str(&text).expect("act1.json must parse");
            }
        }
        panic!("act1.json not found from {:?}", std::env::current_dir());
    }

    /// `(x0, z0, x1, z1)` of a named region.
    fn bounds(region_id: &str) -> (f32, f32, f32, f32) {
        let data = act1();
        let r = data["regions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == region_id)
            .unwrap_or_else(|| panic!("region {region_id} missing from act1.json"))
            .clone();
        let b = &r["bounds"];
        (
            b["x0"].as_f64().unwrap() as f32,
            b["z0"].as_f64().unwrap() as f32,
            b["x1"].as_f64().unwrap() as f32,
            b["z1"].as_f64().unwrap() as f32,
        )
    }

    fn inside(region_id: &str, px: f32, pz: f32) -> bool {
        let (x0, z0, x1, z1) = bounds(region_id);
        px >= x0 && px <= x1 && pz >= z0 && pz <= z1
    }

    // ---- the mode flag ------------------------------------------------------

    #[test]
    fn parse_selects_the_two_scenarios() {
        assert_eq!(Chaos::parse(Some("zone_early")), Chaos::ZoneEarly);
        assert_eq!(Chaos::parse(Some("zone")), Chaos::ZoneEarly);
        assert_eq!(Chaos::parse(Some("kill_early")), Chaos::KillEarly);
        assert_eq!(Chaos::parse(Some("kill")), Chaos::KillEarly);
        assert_eq!(Chaos::parse(Some("  kill_early  ")), Chaos::KillEarly);
    }

    /// A typo must not silently select a scenario, and it must not select the
    /// *other* one — a run graded against the wrong contract is worse than no
    /// run.
    #[test]
    fn parse_degrades_unknown_values_to_none() {
        for raw in [None, Some(""), Some("1"), Some("true"), Some("zonearly"), Some("KILL_EARLY")] {
            assert_eq!(Chaos::parse(raw), Chaos::None, "raw={raw:?} must be None");
        }
        assert!(!Chaos::None.zone_early());
        assert!(!Chaos::None.kill_early());
    }

    #[test]
    fn labels_are_what_the_driver_greps() {
        assert_eq!(Chaos::None.label(), "none");
        assert_eq!(Chaos::ZoneEarly.label(), "zone_early");
        assert_eq!(Chaos::KillEarly.label(), "kill_early");
        // The label round-trips: the banner in a log re-selects the same mode.
        for m in [Chaos::None, Chaos::ZoneEarly, Chaos::KillEarly] {
            assert_eq!(Chaos::parse(Some(m.label())), m);
        }
    }

    // ---- the contract both scenarios rest on --------------------------------

    /// Both scenarios are defined relative to q3's objective list: `o1_east`
    /// first (the region), `o3_defeat` third (the kill). If the story data ever
    /// reorders them, the scenarios stop testing what they claim to.
    #[test]
    fn q3_objectives_are_in_the_order_the_scenarios_assume() {
        let data = act1();
        let q3 = data["quests"]
            .as_array()
            .unwrap()
            .iter()
            .find(|q| q["id"] == "q3_gatekeeper")
            .expect("q3_gatekeeper missing")
            .clone();
        let objs = q3["objectives"].as_array().unwrap();

        assert_eq!(objs[0]["id"], "o1_east");
        assert_eq!(objs[0]["kind"], "reach_zone");
        assert_eq!(objs[0]["target"], "guard_post_east");

        assert_eq!(objs[2]["id"], "o3_defeat");
        assert_eq!(objs[2]["kind"], "defeat");
        assert_eq!(objs[2]["target"], "garren_husk");

        // o3 sits BEHIND o1 — that is what makes an early kill out of order.
        assert!(
            2 > 0,
            "o3_defeat must come after o1_east or kill_early proves nothing"
        );
    }

    // ---- kill_early geometry ------------------------------------------------

    /// The whole scenario is "the kill lands while the player is outside the
    /// region". If the stand were inside it, `o1_east` would score first and the
    /// run would just be the ordinary route with extra steps.
    #[test]
    fn ambush_stand_is_outside_guard_post_east() {
        assert!(
            !inside("guard_post_east", AMBUSH_X, AMBUSH_Z),
            "ambush stand ({AMBUSH_X},{AMBUSH_Z}) must be OUTSIDE guard_post_east {:?}",
            bounds("guard_post_east")
        );
        let (x0, _, _, _) = bounds("guard_post_east");
        assert!(AMBUSH_X < x0, "ambush x={AMBUSH_X} must be west of x0={x0}");
    }

    /// ...and Garren has to be able to reach it, or the demo stands there
    /// swinging at nothing until the timeout and the log blames the fix.
    #[test]
    fn garren_can_walk_to_the_ambush_stand_without_leashing() {
        let gap = (garren_west_reach() - AMBUSH_X).abs();
        assert!(
            gap <= HUSK_LEASH,
            "Garren's western patrol turn is x={}, {gap} blocks from the stand — \
             past the {HUSK_LEASH}-block leash he drops back to patrol and never arrives",
            garren_west_reach()
        );
    }

    /// The stand must be far enough west that his patrol cannot drag the fight
    /// across the region line on its own.
    #[test]
    fn ambush_stand_has_margin_on_the_region_line() {
        let (x0, _, _, _) = bounds("guard_post_east");
        assert!(
            x0 - AMBUSH_X >= 1.5,
            "only {} blocks between the stand and the region line — knockback would \
             push the kill inside and the scenario would score o1_east first",
            x0 - AMBUSH_X
        );
    }

    #[test]
    fn ambush_stands_on_the_proven_clear_lane() {
        assert_eq!(AMBUSH_Z, LANE_Z, "the ambush must stand on the lane phase 2 already walks");
    }

    // ---- zone_early geometry ------------------------------------------------

    /// The detour has to actually enter the region, or `QUEST_AREA
    /// enter=guard_post_east` never prints and the run grades nothing.
    #[test]
    fn zone_early_detour_steps_inside_guard_post_east() {
        let hit: Vec<_> = GATE_ROUTE_ZONE_EARLY
            .iter()
            .enumerate()
            .filter(|(_, &(x, z))| inside("guard_post_east", x, z))
            .collect();
        assert!(
            !hit.is_empty(),
            "no waypoint of the zone_early detour lands in guard_post_east {:?}",
            bounds("guard_post_east")
        );
        // And it does so AFTER the shared prefix — the gate square comes first,
        // so q2 is finished and q3 is the quest still Locked when it happens.
        assert!(
            hit.iter().all(|(i, _)| *i >= GATE_ROUTE_SHARED_LEGS),
            "the detour enters the region during the shared prefix — it would no \
             longer be 'before q3', it would be part of the ordinary route"
        );
    }

    /// It has to come back, too: phase 1 presses E at Maren, and she stands in
    /// the gate arch.
    #[test]
    fn zone_early_detour_ends_back_on_the_gate_square() {
        let &(x, z) = GATE_ROUTE_ZONE_EARLY.last().unwrap();
        assert!(
            inside("gate_square", x, z),
            "the detour ends at ({x},{z}), outside gate_square {:?} — phase 1 would \
             time out reaching Maren and blame the interaction path",
            bounds("gate_square")
        );
    }

    /// The return leg must come back through the ramp foot. Walking straight
    /// north from the lane onto the plateau is a two-block climb and step-up is
    /// one — the body would wedge and the log would read like a quest bug.
    #[test]
    fn zone_early_return_climbs_the_ramp_instead_of_the_plateau_face() {
        let n = GATE_ROUTE_ZONE_EARLY.len();
        assert_eq!(GATE_ROUTE_ZONE_EARLY[n - 3], (GATE_X, LANE_Z));
        assert_eq!(GATE_ROUTE_ZONE_EARLY[n - 2], (GATE_X, RAMP_FOOT_Z));
        assert_eq!(GATE_ROUTE_ZONE_EARLY[n - 1], (GATE_X, GATE_SQUARE_Z));
        // Same column the ordinary route climbs.
        assert_eq!(GATE_ROUTE_ZONE_EARLY[3], (GATE_X, RAMP_FOOT_Z));
        assert_eq!(GATE_ROUTE_ZONE_EARLY[4], (GATE_X, GATE_SQUARE_Z));
    }

    /// Every east-west leg of the detour runs on the lane that is documented
    /// clear from x=30 to x=52 — the demo must not invent new ground.
    #[test]
    fn zone_early_detour_travels_only_the_proven_lane() {
        for w in GATE_ROUTE_ZONE_EARLY[GATE_ROUTE_SHARED_LEGS..].windows(2) {
            let ((x0, z0), (x1, z1)) = (w[0], w[1]);
            if (x1 - x0).abs() > 0.1 {
                assert_eq!(z0, LANE_Z, "east-west leg {w:?} leaves the clear lane");
                assert_eq!(z1, LANE_Z, "east-west leg {w:?} leaves the clear lane");
                assert!(
                    x0 >= 30.0 && x0 <= 52.0 && x1 >= 30.0 && x1 <= 52.0,
                    "east-west leg {w:?} runs past the surveyed x=30..52 span"
                );
            }
        }
    }

    /// A detour that never leaves the region it entered would let the ordinary
    /// `entered` latch stay true and prove nothing about a *return*.
    #[test]
    fn zone_early_detour_leaves_the_region_before_it_comes_back() {
        let (enter_idx, _) = GATE_ROUTE_ZONE_EARLY
            .iter()
            .enumerate()
            .find(|(_, &(x, z))| inside("guard_post_east", x, z))
            .unwrap();
        assert!(
            GATE_ROUTE_ZONE_EARLY[enter_idx + 1..]
                .iter()
                .any(|&(x, z)| !inside("guard_post_east", x, z)),
            "the detour never steps back out of guard_post_east"
        );
    }
}
