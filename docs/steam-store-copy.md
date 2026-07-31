# Voxelforge — Steam Store Page Copy

> English copy for the Steam store page. Source of truth for what the game actually has:
> `docs/GAME-VISION.md` (pillars + Milestone 1 scope), `docs/first-playable-loop.md` (the only
> content slice that exists today), `docs/combat-design.md` (shipped mechanics), `docs/gate3-plan.md`
> (the LookPlugin quality tiers). Nothing below sells a feature that isn't in that scope.
> Art direction / capsule assets: `docs/steam-store-art.md` (owned by the art lane — not touched here).
> Owner: Monanisa (Design). Last updated: 2026-08-01.

---

## 1. Short description (store search snippets, ≤300 characters)

> A voxel world lit like a photograph, with the weight of a soulslike underneath. Explore the ruins
> of Edhari, read every enemy's telegraph, and fight with real stamina and real consequence. Blocky
> geometry, cinematic golden-hour light — third-person, single-player.

(264 characters, counted with spaces.)

**Alt (for A/B testing on the storefront), 238 characters:**

> Blocky world, honest combat. Explore a golden-hour voxel village, dodge on a stamina budget, and
> read every telegraph before you punish it. A story-driven third-person soulslike where the toy-simple
> geometry hides real, deliberate weight.

---

## 2. About This Game

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

**A world that remembers what you did to it.**
- Place and destroy blocks to shape the space around you — wall off a corridor, bridge a gap, or
  clear your own path. It's a tool for solving the world, not a crafting minigame: no recipes, no
  menus, just pick up and place.
- Rest at campfires to heal and reset the fight — your first one lights the moment you arrive, and a
  second unlocks the moment you win.

**Built to look like a photograph of a toy.**
- Hard-edged voxel geometry stays exactly that: blocky, readable, unapologetically toy-like.
- A full PBR lighting stack — golden-hour key light, soft PCSS shadows, ambient occlusion, bloom,
  distance haze, and depth-of-field at higher settings — wraps that geometry in cinematic,
  photographic light. Four scalable quality tiers (Low → Ultra) let you choose your balance of
  fidelity and frame rate, right down to volumetric god rays at the top end.

---

## 3. Feature list (bullet form, for the feature-list module)

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
- **Place and destroy blocks.** A fast, tactile building tool for shaping your path through the world.
- **Four graphics quality tiers (Low → Ultra).** Scalable post-processing stack — tonemapping, color
  grading, bloom, soft shadows, ambient occlusion, depth of field, distance fog, and volumetric god
  rays at Ultra.
- **Native desktop build.** Built in Rust on Bevy/wgpu — no browser runtime, no plugin required.

---

## 4. System requirements

**Status: estimated, not benchmarked.** These numbers are derived from the LookPlugin quality tiers
in `docs/gate3-plan.md` and the GTX 1660-class Ultra-tier target in `docs/GAME-VISION.md`'s Milestone 1
scope table — nobody has run a real FPS pass on a spread of hardware yet. **Treat every number below as
a placeholder pending Rose's actual FPS benchmarking; do not lock these in the Steam backend until she
confirms real numbers.**

### Minimum (Low quality tier — tonemap, color grading, bloom only; no shadows/SSAO/DOF)

| | |
|---|---|
| OS | Windows 10 64-bit |
| Processor | Quad-core CPU, 2.5 GHz (e.g. Intel Core i3-8100 / AMD Ryzen 3 1200) |
| Memory | 8 GB RAM |
| Graphics | GPU with Vulkan 1.2 or DirectX 12 support, 2 GB VRAM (e.g. GTX 960 / RX 560) |
| Storage | 4 GB available space |
| Additional Notes | Low quality tier targeted at 60 FPS — unverified |

### Recommended (Ultra quality tier — full stack incl. PCSS soft shadows, SSAO, DOF, volumetric god rays)

| | |
|---|---|
| OS | Windows 10/11 64-bit |
| Processor | Quad-core CPU, 3.5 GHz (e.g. Intel Core i5-9600K / AMD Ryzen 5 3600) |
| Memory | 16 GB RAM |
| Graphics | GTX 1660 or better, 6 GB VRAM |
| Storage | 4 GB available space (SSD recommended) |
| Additional Notes | Ultra quality tier targeted at 60 FPS on GTX 1660-class hardware per the Milestone 1 rendering bar — unverified against a real benchmark pass |

*(Storage figure is a placeholder — the shipping build size isn't final; revisit once the vertical
slice is packaged.)*

---

## 5. Suggested Steam tags

In priority order — pick up to the storefront's limit, keep the first ~10 as the ones that must land:

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

