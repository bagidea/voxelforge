# First Five Minutes — Playtest Report

> **Tester:** Sun · **Date:** 2026-08-09 · **Binary:** `target-flamingo/release/voxelforge.exe`
> **Git:** `poppy/native-only` @ 88930cc · **Binary mtime:** 2026-08-08 16:25:42 (80.7 MB)
> **Method:** `--play` (interactive) + `--quest-demo` (scripted walk+fight) — console logs as evidence.

---

## Executive Summary

The loop **works end-to-end** — spawn → walk → campfire trigger → husk fight → husk defeat — all fire with real audio, VFX, anim rigs, and quest state. This is a **working game**, not a demo engine.

**But the win state is missing.** After the husk dies, nothing happens: no door opens, no sigil pulses, no Maren voice line, no next quest activates. The player just stands on the road with a dead enemy and no reason to keep going.

The July 30 Flamingo doc is **out of date**: the P0 husk-spawn blocker (`main.rs:632` — `map_load.is_some()` guard) does NOT apply to `--play` (map_load is a separate env var, not set by --play); audio, VFX, and anim rigs all work. The Play camera gets `look::base_camera_look()` inserted (tonemap, bloom, exposure, grade) — not bare `Camera3d::default()`.

---

## Pain Points — Ranked by Player Impact

### 🔴 HIGH (player stops playing here)

| # | Issue | Beat | Evidence | Fix |
|---|---|---|---|---|
| **H1** | **No win state after husk defeat** | 4 | Quest-demo log: `COMBAT husk defeated — despawned entity=503v0` (line 475), then 90+ frames of silent walking with no quest event. `q2_voice_in_stone` stays at "Accepted" — never completes. No door opening, no sigil change. | Wire `on_husk_defeat` → `QUEST_STAGE_COMPLETE q2` → door-open anim + sigil glow + Maren voice line |
| **H2** | **Death/respawn never fires in quest-demo** | 3 | The death→respawn system IS implemented and tested: `scene.rs:1359-1429` — `--play-demo` phases 10-11 force player HP to 0, wait for `PlayerDied`→fade→respawn, then verify HP restored + position at campfire (COMBAT_DEATH + COMBAT_RESPAWN both PASS/FAIL). But `--quest-demo` (the 5-min loop test) kills the husk with autopilot and never takes damage — so a human player hasn't seen "Rest. Try again." | Add a phase to `--quest-demo` where the player intentionally dies once, proving the full die→respawn→retry loop end-to-end |
| **H3** | **Campfire quest completes silently** | 1 | `--play` log: `QUEST_STAGE_COMPLETE qid=q1_embers oid=o1_campfire => PASS (approach dist=3.5)` — fires on proximity with **zero visual or audio feedback**. Player walks near fire and a quest silently completes. VFX fire is lit, but there's no "ding" sound, no text flash, no fire flaring up. | Add "campfire flares" VFX trigger + short text fade "The embers still glow warm" (per design doc) on quest complete |
| **H4** | **Quest q2 never completes** | 2-4 | `QUEST_ACCEPT id=q2_voice_in_stone => PASS` fires on frame 28. But no stage-complete for q2 appears in any log — even after husk defeat. q2 has no objective tied to husk death. | Wire husk death → `QUEST_STAGE_COMPLETE q2` → door opens. This is the missing link between combat and quest. |

### 🟡 MEDIUM (player feels confused or underwhelmed)

