# Voxelforge — Game Vision

> **North-star pillars document.** One page, skimmable. Last updated: 2026-07-26.

---

## One-Line Pitch

> *A story-driven soulslike built inside a living voxel world — the weight of Elden Ring, the creativity of Minecraft, seen through a third-person lens.*

---

## Design Pillars

### 1. Story First, Sandbox Second
The world has a narrative spine. Every region, dungeon, and NPC exists to serve the campaign. Players explore a hand-crafted world with authored beats, not a procedurally generated void. Creativity tools (building, mining) amplify the story — they don't replace it.

### 2. Souls Weight in a Toy World
Combat is deliberate, stamina-gated, and readable. Every enemy telegraphs its attack. Every dodge costs something. Death is a teaching tool, not a punishment spiral. The visual language is blocky and cozy; the combat feel is grounded and demanding — that tension is the hook.

### 3. Third-Person Connection
The player character is always visible. The camera pulls back to reveal the world, connects the player to their avatar, and makes every building act feel personal. First-person is not on the roadmap.

### 4. AAA Light in Block Geometry
Voxel geometry stays hard-edged and un-smoothed — that is the identity. The shader stack (PBR, golden-hour key light, god rays, PCSS soft shadow, GI bounce, DOF) does the heavy lifting. The contrast between toy-simple geometry and cinematic lighting *is* the visual signature. Reference: `docs/assets/golden-beauty-shot-ref.png`.

### 5. The World Remembers Your Shape
Player-placed and player-destroyed blocks persist meaningfully. You can wall off a dungeon corridor, bridge a gap, or block an enemy's path. The world is not decorative — it is a tool and a record of your journey.

### 6. Craft for Purpose, Not Complexity
Building is fast and tactile. No multi-step crafting chains, no recipe memorisation. The system should feel like sketching, not engineering. Depth comes from *what* you build (narrative triggers, combat advantages, shortcuts), not *how many steps* it takes.

---

## Core Gameplay Loop

```
Explore voxel world
  → Discover story fragment / boss gate
    → Gather materials / build a path or shelter
      → Engage in soulslike combat (dodge · stamina · big readable enemies)
        → Defeat boss → unlock next region / story chapter
          → Repeat at higher stakes
```

Rest points (campfires) reset enemies and restore health. Death returns you to the last campfire with full enemy respawn — no item loss.

---

## Target Look

Warm key-lit golden hour. Walnut-and-honey palette (`#6B4A2E` wood, `#F4B860` sun, `#C88A4A` bounce, `#4FC9D6` teal accent). Hard voxel edges wrapped in soft photographic lighting. Think: a cozy kitchen at 5 pm, suddenly a boss walks in.

Visual reference locked: `docs/assets/golden-beauty-shot-ref.png` · Look Bible: `docs/look-bible.md`.

---

## Tech Stack — Locked

| Layer | Choice | Notes |
|---|---|---|
| Language | **Rust** | Memory safety, performance, no GC pauses |
| Engine | **Bevy** | ECS-native, Rust-first, no royalties |
| Renderer | **wgpu (WebGPU)** | Cross-platform GPU; path to web export later |
| Voxel layer | Custom chunk system in Bevy | Built on top of Bevy ECS; no third-party voxel crate required for v1 |

> **Note on Godot:** Godot appears on the office wallpaper and in unrelated projects. It is **not** used in Voxelforge. This stack is decided — do not reopen the engine question.

---

## Vertical Slice — Milestone 1 Scope

**Goal:** A single playable chapter that proves the loop end-to-end. Target playtime: 15–20 min.

| Area | Deliverable |
|---|---|
| World | 1 hand-crafted region (village ruins + 1 dungeon hub), ~48×48 block footprint |
| Story | Opening cinematic + 3 NPC dialogues + 1 climactic boss encounter |
| Combat | Player: dodge · light attack · heavy attack · stamina bar · death/respawn |
| Enemies | 2 enemy archetypes (melee grunt + mid-boss with 2-phase attack) |
| Building | Place / destroy blocks; no crafting UI required |
| Camera | Third-person with lock-on toggle |
| Rendering | Ultra tier at 60 fps on GTX 1660+ (golden-beauty-shot gate PASS) |

---

## Non-Goals for v1

- **No co-op / multiplayer** — single-player campaign only; netcode is future work.
- **No procedural world generation** — hand-crafted map only.
- **No crafting system** — gather blocks and place them, nothing more.
- **No inventory / loot depth** — 1 weapon type; upgrade is story-gated.
- **No vehicles, mounts, or traversal gadgets.**
- **No Lite / Web tier** — Ultra desktop is the only v1 target platform.
- **No economy, marketplace, or progression meta.**

---

*Owner: CEO. Design lead: Monanisa. Engine spike: Poppy. Document is a living brief — revisit after each milestone playtest.*