Avoid: "Multiplayer," "Open World," "Survival," "Crafting," "Sandbox" as primary tags — all imply
scope the game explicitly does not have for v1 (see `GAME-VISION.md` §Non-Goals). "Building" stays
low-priority since it's a traversal tool here, not a system deep enough to tag as a pillar.

---

## 6. Shot list — what the store page needs

**Current state: none of this is captured yet.** The only renders that exist today are the Gate 3
LookPlugin proof shots (`docs/gate3-plan.md`) — boot/walk/combat frames captured for engineering
sign-off, not composed for marketing (no framing pass, no variety, no UI-clean guarantee). The key
art and capsule crops in `docs/assets/steam/` cover the *masthead* assets; everything below is the
separate gameplay-screenshot pass Steam requires alongside them.

### Screenshots — 10 needed (Steam minimum is 5; 10 gives room to drop weak ones)

All at Ultra quality tier, 16:9, 1920×1080 minimum (Steam recommends up to 5120×2880 source so it can
downscale) — Steam displays screenshots as 1280×720 minimum. Capture via `VOXELFORGE_LOOK_QUALITY=ultra`
+ `VOXELFORGE_SHOT=<path>`, same mechanism as `scripts/gate3_shoot.sh`, but framed for marketing rather
than gate proof (no HUD-only crops, hold longer for a clean composition beat).

1. **Establishing shot** — wide view of the Village of Edhari ruins from the collapsed-shelter spawn,
   golden-hour light, dungeon gate visible in the distance. Sells the "cozy toy world" read at a glance.
2. **Third-person traversal** — player character mid-walk through the courtyard, cloak-tell visible,
   camera at the standard 60° pull-back. Sells "you always see your character."
3. **Environmental story beat** — close on one of the three discoverable fragments (child's drawing,
   or the well-side carved text) with the character in frame reading/approaching it.
4. **NPC dialogue moment** — the dungeon gate scene with Elder Maren's hand/shadow visible through the
   crack, dialogue text on screen. Sells "there's a story here."
5. **Combat — telegraph beat** — Guard Husk mid wind-up (arm raised, glint), player positioned to
   react. This is the single most important shot: it sells the soulslike read in one frame.
6. **Combat — dodge/i-frame beat** — player mid-roll through/past an active attack, motion-blurred or
   timed at the roll's peak. Sells the mechanical hook, not just the aesthetic.
7. **Combat — lock-on HUD** — reticle on the Husk, stamina bar visibly drawn down, health bar in frame.
   Sells "there's a real system here," and doubles as the UI-legibility proof shot.
8. **Post-victory beat** — the guard post door mid-slide-open animation, campfire glow just inside.
   Sells "the world reacts to what you do," pillar 5.
9. **Building/placement moment** — player placing or removing a block mid-traversal (a bridge or a
   wall), framed so the toy-geometry read is obvious. Sells the one non-combat system in scope.
10. **Quality-tier hero shot** — the single most cinematic frame the Ultra tier can produce (volumetric
    god rays through the dungeon gate arch, DOF on a foreground silhouette). This is the "screenshot
    that gets screenshotted" — spend the most iteration time here.

Do not reuse the golden-beauty-shot reference kitchen composition (`docs/assets/golden-beauty-shot-ref.png`)
verbatim — that's an internal look-dev reference, not a real location in the shipping slice.

### Trailer — 1 required, ~60–90 seconds

Steam accepts multiple trailers, but only one is required to launch a page. Structure, front-loaded
per Steam's own guidance (first 5 seconds decide if a viewer keeps watching):

1. **0:00–0:05** — cold open on the establishing shot (screenshot #1's motion equivalent), no logo yet.
2. **0:05–0:20** — silent exploration beat: spawn, walk toward the campfire, glimpse of the dungeon
   gate. Let the lighting stack sell itself before any combat or text appears.
3. **0:20–0:45** — combat: telegraph → dodge → punish, at least one full exchange shown uncut so the
   stamina/dodge loop reads as a real system, not a highlight-reel cut.
4. **0:45–0:55** — the Maren dialogue beat (one line, subtitled) — story hook, fast.
5. **0:55–1:05** — guard post door opening — the payoff beat.
6. **1:05–1:15** — logo + title card + "Wishlist now" / release-window card.

No voiceover needed — let ambient audio (the underground rhythmic sound, the wind-up grunts, the door
slide) carry it; add text cards only for the one dialogue line and the closing card.

### Not needed yet

- Animated capsule / GIF — nice-to-have, not required to launch the page; revisit after the trailer
  exists (it's typically a trailer excerpt anyway).
- Additional-language screenshots — ship with the English set first; only needed once localized text
  actually renders in-game.

---

*Copy: Monanisa. Combat/scope facts sourced from Sahara's `docs/combat-design.md` and the design docs
above — flag anything here as first, not verified in-engine.*
