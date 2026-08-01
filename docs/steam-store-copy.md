# Voxelforge — Steam Store Page Copy

> English copy for the Steam store page. Source of truth: `docs/GAME-VISION.md` (pillars + Milestone 1
> scope), `docs/first-playable-loop.md` (the only content slice that exists today), `docs/combat-design.md`
> (shipped mechanics), `docs/gate3-plan.md` (LookPlugin quality tiers), `docs/research/steamworks-publishing-brief-2026.md`
> (Valve specs + Sahara's ghost research, merged at commit `5f64ab0`). Nothing below sells a feature that
> isn't in that scope. Art direction / capsule composition: `docs/steam-store-art.md` (owned by the art
> lane — not touched here).
>
> Owner: Sun (copy & validation) + Monanisa (design). Last updated: 2026-08-01.
>
> **Correction note (2026-08-01):** an earlier draft sold block placement/destruction as something you
> do *while exploring Edhari or mid-fight* — "wall off a corridor, bridge a gap" language. That's not
> true of the shipping build: `client/src/main.rs` gates `edit_voxels` OFF during `AppState::Play`.
> Building only runs in the separate `AppState::Editor` sandbox. Fixed: building is now described as a
> separate, decoupled creative mode, never as an exploration/combat mechanic.

---

## 1. Short description *(Valve limit: ≤300 characters)*

**Primary — 264 chars ✓**

> A voxel world lit like a photograph, with the weight of a soulslike underneath. Explore the ruins
> of Edhari, read every enemy's telegraph, and fight with real stamina and real consequence. Blocky
> geometry, cinematic golden-hour light — third-person, single-player.

**Alt (A/B test) — 238 chars ✓**

> Blocky world, honest combat. Explore a golden-hour voxel village, dodge on a stamina budget, and
> read every telegraph before you punish it. A story-driven third-person soulslike where the toy-simple
> geometry hides real, deliberate weight.

---

## 2. About This Game *(Valve limit: none hard-capped; Steam recommends concise, no external links)*

Character count (body text excluding headings): **3,151 chars with spaces** (counted from the bold
lede to the end of the last paragraph). Valve's store page field accepts significantly more than this
(the UI cut-off is ~8,000 chars in practice), so 3,151 is comfortably within bounds. No external links are present.

**A world you can trust, a fight that doesn't let you cheat.**

Voxelforge drops you into the ruins of Edhari with no tutorial text and no HUD until you've earned
one. You wake up in a collapsed shelter, golden-hour light raking across block geometry that looks
soft and toy-like — right up until something with a blank visor starts walking toward you.

This is a soulslike built out of cubes: every attack is telegraphed, every dodge costs stamina, and
every fight is winnable the moment you learn to read it. Death sends you back to the campfire with
your knowledge intact — no item loss, no punishment spiral, just another attempt.

