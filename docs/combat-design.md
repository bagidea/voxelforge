# Voxelforge Combat Design — Soulslike Loop for the First Playable

> **Engineer-facing spec.** Rust + Bevy 0.19 + wgpu. Concrete numbers for immediate implementation.  
> **Scope:** single-player, first-playable chapter (village ruins → guard post → dungeon gate).  
> **Last updated:** 2026-07-27 · Owner: Sahara (Research & Design)

---

## 1. Design Goal

Voxelforge combat must feel **deliberate, readable, and fair** inside a blocky voxel world. The visual language is toy-like, but the combat weight comes from Elden Ring / Dark Souls: every action costs stamina, every enemy telegraphs, and death teaches rather than punishes.

We deliberately **do not** copy anime-action speed (Black Myth: Wukong / Devil May Cry). Instead we borrow:

- **Stamina-gated commitment** from Elden Ring / Dark Souls.
- **Poise-as-commitment-armor** from Dark Souls 3, simplified.
- **Honest, layered telegraphs** from Sekiro and Hytale.
- **i-frame dodge** from Elden Ring, calibrated shorter for a first encounter.

### Why these fit voxel

- Voxel characters are small and low-detail; fast, subtle animations would be unreadable. Slow wind-ups and big pose changes read clearly on blocky rigs.
- Blocky terrain creates natural chokepoints and cover; stamina pacing gives the player time to use terrain.
- Third-person camera on blocky geometry needs lock-on to keep melee readable.

---

## 2. Core Combat Loop

### 2.1 Stamina

Stamina is the central tension. Every attack, dodge, block, parry, and sprint draws from one shared pool.

| Action | Cost | Recovery delay | Rationale |
|--------|------|----------------|-----------|
| Light attack | 15 | 0.25 s | Fast, spammable only if stamina allows |
| Heavy attack | 35 | 0.40 s | High commitment |
| Charged attack | 50 | 0.50 s | Highest commitment; hold > 0.6 s before release |
| Dodge / roll | 20 | 0.30 s | ER roll costs ~12 with a pool that can exceed 150; scaled to our 100-max pool |
| Block (per hit) | 15 | 0.20 s | Holding block is free; getting hit while blocking costs stamina |
| Parry attempt | 10 | 0.35 s | Cheaper than a blocked hit, but punishes whiff |
| Sprint | 25 / sec | — | Shared pool; no separate sprint meter |

- **Max stamina:** 100 (flat for v1; no Endurance scaling yet).
- **Stamina recovery:** 40 points/second when not in an action-state and above 0.
- **Exhaustion:** If stamina hits 0, the player cannot attack/dodge/block for **0.8 s** and the recovery delay is doubled until stamina is back above 20.
- **Equip load:** v1 uses a single fixed Medium load. Roll i-frames and distance are fixed; load-based roll variants are **Out of Scope** for the first playable.

> **Reference:** Elden Ring uses ~12 stamina per roll with a pool that can exceed 150; our smaller pool makes every dodge matter. Dark Souls stamina recovery is gated by equip load; we keep Medium-load values for v1.

### 2.2 Dodge / Roll + i-frames

| Property | Value |
|----------|-------|
| Input | Space (KB) / A (gamepad) while moving |
| i-frame window | **10 frames @ 60 FPS** (~167 ms) |
| Recovery frames | 12 frames @ 60 FPS (~200 ms) before next action |
| Roll distance | 2.5 blocks |
| Direction | 4-way relative to camera when unlocked; relative to target when locked-on |
| Cost | 20 stamina |

> **Reference:** Elden Ring Medium roll = 13 i-frames at 30 FPS logic (~26 display frames at 60 FPS). We choose **10 @ 60 FPS** because the Guard Husk telegraph is 0.8 s and the arena is small; a slightly shorter window still leaves a ~0.48 s reaction buffer after the telegraph begins.

### 2.3 Lock-on / Soft-lock

| Property | Value | Notes |
|----------|-------|-------|
| Toggle input | R / RB | Press to lock nearest enemy; press again to unlock |
| Max range | **16 blocks** | Matches first-playable arena + courtyard sightlines |
| Acquisition cone | 45° horizontal, 30° vertical | Prevents snapping to enemies behind camera |
| Camera snap speed | **8 rad/s** | Responsive but readable transition |
| Reticle | Small diamond over target centre mass | |
| Auto-unlock | Target dies / target > 20 blocks / manual toggle / LoS blocked > 1.5 s | |