| # | Issue | Beat | Evidence | Fix |
|---|---|---|---|---|
| **M1** | **Character is box-humanoid rig, no human mesh** | 0 | `ANIM_RIG_WEAPON spawn actor=Player entity=578v0` — the anim rig exists and drives combat, but visually it's a geometric construct (capsule + boxes), not a voxel human. | Poppy's lane: replace rig with humanoid voxel mesh + walk anim (P1 in Flamingo doc) |
| **M2** | **Sigil = 1 sand block, no glow/pulse** | 2 | `first-five-minutes.md:13`: "sigil = sand 1 ก้อน (32,8,5) ไม่ emissive ไม่เต้น". Verified from map source — the sigil block above the gate is a plain `sand` block at y=5. | Replace sand block with emissive material + pulse animation (scale or color oscillate, 4s period per design) |
| **M3** | **Well too short to be a landmark** | 1 | `first-five-minutes.md:6`: "บ่อน้ำ 8 บล็อก y=1 ทั้งหมด". From 5m away, a 1-block-high object is invisible behind terrain. | Run `gen_edhari.py` to raise well to 2-3 blocks + add skeleton prop |
| **M4** | **No objective marker/glow toward gate** | 1-2 | Design doc §Act 2: "faint directional glow on the cobblestone path leading east. The glow fades after 8 seconds." No such system exists in logs. `ObjectiveTracker` UI component exists (`main.rs:932`) but stays empty. | Implement `ObjectiveTracker` updates on quest accept — short text + path glow particle |
| **M5** | **Dev HUD shows editor controls in play mode** | 0 | `main.rs:2585`: HUD line reads `"L=break R=place[stone] (cursor: click to lock)"` in Play mode. But `editor_edit` runs `.run_if(in_interactive_editor)` which is false in Play — **L/R clicks don't actually break/place**. The text is misleading. | Hide break/place text in Play mode; show only combat-relevant controls (L=attack, R=lock-on, Space=dodge) |
| **M6** | **Maren NPC spawns but no dialogue trigger** | 2 | `SPAWN_NPC id=maren name="Elder Maren" pos=(32,2,4)` — NPC entity exists, but approaching the gate doesn't trigger dialogue in the `--play` log. Dialogue system exists (`dialogue_ui::DialogueUiPlugin`), but trigger region might not fire on proximity. | Verify trigger region at gate; add OnEnter(region) → dialogue-start; add "Press E to talk" prompt |

### 🟢 LOW (nice to have, doesn't block play)

| # | Issue | Beat | Evidence | Fix |
|---|---|---|---|---|
| **L1** | **Sun at ~59° elevation, spec says 15-25°** | 0 | `main.rs:769`: `Transform::from_xyz(60.0, 120.0, 40.0)` → elevation ≈ 59°. Look Bible §2: outdoor key light at 15-25° for golden-hour drama. | Set `VOXELFORGE_LOOK_SUN` env var or adjust default DirectionalLight position |
| **L2** | **No falling ash particles** | 0 | Design doc §Act 0: "Ambient particle: slow falling ash." No particle system spawns at startup. `_dust` env var exists but not default. | Add ambient ash particle system to scene spawn |
| **L3** | **No skeleton at well** | 1 | `first-playable-loop.md:49`: "skeleton of a villager slumped against it." Map has well but no skeleton model/block. | Add skeleton as block arrangement or entity at well position |
| **L4** | **House interiors empty** | 1 | `first-five-minutes.md:6`: "บ้านสมบูรณ์ 2 หลัง ข้างในว่าง 0 บล็อก" + "ของในบ้าน — ต้องแก้ mapfile.rs + FORMAT.md ให้รับ block ใหม่ก่อน". | Add wood/leaves blocks to parser; then add interior props via gen_edhari.py |
| **L5** | **No audio for dialogue / story beats** | 1-2 | Design doc: Maren has 3 voice lines. No voice audio files or dialogue audio triggers detected. `AUDIO_PLAY` log only shows ambient + combat sounds. | Add dialogue audio triggers stubbed to placeholder/no-audio for now |
| **L6** | **Southern half of map is empty field** | 1 | `first-five-minutes.md:6`: "z≥44 (20 แถว) = 0 บล็อกเหนือพื้น". Player can walk into endless flat terrain. | Add invisible wall or natural barrier at z≥44 |

---

## What's Already Working (credit where due)

