# First Playable Loop — "The First 10 Minutes of Voxelforge"

> **Design spec.** Story-mode-first. Engineer-facing — concrete enough to implement against.
> Tech: Rust + Bevy + wgpu. Combat spec resolved; see `docs/combat-design.md`.
> Last updated: 2026-07-27 · Owner: Monanisa (Design)

---

## Overview

The first 10 minutes teach the player four things, in order:

1. **You have a body** — third-person camera, the world is voxel, it's beautiful.
2. **Something is wrong here** — story hook lands before the first fight.
3. **Enemies hurt you** — first combat: learn or die.
4. **Death is a door, not a wall** — respawn at campfire, try again.

Target runtime: **8–12 min** on first play, ~4 min on retry.

---

## Act 0 — Cold Open (0:00–0:45)

**No menu. No loading screen text.** Player wakes up directly in the world.

### Spawn Conditions
- Location: inside a crumbling stone shelter at the edge of **Village of Edhari** (hand-crafted, ~48×48 block footprint).
- Time of day: late afternoon — golden key light from the west, long shadows.
- Weather: clear. Ambient particle: slow falling ash (visual signal: something burned here).

### What the Player Sees First
- Their own character's back — third-person camera at 60° angle, 5 m pull-back.
- Through the collapsed wall: the village courtyard, block towers, and a distant dungeon gate.
- A single campfire already lit 3 blocks away. **Tutorial implicit:** the campfire exists so the player instinctively walks toward warmth.

### Player Input: Zero Guidance Text
No UI prompts. The camera and environment direct attention. First input the player naturally makes: move toward the campfire.

---

## Act 1 — Spawn → Explore (0:45–3:30)

### Environment: Village of Edhari

| Zone | Description | Radius from spawn |
|---|---|---|
| Campfire square | Safe rest point; activates automatically on approach | 0–6 m |
| Collapsed houses | Ruined voxel structures; 2–3 have intact interiors with story fragments | 6–20 m |
| Village well | Centre landmark; skeleton of a villager slumped against it | 15 m |
| Dungeon gate | Large stone arch, sealed; glowing sigil above the door | 30 m north |

### Story Fragments (Environmental)
Three discoverable beats — none are mandatory, all hint at the plot:

1. **House interior:** A child's drawing on a wood block wall — figures fleeing something large. Torched corner.
2. **Well-side:** Carved text on the well rim: *"He came from below. We fed him everything. He left anyway."*
3. **Dungeon gate:** The sigil pulses once every 4 seconds. A faint sound plays — rhythmic, underground. Approaching within 5 blocks triggers ambient rumble (haptics if supported).

### Camera Behaviour During Exploration
- Roblox-style: right-stick / mouse rotates freely.
- Camera collides with voxel geometry (no clipping through walls).
- Auto-correct to player back when idle for 3 seconds (soft, 1.5 s ease).
- **No lock-on** during exploration — that feature is introduced in combat.

### Discovery Gate → Story Beat
When the player approaches the dungeon gate, a **triggered dialogue** fires.

---

## Act 2 — First Story Beat (3:30–5:00)

### Character: Elder Maren (NPC)

| Property | Value |
|---|---|
| Location | Behind the dungeon gate, calling through a crack in the stone |
| Voice | Elderly woman, urgent but not panicked |
| Appearance | Not fully visible — shadow and one hand reaching through the gap |

### Dialogue (3 lines max — skippable after first play)

> **Maren:** "You're awake. Good. The seal on this gate will hold another hour — maybe less."
>
> **Maren:** "The creature below — the one that took our people — it hasn't left. It's *nesting*."
>
> **Maren:** "There is a way in. The eastern guard post. But something is already patrolling outside it."

**Player objective set:** A simple objective marker appears — no quest log, just a faint directional glow on the cobblestone path leading east. The glow fades after 8 seconds (teaches: follow it now or find your own way).

### What the Player Learns Here
- There is a named threat below.
- Their goal is the dungeon — but a gatekeeper stands between them and the entrance.
- NPC exists and can be a source of information.

---

## Act 3 — First Combat Encounter (5:00–8:30)

### Enemy: Guard Husk (Archetype: Melee Grunt)