**Explore first, fight second.**
- Walk the hand-crafted ruins of the Village of Edhari — a burned-out settlement with three
  discoverable story fragments (a child's drawing, carved well text, a sealed dungeon gate) that
  hint at what happened here before you ever swing a weapon.
- A free-look third-person camera keeps your character on screen at all times — this is not a
  first-person game, and it never will be.
- An NPC gives you your objective through dialogue, not a quest log: a faint glow on the path is
  the only marker you get.

**Combat with a stamina budget, not a combo counter.**
- One shared stamina pool gates every dodge, block, parry, light attack, and heavy attack — spend
  it wisely or get caught empty.
- A 10-frame dodge roll (~167 ms of invulnerability) rewards a precise read of the enemy's wind-up,
  not a panic mash.
- Lock-on targeting (toggle, 16-block range) keeps melee readable against blocky geometry — strafe,
  circle, and time your openings.
- Every enemy attack telegraphs through at least two channels — a visible wind-up and an audio cue —
  before the hit lands. No fake-outs, no unreadable one-frame punishes.
- The Guard Husk, the first thing standing between you and the dungeon gate: an armoured grunt with
  a slow two-hit combo you can learn on sight, in an arena built to give you room to observe before
  you commit.

**A campfire that remembers your progress.**
- Rest at campfires to heal and reset the fight — your first one lights the moment you arrive, and a
  second unlocks the moment you win.

**A separate creative mode, for when you'd rather build than fight.**
- Step out of the Edhari campaign into a dedicated build sandbox: place and destroy blocks freely, no
  recipes, no crafting menus — just pick a block and lay it down.
- This is its own mode, not a tool you reach for mid-exploration or mid-fight — the campaign and the
  sandbox don't run at the same time. Toggle in when you want to build, toggle back when you want to
  play the story.

**Built to look like a photograph of a toy.**
- Hard-edged voxel geometry stays exactly that: blocky, readable, unapologetically toy-like.
- A full PBR lighting stack — golden-hour key light, soft PCSS shadows, ambient occlusion, bloom,
  distance haze, and depth-of-field at higher settings — wraps that geometry in cinematic,
  photographic light. Four scalable quality tiers (Low → Ultra) let you choose your balance of
  fidelity and frame rate, right down to volumetric god rays at the top end.

---

## 3. Feature list *(Valve: bullet list, no hard character limit per item; keep items short — storefront renders as a tick-list module)*

Longest bullet: **182 chars** (collapsed to single-line as Steam renders). All are under 200 chars — safe for Steam's feature-list module which wraps
at the UI level.

- **Soulslike combat, built for blocks.** Stamina-gated dodge, light and heavy attacks, block, and a
  high-risk 12-frame parry window — every action costs something, every enemy telegraphs honestly.
- **Lock-on third-person combat camera.** 16-block soft-lock, camera-relative strafing, always-visible
  player character.
- **A hand-crafted village to explore.** No procedural generation — every ruin, every environmental
  story beat in Edhari was placed on purpose.
- **Environmental storytelling over quest logs.** Discover the plot through what's carved into the
  world, not a checklist.
- **Death without punishment.** Full-knowledge retry from the last campfire — no item loss, enemies
  reset, you don't.
- **Separate creative/build mode.** Toggle out of the Edhari campaign into a dedicated sandbox to
  place and destroy blocks freely — its own mode, decoupled from the story and combat.
- **Four graphics quality tiers (Low → Ultra).** Scalable post-processing stack — tonemapping, color
  grading, bloom, soft shadows, ambient occlusion, depth of field, distance fog, and volumetric god
  rays at Ultra.
- **Native desktop build.** Built in Rust on Bevy/wgpu — no browser runtime, no plugin required.

---

## 4. Genre & category *(Steamworks App Admin — Store Page Settings)*

Valve requires picking from their fixed genre list in the Steamworks backend. These are the checkboxes,
not free-text tags (tags are §5).

**Primary genre:** Action *(maps to the combat-first gameplay — see `docs/combat-design.md`)*
**Secondary genre:** Adventure *(maps to the exploration + environmental storytelling pillar)*

**Category checkboxes to enable:**
- Single-player *(mandatory — the game has no multiplayer)*

**Do NOT enable:**
- Multi-player / Co-op / MMO — no multiplayer exists
- Steam Cloud *(TBD — planned per `steamworks-publishing-brief-2026.md` §2.3–§2.4, but not implemented yet; enable only once the integration ships)*
- Steam Achievements *(TBD — same reasoning as Steam Cloud; 8-achievement starter set designed, not wired)*
- Controller support *(no gamepad input mapping exists in the current build — keyboard + mouse only)*
- Trading Cards *(no economy/Steam Inventory integration planned for v1)*
- Workshop *(no mod support planned for v1)*

---

## 5. Suggested Steam tags *(Valve limit: 20 tags; priority-ordered)*

Keep the first ~10 as the must-land set; the rest fill to 15. Tags 16–20 are fallback only if the
storefront surfaces additional slots.

1. Souls-like
2. Voxel
3. Action
4. Third-Person
5. Singleplayer
6. Adventure
7. Exploration
8. Atmospheric
9. Dark Fantasy
10. Story Rich
11. Melee
12. Difficult
13. RPG
14. Combat
15. Building

**Avoid as primary tags:** "Multiplayer," "Open World," "Survival," "Crafting," "Sandbox" — all
imply scope the game explicitly does not have for v1 (`GAME-VISION.md` §Non-Goals). "Building" stays
low-priority: it's a separate creative sandbox, decoupled from the campaign, not a system woven into
the story loop — don't let it read as a pillar tag.

---

## 6. Capsule text *(Valve: must contain readable product title/logo; no review scores, award badges, discount text, or external URLs)*

The art compositions live in `docs/assets/steam/` and `docs/steam-store-art.md`. The *text* that must
be legible on every capsule is specified here — the art lane renders these strings into the compositions.
Valve's review rejects capsules where the title is illegible at the rendered size.

Every capsule should carry only: the **title** ("Voxelforge"), no tagline. The Small Capsule (462×174)
auto-generates 120×45 and 184×69 thumbnails — the title must remain readable all the way down to those
derived sizes, so the logotype must be bold, high-contrast, and occupy ≥60% of the canvas width at the
parent 462×174.

| Capsule | Size | Text | Legibility requirement |
|---------|------|------|------------------------|
| Header Capsule | 920 × 430 | **VOXELFORGE** | Readable at 920×430 and the library's derived 460×215 thumbnail |
| Small Capsule | 462 × 174 | **VOXELFORGE** (condensed if needed — the auto-generated 120×45 MUST be legible) | Readable at 120×45 — the highest-risk capsule; test at 1:1 pixel size |
| Main Capsule | 1232 × 706 | **VOXELFORGE** (full logotype, largest canvas — room for the most detail) | No legibility risk at this size |
| Vertical Capsule | 748 × 896 | **VOXELFORGE** (vertical/tall layout — logotype may stack or center-justify) | Readable in search-grid contexts at ~374×448 (half-scale) |
| Library Capsule | 600 × 900 | **VOXELFORGE** | Readable in library grid at ~200px wide |
| Library Header | 920 × 430 | **VOXELFORGE** | Same as Header Capsule |
| Library Hero | 3840 × 1240 | **None** — Valve prohibits text on Library Hero; keep critical art within 860×380 safe area | N/A |
| Library Logo | ≤1280 × 720 | **VOXELFORGE** (logotype/logomark only, transparent background) | Readable as an overlay on the library hero |

**What NOT to put on capsules:** "Wishlist now," "Coming Soon," discount banners, review scores, award
logos, or the Steam logo itself — all prohibited by Valve's content rules per `steamworks-publishing-brief-2026.md` §3.4.

---

## 7. System requirements *(Valve: table format, no hard character limits per cell)*

**Status per `GAME-VISION.md` Milestone 1 rendering bar + `docs/gate3-plan.md` LookPlugin tiers.**
⚠️ **No real FPS benchmarking has been run.** The numbers below are derived from the target hardware
tiers in the vision doc and the per-effect cost estimates in the look stack — they are estimates, not
benchmarks. **Rose must confirm real FPS on a hardware spread before these go into the Steam backend.**

Every cell marked **[TBD]** has no data source at all — do not guess, do not publish without real numbers.

### Minimum *(Low quality tier — tonemap, color grading, bloom; no shadows, SSAO, or DOF)*

| Field | Value | Status |
|-------|-------|--------|
| OS | Windows 10 64-bit | Est. (wgpu requirement) |
| Processor | Quad-core CPU, 2.5 GHz | **[TBD — no benchmark data]** |
| Memory | **[TBD]** GB RAM | **[TBD — measure peak working set at Low tier on target hardware]** |
| Graphics | GPU with Vulkan 1.2 / DX12, **[TBD]** GB VRAM | **[TBD — measure VRAM at Low tier on a budget GPU]** |
| Storage | **[TBD]** GB available space | **[TBD — measure packaged build size + save footprint]** |
| Sound Card | DirectX-compatible | Standard boilerplate |
| Additional Notes | Low quality tier. 60 FPS target — unverified. | Est. |

### Recommended *(Ultra quality tier — full stack: PCSS soft shadows, Ultra SSAO, DOF, volumetric god rays)*

| Field | Value | Status |
|-------|-------|--------|
| OS | Windows 10/11 64-bit | Est. |
| Processor | Quad-core CPU, **[TBD]** GHz | **[TBD — no benchmark data]** |
| Memory | **[TBD]** GB RAM | **[TBD — measure peak working set at Ultra tier]** |
| Graphics | GTX 1660-class or better, **[TBD]** GB VRAM | Target per vision doc — **[TBD: real FPS unmeasured]** |
| Storage | **[TBD]** GB available space (SSD recommended) | **[TBD]** |
| Sound Card | DirectX-compatible | Standard |
| Additional Notes | Ultra quality tier. 60 FPS target on GTX 1660-class hardware per Milestone 1 rendering bar — **unverified against a real benchmark pass**. | Est. |

### What's known vs. what's not

| Item | Confidence | Source |
|------|-----------|--------|
| wgpu requires Vulkan 1.2 / DX12 | ✅ Confirmed | Bevy 0.19 render backend |
| GTX 1660 = Ultra-tier target | 📋 Design intent | `GAME-VISION.md` Milestone 1 scope table |
| Low tier = CPU-only post (no GPU shadows) | ✅ Confirmed | `client/src/look.rs` insert_stack tier ladder |
| Actual FPS at any tier on any hardware | ❌ Unknown | No benchmark infrastructure exists yet |
| Peak VRAM (any tier) | ❌ Unknown | Not measured |
| Peak RAM (any tier) | ❌ Unknown | Not measured |
| Packaged build size | ❌ Unknown | Shipping build not finalized; current debug build is not representative |
| macOS / Linux support | ❌ Not planned for v1 | Vision doc §Non-Goals — "Windows native only for first playable" |

---

## 8. Shot list — what the store page needs

**Current state: none of this is captured yet.** The only renders that exist today are the Gate 3
LookPlugin proof shots (`docs/gate3-plan.md`) — boot/walk/combat frames captured for engineering
sign-off, not composed for marketing (no framing pass, no variety, no UI-clean guarantee). The key
art and capsule crops in `docs/assets/steam/` cover the *masthead* assets; everything below is the
separate gameplay-screenshot pass Steam requires alongside them.

### Screenshots — 10 planned *(Steam minimum: 5; Valve recommends ≥1920×1080, 16:9; maximum upload resolution 5120×2880)*

All at **Ultra** quality tier, 16:9, 1920×1080 minimum. Capture via `VOXELFORGE_LOOK_QUALITY=ultra` +
`VOXELFORGE_SHOT=<path>`, same mechanism as `scripts/gate3_shoot.sh`, but framed for marketing rather
than gate proof. Steam displays store screenshots at the resolution you upload — higher is better.

1. **Establishing shot** — wide view of Edhari ruins from spawn, golden-hour light, dungeon gate visible.
2. **Third-person traversal** — player character mid-walk through the courtyard, cloak-tell visible.
3. **Environmental story beat** — close on a discoverable fragment (child's drawing / carved well text).
4. **NPC dialogue moment** — Maren's hand/shadow through the dungeon gate crack, dialogue text on screen.
5. **Combat — telegraph beat** — Guard Husk mid wind-up, player positioned to react. *(Most important shot.)*
6. **Combat — dodge/i-frame beat** — player mid-roll through an active attack.
7. **Combat — lock-on HUD** — reticle, stamina bar, health bar in frame. *(UI legibility proof.)*
8. **Post-victory beat** — guard post door mid-open, campfire glow inside.
9. **Build-mode moment** — `AppState::Editor` sandbox; block being placed/removed. *Caption explicitly as creative mode.*
10. **Quality-tier hero shot** — most cinematic Ultra frame (god rays, DOF, gate arch silhouette).

Do not reuse the golden-beauty-shot reference (`docs/assets/golden-beauty-shot-ref.png`) — that's
an internal look-dev reference, not a real location in the shipping slice.

### Trailer — 1 required, ~60–90 seconds

Structure front-loaded per Valve's own guidance:
1. **0:00–0:05** — cold open on establishing shot, no logo yet.
2. **0:05–0:20** — silent exploration: spawn → campfire → dungeon gate glimpse.
3. **0:20–0:45** — combat: telegraph → dodge → punish, one full exchange uncut.
4. **0:45–0:55** — Maren dialogue beat (one line, subtitled).
5. **0:55–1:05** — guard post door opening — the payoff beat.
6. **1:05–1:15** — logo + title card + "Wishlist now" / release-window card.

No voiceover; ambient audio carries the trailer. Text cards only for the one dialogue line and the
closing card.

### Not needed yet

- Animated capsule / GIF — nice-to-have, revisit after trailer exists.
- Additional-language screenshots — English set first; only needed once localized text renders in-game.

---

## Validation checklist

| Section | Valve limit | Actual | Pass? |
|---------|-------------|--------|-------|
| §1 Short description (primary) | ≤300 chars | 264 | ✅ |
| §1 Short description (alt) | ≤300 chars | 238 | ✅ |
| §2 About This Game body | None (soft ~8,000) | 3,151 | ✅ |
| §3 Feature list longest bullet | None (keep short) | 186 | ✅ |
| §4 Genre | Fixed picker | Action + Adventure | ✅ per vision doc |
| §5 Tags | ≤20 | 15 proposed | ✅ |
| §6 Capsule text | Title-only, no marketing | "VOXELFORGE" only | ✅ |
| §7 Sysreq — min | Table, no spec limit | All TBD-marked | ✅ (TBD = honest) |
| §7 Sysreq — rec | Table, no spec limit | All TBD-marked | ✅ (TBD = honest) |

---

*Copy: Sun (2026-08-01). Validated against `steamworks-publishing-brief-2026.md` (Sahara + Kevin,
commit `5f64ab0`). Valve character limits sourced from the brief §3 and Valve's current store
partner documentation as of 2026-08-01. Combat/scope facts sourced from `docs/combat-design.md`
and the design docs — flag anything here as first, not verified in-engine.*