When locked-on:
- Player movement strafes around target (A/D = circle, W/S = advance/retreat).
- Attacks orient toward target.
- Camera frames both player and target with a slight bias to target side.

When unlocked:
- Attacks use current facing (camera-relative).
- Ranged/AOE later will need free aim; melee v1 uses soft auto-aim within a 15° cone.

> **Reference:** Z-targeting from *Ocarina of Time* / hard-lock from *Dark Souls*. Soft-lock hybrid lets the player break lock for environmental awareness.

### 2.4 Attack Types

| Attack | Input | Damage | Poise dmg | Total time | Notes |
|--------|-------|--------|-----------|------------|-------|
| Light | LMB / X | 20 | 15 | 0.35 s | Combo: up to 3 hits, each +10% damage if chain lands |
| Heavy | Hold RMB / Y | 45 | 40 | 0.85 s | Hyper armor during last 0.40 s; cannot cancel |
| Charged | Hold RMB/Y > 0.6 s | 70 | 60 | 1.10 s total | Release automatically; early release = Heavy |

- **Combo reset window:** 0.6 s after a light attack.
- **Action queue:** at most 1 input buffered for 0.15 s.
- **Cancels:** Light can cancel into dodge up to 0.15 s after startup. Heavy cannot cancel.

### 2.5 Block & Parry

**Block (hold RMB / LT):**
- Reduces incoming damage by 50%.
- Costs 15 stamina per hit blocked.
- Cannot block while attacking or dodging.
- If stamina < block cost, block fails and player takes full damage + **guard-break stagger** 0.6 s.

**Parry (tap RMB / LT within parry window):**
- Parry window: **0.20 s** (12 frames @ 60 FPS) centered on impact.
- Successful parry: negates damage, deals 25 posture damage to attacker, opens **1.2 s punish window** where attacker takes +25% damage.
- Failed parry: player takes 25% extra damage, 0.5 s recovery.
- Parry does **not** work on heavy “unparryable” attacks (indicated by red VFX flash during wind-up).

> **Reference:** Sekiro deflect is the core loop; our parry is narrower and riskier to keep it optional, not mandatory.

---

## 3. Damage & Poise Model

### 3.1 Health

- **Player max HP:** 100.
- **Enemy HP:** see §4 archetypes.
- **No healing in combat** for the first playable; rest at campfire restores HP and resets enemies.

### 3.2 Poise / Posture

Every combatant has a **Max Poise** value and a hidden **Poise Health** that drains when taking poise damage.

- Poise damage from attacks subtracts from current poise health.
- Poise health regenerates at **10/second** after **2.0 s** without taking poise damage.
- When poise health reaches 0, target enters **Stagger** for **1.5 s**.
  - During stagger: cannot act, takes +30% damage.
  - After stagger: poise health resets to max.
- **Hyper armor:** during heavy-attack wind-up (last 0.4 s) and some enemy attacks, poise damage taken is reduced by 75% and stagger is suppressed.

| Actor | Max Poise | Notes |
|-------|-----------|-------|
| Player | 40 | Light attacks from grunts stagger if 3 hits land quickly |
| Guard Husk | 30 | Staggers after 2 light attacks or 1 heavy |
| Ash Hound | 15 | Staggers after 1 light attack; designed to be interrupted |
| Hearth Knight | 80 | Hyper armor on most attacks; needs heavy/charged or parry |

> **Reference:** Dark Souls 3 poise only functions during hyper armor; we make it always active but lower numbers so small enemies feel interruptible while heavies feel committed. Sekiro posture breaks replace health bars for some enemies; we use posture as a stagger gate, not a death gate.

### 3.3 Damage Type

v1 uses a single physical damage type. Future types (fire, blunt, slash, pierce) are **Out of Scope**.

---

## 4. Enemy Archetypes (First Playable)

### 4.1 Guard Husk — Melee Grunt

First-encounter enemy in `docs/first-playable-loop.md`.

| Stat | Value |
|------|-------|
| HP | **80** |
| Poise | 30 |
| Walk speed | 1.5 blocks/s |
| Turn speed | 2.0 rad/s |

**Attack: 2-Hit Combo**
- Wind-up: **0.8 s** (arm winds back)
- First swing: 15 damage, 15 poise, active frames 0.2 s
- Second swing: 20 damage, 20 poise, active frames 0.2 s
- Pause between combos: 1.5 s
- Telegraph: raised arm + weapon glint + grunt sound