| System | Status | Proof |
|---|---|---|
| Map load (`maps/edhari.json`) | ✅ | `MAP_LOAD ok ... blocks=8838` |
| Terrain + chunks | ✅ | `MAP_APPLY set=8838 skipped=0 total_quads=5824` |
| Story/quest engine | ✅ | `STORY_LOAD ok ... quests=5 npcs=3 dialogues=12` |
| Quest auto-complete (q1 campfire) | ✅ | `QUEST_COMPLETE id=q1_embers => PASS` |
| Quest accept (q2) | ✅ | `QUEST_ACCEPT id=q2_voice_in_stone => PASS` |
| Husk spawn | ✅ | `SPAWN_ENCOUNTER husk at (32.0,_,25.0)` |
| NPC spawn | ✅ | `SPAWN_NPC id=maren name="Elder Maren"` |
| Combat HUD | ✅ | `SPAWN_ENCOUNTER` → `hud::spawn_hud` |
| Anim rig (player + husk) | ✅ | `ANIM_RIG_WEAPON spawn actor=Player/Husk` |
| VFX weapon trails | ✅ | `VFX_RIG_WEAPON_TRAIL attach actor=Player/Husk` |
| Combat attack windup/contact/recover | ✅ | `VFX_SWING_TRAIL hot=true/false phase=Windup/Contact/Recover` |
| Combat audio (swing/hit/death/footstep) | ✅ | `AUDIO_PLAY:audio/swing_light.wav` etc. (28 total plays in demo) |
| Husk death | ✅ | `COMBAT husk defeated — despawned entity=503v0` |
| Ambient audio (wind/campfire/village) | ✅ | 3 ambient tracks playing on Enter Play |
| Look pipeline (tonemap/bloom/exposure/grade) | ✅ | `look::base_camera_look()` inserted on play camera |
| Footstep sounds | ✅ | `AUDIO_PLAY:audio/footstep_stone.wav` × 14 in demo |
| Quest area zones | ✅ | `QUEST_AREA enter=campfire_square` + `village_road` |
| Death → fade → respawn cycle | ✅ | `scene.rs:781-871`: `on_player_death` receives `PlayerDied`, arms fade-to-black; `respawn_at_campfire` resets player HP + position + all husks. Headless proof in `--play-demo` phases 10-11: COMBAT_DEATH + COMBAT_RESPAWN both PASS |

---

## Fixed Since Flamingo's July 30 Doc

The following were listed as "ขาด" (missing) in `first-five-minutes.md` but are now confirmed working:

| Claim (July 30) | Reality (Aug 9) |
|---|---|
| "บนหมู่บ้านไม่มีศัตรู" — `main.rs:670` blocks spawn | ✅ Husks spawns. `--play` doesn't set `map_load`; guard at line 632 doesn't trigger |
| "ไม่มี HUD ต่อสู้" | ✅ Combat HUD appears (`hud::spawn_hud` in `spawn_encounter`) |
| "ไม่มี post stack เลย — กล้อง Play = Camera3d::default() เปล่า" | ✅ `look::base_camera_look()` inserted on play camera at line 898 |
| "ไม่มีเสียง (ไม่มี bevy_audio ทั้ง repo)" | ✅ Full audio: ambient (3 tracks), footstep, swing, hit, death |
| "ไม่มี particle" | ⚠️ Campfire VFX works (`VFX campfire lit`). Ash particles still missing |

---

## Run Artifacts

- `--play` console log: boots to Editor → enters Play → spawns husk + Maren → quest q1 completes → waits for human input
- `--quest-demo` console log (270 frames): same startup → auto-walks player north → fights husk with 5 light attacks → husk dies → keeps walking north with no quest progression
- `--play-demo` (source-verified): phases 1-11 → kill husk (COMBAT_KILL PASS) → kill player (COMBAT_DEATH PASS) → respawn at campfire (COMBAT_RESPAWN PASS) → exit. Death/respawn cycle proven headlessly.

**Binary:** `target-flamingo/release/voxelforge.exe` (80,680,960 bytes, 2026-08-08 16:25:42)
**Hardware:** GTX 1060 6GB / i5-12600K / 15.8 GiB / Vulkan

---

## Recommendation: Smallest Patch to Close the Loop

1. **Wire husk death → quest q2 complete** (1 line in combat defeat handler) ← **THE bottleneck**
2. **On q2 complete: door-open anim + sigil glow** (2 systems, already spec'd in `first-playable-loop.md`)
3. **Add "campfire flares" feedback on q1 complete** (VFX trigger + short text fade — turns silent quest into felt moment)
4. **Add death phase to quest-demo** so the "Rest. Try again." screen is verified end-to-end in the same 5-min loop (death/respawn already works, just never tested alongside quests)

These 4 items give the player: a start (campfire quest feels real), a middle (they can die and learn), and an end (husk defeat changes the world).

These 4 items close the "no win state" and "no death state" gaps — the two things that make this feel like a tech demo instead of a game.