| Property | Value |
|---|---|
| Appearance | Armoured voxel figure, ~2.5 blocks tall, slow gait |
| Health | 80 |
| Attack pattern | 2-hit combo → pause → repeat. Telegraph: winds arm back visibly for 0.8 s before each strike |
| Patrol route | Loops a 12-block stretch in front of the guard post door |

### Combat Encounter Setup

**Approach:** The player rounds a corner and sees the Husk from 12+ blocks away — enough time to stop, observe, and plan. No ambush on the first encounter.

**Arena:** A flat courtyard, 16×16 blocks. One low wall on the south side (3 blocks high — can be used for positioning). No other voxel manipulation needed for the fight.

**Campfire:** Visible from the arena entrance, 6 blocks behind the player. They can retreat to it (enemies do not follow past this invisible boundary — ⟨design note: define boundary trigger in Bevy system⟩).

### Core Mechanics Introduced (in the fight)

| Mechanic | How the player discovers it |
|---|---|
| **Dodge (i-frames)** | Enemy wind-up is slow — rolling sideways before impact clearly avoids damage |
| **Stamina cost** | Two dodges in a row drain the bar visibly; a third dodge fails → player stumbles |
| **Light attack** | Pressing attack during enemy's recovery window deals damage; pressing during the combo doesn't |
| **Lock-on toggle** | UI hint appears after the first hit taken: `[R / RB] Lock On` |

### First-Death Design
If the player dies:
- **No death screen.** Fade to black (0.5 s), then fade in at the campfire with a single Bevy text component: *"Rest. Try again."* — disappears after 3 s.
- Enemy fully respawns.
- Player retains their knowledge — no mechanical loss.
- Campfire does not need to be re-lit.

### Win Condition
Guard Husk defeated → guard post door opens (animation: blocks slide apart, 1.5 s). The dungeon entrance is now reachable.

---

## Act 4 — Win / Lose Resolution + Player Learning (8:30–10:00)

### On Victory

| Beat | Implementation |
|---|---|
| Husk drops | Nothing. No loot, no XP number. The *door opening* is the reward. |
| Maren's voice | Distant: *"You found a way. Keep going."* (positional audio from the gate direction) |
| Camera moment | Auto-pan to the dungeon gate sigil pulsing faster — signals next objective |
| Campfire | A second campfire activates just inside the guard post (saves new respawn point) |

**Player has learned:**
- How to dodge and when
- Stamina is a resource, not a timer
- Enemies have readable telegraphs
- Death resets to campfire, not to the title screen
- The world reacts to their progress (door opened, fire activated)

### On Repeat Attempt (after death)

- Maren line does not replay unless player walks to the gate again.
- Patrol enemy is back, but the player knows the pattern now.
- Target time for players who "got it": under 3 minutes.

---

## Resolved Combat Values (from `docs/combat-design.md`)

| System | Value | Calibrated against |
|--------|-------|-------------------|
| Guard Husk HP | 80 | ~4 light attacks or 2 heavy attacks |
| Player max HP | 100 | 2–3 mistakes before death vs Husk |
| Player max stamina | 100 | flat v1 pool |
| Stamina recovery | 40/s, 0.3 s delay after action | Medium-load ER feel at smaller pool |
| Light / Heavy / Dodge stamina cost | 15 / 35 / 20 | ER roll cost scaled to 100-max pool |
| Dodge i-frames | 10 frames @ 60 FPS (~167 ms) | Slightly shorter than ER Medium roll |
| Dodge recovery | 12 frames @ 60 FPS (~200 ms) | |
| Hit-stop (player hits enemy) | 80 ms | Fighting-game impact frame budget |
| Screen-shake (heavy hit) | 0.10 amplitude / 0.20 s | Readable on blocky geometry |
| Lock-on max range | 16 blocks | Arena + courtyard sightlines |
| Lock-on camera snap speed | 8 rad/s | Responsive but readable |

All numbers are implementation defaults — playtest tuning expected.

---

## Out of Scope for This Loop

- Building / block placement (introduced in Chapter 2)
- Heavy attack (unlocked after Chapter 1 boss)
- NPC dialogue tree depth beyond these 3 lines
- Any UI beyond: health bar, stamina bar, objective glow, lock-on reticle

---

*Design: Monanisa. Combat values resolved with Sahara (research). Engine implementation: Poppy.*