**Behaviour**
- Patrols 12-block loop.
- On aggro: walks toward player, attacks when within 2.0 blocks.
- If player retreats > 6 blocks, returns to patrol (campfire boundary handles the rest).

### 4.2 Ash Hound — Fast Skirmisher

| Stat | Value |
|------|-------|
| HP | 50 |
| Poise | 15 |
| Walk speed | 3.5 blocks/s |

**Attack: Lunge Flurry**
- Telegraph: 0.4 s (low crouch + hiss)
- 3-hit combo: 10 damage / 8 poise each
- Total duration: 0.9 s
- Recovery: 0.8 s

**Behaviour**
- Circles player, attacks from flanks.
- Back-dashes after flurry.
- Designed to teach lock-on switching and parry timing.

### 4.3 Hearth Knight — Heavy Elite / Mini-Boss

Planned for end of Chapter 1; included here so the archetype set is complete.

| Stat | Value |
|------|-------|
| HP | 200 |
| Poise | 80 |
| Walk speed | 1.8 blocks/s |

**Phase 1 (> 50% HP)**
- Slow overhead: 30 damage, 40 poise, 1.2 s telegraph
- Shield bash: 20 damage, 30 poise, 0.9 s telegraph, **unparryable**

**Phase 2 (≤ 50% HP)**
- Attack speed +20%
- Adds jump slam: 40 damage, 50 poise, 1.5 s telegraph
- Some combos gain hyper armor

---

## 5. Feedback & Telegraphs

### 5.1 Telegraphs

Every attack must broadcast intent through **at least two channels**:

| Threat level | Visual | Audio | VFX |
|--------------|--------|-------|-----|
| Fast jab | Limb pullback 0.3–0.5 s | Short wind-up hiss | None or small white flash |
| Heavy strike | Full body wind-up 0.8–1.5 s | Louder grunt / weapon drag | Weapon glint, red eye flash on boss |
| Unparryable | Red aura during wind-up | Distorted metal sound | Red particle ring |

- Telegraphs are **honest**: active hitbox appears only after wind-up ends; no fake-outs.
- Wind-up animation scales with weapon size; voxel rigs use exaggerated poses because small details disappear at a distance.

### 5.2 Hit-Stop / Hit-Pause

| Event | Duration | Who pauses |
|-------|----------|------------|
| Player hits enemy | 80 ms | Both attacker and target |
| Player parries enemy | 120 ms | Both |
| Enemy hits player | 100 ms | Both |
| Stagger / critical punish | 150 ms | Target only |

Implementation: pause affected entities' animation/update systems; camera and audio continue. Do **not** use global time dilation.

> **Reference:** Fighting games use 4–12 frame hit-stop to sell impact; our 80 ms ≈ 5 frames @ 60 FPS on light hits, 150 ms on criticals.

### 5.3 Screen-Shake

| Event | Amplitude | Duration | Direction |
|-------|-----------|----------|-----------|
| Player light attack lands | 0.04 | 0.10 s | Toward impact |
| Player heavy/charged lands | 0.10 | 0.20 s | Toward impact |
| Player parry succeeds | 0.12 | 0.18 s | Away from enemy |
| Enemy heavy hits player | 0.15 | 0.25 s | Away from enemy |
| Stagger break | 0.20 | 0.30 s | Radial |

- Shake uses diminishing magnitude (sine decay).
- Accessibility: must be toggleable in settings.

### 5.4 Hit Reaction Animations

| Hit strength | Reaction |
|--------------|----------|
| Poise still > 0, light hit | Flinch: head/body jerk, 0.15 s |
| Poise still > 0, heavy hit | Stumble: step back, 0.35 s |
| Poise broken | Stagger: fall to knees / stun pose, 1.5 s |

For voxel characters, reactions are key-frame pose changes, not mocap; exaggerate torso rotation and weapon displacement so the state change reads at a distance.

---

## 6. First-Playable Combat Values (summary)

These are the concrete numbers that fill the `⟨TBD⟩` slots in `docs/first-playable-loop.md`.

