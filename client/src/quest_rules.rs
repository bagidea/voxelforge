//! Objective-ordering rules for the Act-1 quest loop — pure, no Bevy.
//!
//! Everything that decides *which objective is next* and *whether a quest is
//! finished* lives here, so it can be run headlessly. `quest.rs` cannot: its
//! tests are only reachable through the `voxelforge` bin target, which links
//! Bevy + wgpu and dies at DLL init on this machine (`0xc0000142`), so the 16
//! tests sitting in `quest.rs` have never once executed. These do:
//!
//! ```text
//! rustc --edition 2021 --test client/src/quest_rules.rs \
//!   --extern serde_json=target-flamingo/release/deps/libserde_json-<hash>.rlib \
//!   -L dependency=target-flamingo/release/deps -o _probe/quest_rules_test.exe
//! ./_probe/quest_rules_test.exe --nocapture
//! ```
//!
//! No cargo, no build lock, no GPU. The tests read the shipped
//! `assets/story/act1.json`, so they fail when the data drifts from the rules.
//!
//! ## Why these four functions
//!
//! `quest.rs` advances a quest with `prog.current_objective += 1` at six sites
//! and re-derives "is the quest done?" with the same
//! `filter(!optional).all(contains)` fold at six more. That is fine while every
//! objective is completed in list order — and wrong the moment one is not.
//! A `defeat` objective can be satisfied out of order (the body despawns the
//! frame it dies; if the kill is not credited then, it can never be credited),
//! and `+= 1` from an out-of-order completion steps *over* the objective the
//! player still owes.
//!
//! [`first_incomplete`] replaces `+= 1` with something that cannot skip, and
//! [`defeat_objective_to_credit`] lets a kill land on the objective it belongs
//! to regardless of where the player is in the list.

/// The slice of an objective these rules need. Implemented on
/// `quest::ObjectiveDef` for the game and on a stub in the tests, so the rules
/// stay free of the serde/Bevy data model.
pub trait ObjectiveLike {
    fn id(&self) -> &str;
    fn kind(&self) -> &str;
    fn optional(&self) -> bool;
}

/// Index of the first objective the player still owes.
///
/// This is the honest form of `current_objective += 1`: it is derived from
/// `completed`, so completing an objective out of order re-points the cursor at
/// the earliest one still outstanding instead of stepping past it. Returns
/// `objectives.len()` when nothing is left — every caller already treats an
/// out-of-range index as "no current objective" (`objectives.get(i)`).
pub fn first_incomplete<O: ObjectiveLike>(objectives: &[O], completed: &[String]) -> usize {
    objectives
        .iter()
        .position(|o| !completed.iter().any(|c| c == o.id()))
        .unwrap_or(objectives.len())
}

/// Is every non-optional objective complete? (Optional ones never block.)
pub fn all_required_done<O: ObjectiveLike>(objectives: &[O], completed: &[String]) -> bool {
    objectives
        .iter()
        .filter(|o| !o.optional())
        .all(|o| completed.iter().any(|c| c == o.id()))
}

/// Which `defeat` objective should an enemy death be credited to?
///
/// The first incomplete `defeat` objective in the quest — *not* whichever
/// objective happens to be current. A corpse is gone the frame after it dies,
/// so a kill that arrives while an earlier objective is outstanding is a kill
/// that can never be re-offered: crediting only `current_objective` turns
/// "fought the guard before Maren finished talking" into a quest that can never
/// be completed and an enemy that can never be re-killed.
///
/// Optional `defeat` objectives are eligible too — they are optional to the
/// *quest*, not un-creditable.
pub fn defeat_objective_to_credit<O: ObjectiveLike>(
    objectives: &[O],
    completed: &[String],
) -> Option<usize> {
    objectives.iter().position(|o| {
        o.kind() == "defeat" && !completed.iter().any(|c| c == o.id())
    })
}

/// Is `(px, pz)` inside an inclusive region box? Horizontal only — regions are
/// footprints, the player walks.
///
/// Presence, not entry. `check_area_triggers` latched every region in a
/// `Local<HashSet>` on first entry and evaluated `reach_zone` objectives only
/// inside that once-only branch, so a region visited before its quest went
/// Active was spent: walk east past the guard post while Maren is still
/// talking, and q3's `o1_east` has no second chance — with it, q3, q4 and q5.
pub fn region_contains(x0: f32, z0: f32, x1: f32, z1: f32, px: f32, pz: f32) -> bool {
    px >= x0 && px <= x1 && pz >= z0 && pz <= z1
}

// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct Obj {
        id: String,
        kind: String,
        optional: bool,
    }

    impl ObjectiveLike for Obj {
        fn id(&self) -> &str { &self.id }
        fn kind(&self) -> &str { &self.kind }
        fn optional(&self) -> bool { self.optional }
    }

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

    fn objectives(quest_id: &str) -> Vec<Obj> {
        let data = act1();
        let q = data["quests"]
            .as_array()
            .unwrap()
            .iter()
            .find(|q| q["id"] == quest_id)
            .unwrap_or_else(|| panic!("{quest_id} missing from act1.json"))
            .clone();
        q["objectives"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| Obj {
                id: o["id"].as_str().unwrap().to_string(),
                kind: o["kind"].as_str().unwrap().to_string(),
                optional: o["optional"].as_bool().unwrap_or(false),
            })
            .collect()
    }

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // -- first_incomplete -----------------------------------------------------

    #[test]
    fn cursor_starts_at_the_first_objective() {
        let objs = objectives("q3_gatekeeper");
        assert_eq!(first_incomplete(&objs, &[]), 0);
        assert_eq!(objs[0].id(), "o1_east");
    }

    #[test]
    fn cursor_does_not_step_over_an_owed_objective() {
        // The failure this exists for: the guard dies before the player has
        // walked into the post. `+= 1` would move the cursor from o1_east to
        // o2_observe and o1_east would never be looked at again.
        let objs = objectives("q3_gatekeeper");
        let completed = ids(&["o3_defeat"]);
        assert_eq!(first_incomplete(&objs, &completed), 0, "cursor must stay on o1_east");
        assert_eq!(objs[0].id(), "o1_east");
    }

    #[test]
    fn cursor_skips_what_is_already_done() {
        let objs = objectives("q3_gatekeeper");
        let completed = ids(&["o1_east", "o3_defeat"]);
        assert_eq!(objs[first_incomplete(&objs, &completed)].id(), "o2_observe");
    }

    #[test]
    fn cursor_runs_off_the_end_when_everything_is_done() {
        let objs = objectives("q2_voice_in_stone");
        let completed = ids(&["o1_gate", "o2_listen"]);
        assert_eq!(first_incomplete(&objs, &completed), objs.len());
        assert!(objs.get(first_incomplete(&objs, &completed)).is_none());
    }

    // -- all_required_done ----------------------------------------------------

    #[test]
    fn optional_objectives_never_block_a_quest() {
        // q1's o2_find_survivor and q3's o4_if_you_fall are optional; q3's
        // o4 has no producer in the engine at all, so if it blocked, Act 1
        // would end at q3 forever.
        let q1 = objectives("q1_embers");
        assert!(all_required_done(&q1, &ids(&["o1_campfire"])));

        let q3 = objectives("q3_gatekeeper");
        assert!(q3.iter().any(|o| o.id() == "o4_if_you_fall" && o.optional()));
        assert!(all_required_done(&q3, &ids(&["o1_east", "o2_observe", "o3_defeat"])));
    }

    #[test]
    fn a_missing_required_objective_keeps_the_quest_open() {
        let q4 = objectives("q4_what_walls_remember");
        assert!(!all_required_done(&q4, &ids(&["o1_build", "o2_ledger"])));
        assert!(all_required_done(&q4, &ids(&["o1_build", "o2_ledger", "o3_offering"])));
    }

    #[test]
    fn every_act1_quest_can_be_finished_from_its_required_objectives() {
        let data = act1();
        for q in data["quests"].as_array().unwrap() {
            let qid = q["id"].as_str().unwrap();
            let objs = objectives(qid);
            let required: Vec<String> = objs
                .iter()
                .filter(|o| !o.optional())
                .map(|o| o.id().to_string())
                .collect();
            assert!(
                all_required_done(&objs, &required),
                "{qid} cannot be completed even with every required objective done"
            );
        }
    }

    // -- defeat_objective_to_credit ------------------------------------------

    #[test]
    fn a_kill_is_credited_even_when_it_arrives_early() {
        // Nothing completed yet — the cursor is on o1_east (reach_zone), which
        // is what made the old code drop the kill on the floor.
        let objs = objectives("q3_gatekeeper");
        let idx = defeat_objective_to_credit(&objs, &[]).expect("the kill must land somewhere");
        assert_eq!(objs[idx].id(), "o3_defeat");
        assert_eq!(objs[idx].kind(), "defeat");
    }

    #[test]
    fn a_second_kill_is_not_credited_twice() {
        let objs = objectives("q3_gatekeeper");
        assert!(defeat_objective_to_credit(&objs, &ids(&["o3_defeat"])).is_none());
    }

    #[test]
    fn a_quest_with_no_defeat_objective_ignores_kills() {
        // The village husk on the road dies during q2. q2 has no `defeat`
        // objective, so the kill must not touch it.
        let objs = objectives("q2_voice_in_stone");
        assert!(defeat_objective_to_credit(&objs, &[]).is_none());
    }

    // -- region_contains ------------------------------------------------------

    #[test]
    fn guard_post_bounds_match_where_garren_stands() {
        // GARREN_POS in quest.rs is (53.5, 8.0); the region is x 48-56, z 4-12.
        let data = act1();
        let r = data["regions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "guard_post_east")
            .unwrap()
            .clone();
        let b = &r["bounds"];
        let (x0, z0, x1, z1) = (
            b["x0"].as_f64().unwrap() as f32,
            b["z0"].as_f64().unwrap() as f32,
            b["x1"].as_f64().unwrap() as f32,
            b["z1"].as_f64().unwrap() as f32,
        );
        assert!(region_contains(x0, z0, x1, z1, 53.5, 8.0), "Garren stands inside his own region");
        assert!(region_contains(x0, z0, x1, z1, x0, z0), "bounds are inclusive");
        assert!(!region_contains(x0, z0, x1, z1, 32.0, 29.0), "the campfire is not the guard post");
    }

    #[test]
    fn presence_is_answerable_on_every_frame_not_just_on_entry() {
        // The same point tests true twice — the rule is a predicate on where the
        // player IS, so a region entered before its quest went Active is still
        // there to be entered again.
        let (x0, z0, x1, z1) = (48.0, 4.0, 56.0, 12.0);
        assert!(region_contains(x0, z0, x1, z1, 52.0, 7.0));
        assert!(region_contains(x0, z0, x1, z1, 52.0, 7.0));
    }

    // -- the whole Act-1 walk -------------------------------------------------

    #[test]
    fn act1_walks_end_to_end_with_the_kill_arriving_first() {
        // Worst-case ordering for q3: the player fights Garren the moment he
        // spawns, before the reach_zone and approach objectives are credited.
        // Act 1 must still be finishable.
        let data = act1();
        let mut quest = data["start_quest"].as_str().unwrap().to_string();
        let mut walked = Vec::new();

        while !quest.is_empty() {
            let objs = objectives(&quest);
            let mut completed: Vec<String> = Vec::new();

            // Credit a kill first if this quest has a defeat objective at all.
            if let Some(i) = defeat_objective_to_credit(&objs, &completed) {
                completed.push(objs[i].id().to_string());
                assert_eq!(
                    objs[first_incomplete(&objs, &completed)].id(),
                    objs[0].id(),
                    "{quest}: an early kill must not move the cursor off the first objective"
                );
            }
            // Then walk the rest in cursor order, as the player would.
            let mut guard = 0;
            while first_incomplete(&objs, &completed) < objs.len() {
                let i = first_incomplete(&objs, &completed);
                completed.push(objs[i].id().to_string());
                guard += 1;
                assert!(guard <= objs.len() + 1, "{quest}: cursor is not advancing");
            }
            assert!(all_required_done(&objs, &completed), "{quest} did not finish");
            walked.push(quest.clone());

            let q = data["quests"].as_array().unwrap().iter()
                .find(|q| q["id"] == quest.as_str()).unwrap().clone();
            quest = q["next"].as_str()
                .or_else(|| q["rewards"]["advance_to"].as_str())
                .unwrap_or("")
                .to_string();
        }

        assert_eq!(walked.len(), data["quests"].as_array().unwrap().len(),
            "every quest in act1.json must be walked: {walked:?}");
        assert_eq!(walked.last().unwrap(), "q5_sigil_that_knew_you");
        println!("SIM_ACT1_WALK {}", walked.join(" -> "));
    }
}