| Quantity | Value |
|----------|-------|
| Guard Husk HP | **80** |
| Player max HP | 100 |
| Player max stamina | 100 |
| Stamina recovery | 40/s (delay 0.3 s after action) |
| Light attack stamina cost | 15 |
| Heavy attack stamina cost | 35 |
| Dodge stamina cost | 20 |
| Dodge i-frames | **10 frames @ 60 FPS** (~167 ms) |
| Dodge recovery | 12 frames @ 60 FPS (~200 ms) |
| Lock-on max range | **16 blocks** |
| Lock-on snap speed | **8 rad/s** |
| Light hit hit-stop | 80 ms |
| Heavy hit screen-shake | 0.10 amplitude / 0.20 s |
| Guard Husk telegraph | 0.8 s |

---

## 7. References & Why We Borrowed From Each

| Game | What we took | Why it fits Voxelforge |
|------|--------------|------------------------|
| **Elden Ring** | Stamina-gated commitment, i-frame dodge, honest telegraphs | Creates methodical pace that reads well on blocky models |
| **Dark Souls 3** | Poise-as-commitment-armor, hyper armor on heavy attacks | Lets player choose when to trade; adds matchup depth |
| **Sekiro** | Parry = high-risk deflect that opens a punish window | Optional mastery path; red unparryable cue is unambiguous |
| **Hytale** | Stamina / block / parry / directional dodge in a voxel wrapper | Proves voxel + soulsy combat is a viable combination |
| **Black Myth: Wukong** | Speed *rejected* on purpose; we use slower, readable timing | Voxel scale + tight camera favor clarity over flair |

### Sources

1. [Elden Ring Wiki — Equip Load](https://eldenring.wiki.fextralife.com/Equip+Load) — equip-load thresholds and roll behaviour.
2. [Elden Ring Wiki — Dodging](https://eldenring.wiki.fextralife.com/Dodging) — i-frame counts and recovery frames.
3. [Dark Souls 3 Wiki — Poise](https://darksouls3.wiki.fextralife.com/Poise) — poise health, hyper armor, and stagger formula.
4. [Sekiro — FromSoftware Web Manual](https://www.fromsoftware.jp/manual/sekiroshadowsdietwice/stadia/mechanics.html) — posture, deflect, deathblow.
5. [GameWith — Sekiro Posture System Explained](https://gamewith.net/sekiro/article/show/8483) — posture break and recovery.
6. [Critpoints — Hitstop](https://critpoints.net/2017/05/17/hitstophitfreezehitlaghitpausehitshit/) — hit-stop purpose and timing.
7. [Hytale — Combat System](https://hytale.game/en/combat-system/) — voxel action-combat reference.
8. [Broken Build Studios — Why Enemy Telegraphs Matter](https://brokenbuildstudios.com/why-enemy-telegraphs-matter-in-game-design/) — telegraph design principles.
9. [Critpoints — Lock-On Styles in Action Games](https://critpoints.net/2015/05/24/what-is-your-ideal-form-of-lock-on-in-action-games/) — hard-lock vs soft-lock trade-offs.

---

## 8. Engineer Implementation Checklist (ordered)

1. **Player combat state machine** (`PlayerCombat` component): idle, light, heavy, charged, dodge, block, parry, stagger, dead.
2. **Stamina resource** (`Stamina` component): max 100, drain on actions, regen with delay, exhaustion penalty.
3. **Dodge system**: i-frame window, roll animation + movement, recovery, cost.
4. **Lock-on system**: target acquisition, camera orbit, strafe movement, reticle.
5. **Attack system**: light/heavy/charged inputs, combo timer, damage/poise, hyper armor.
6. **Block/parry system**: damage reduction, parry window, posture damage, guard break.
7. **Poise system**: poise health, regen, stagger, hyper armor.
8. **Enemy archetypes** (`Enemy` component + archetype enum): Guard Husk, Ash Hound, Hearth Knight with stats and move sets.
9. **AI behaviour tree (minimal)**: patrol, aggro, attack selection, recovery, phase transition for Hearth Knight.
10. **Telegraph system**: animation poses, audio cues, VFX timing per attack.
11. **Feedback systems**: hit-stop, screen-shake, hit reactions, death/respawn.
12. **First-encounter integration**: spawn Guard Husk at guard post, tie death to door open, campfire respawn.
13. **HUD**: health bar, stamina bar, lock-on reticle, objective glow.
14. **Playtest tuning**: iterate i-frames, telegraph speed, damage numbers after first-playable session.

---

*Research & design: Sahara. Hand-off to: Monanisa (Design) and Poppy (Engine).*
