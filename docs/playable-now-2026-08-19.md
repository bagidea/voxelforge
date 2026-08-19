# Voxelforge — what's actually playable right now (2026-08-19)

Ran the **existing** release binary — no rebuild triggered. Poppy holds the build lane
today; this report is proof-of-boot + proof-of-play on the binary CEO already has on
disk.

**Binary tested:** `target/release/voxelforge.exe` — 82,093,056 bytes, built
2026-08-18 08:01. Binary source snapshot: commit `d7f95bd` (the nearest commit to that
build timestamp, to the minute). Current `HEAD` on `poppy/native-only` is **9 commits**
ahead of that — see "What's newer than this binary" below.

**Boot result: it booted clean. No panic, no crash, exit code 0 both times.** (There
was an old note that `voxelforge.exe` panicked on every boot — that is **not** the
state of this binary; verified live, not assumed from source.)

## Run it yourself

```powershell
cd E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
.\target\release\voxelforge.exe --play
```

`--play` drops you straight into grounded walk mode on the Edhari map instead of the
block editor. Omit it and you land in the editor (`Tab` swaps Edit/Play at any time).
The game auto-loads `quest_save.json` if one exists next to the exe (this dev machine
already has one from a prior run, so you may land mid-quest instead of a fresh start —
delete that file first for a clean start).

## Controls (as shown by the in-game HUD, this exact binary)

| Action | Key |
|---|---|
| Move | `W` `A` `S` `D` |
| Jump (grounded) / fly up | `Space` |
| Fly down (after toggling fly) | `Shift` |
| Sprint / fast-fly | hold `Ctrl` |
| Toggle fly / walk | `F` |
| Look | mouse (click into the window first — cursor auto-locks) |
| Dig block (Play mode) | `B` |
| Place block (Play mode) | `N` |
| Use / interact | `H` |
| Open bag | `I` |
| Select hotbar slot 1–9 | `1`–`9` |
| Attack | `Mouse Left` |
| Block | hold `Mouse Right` |
| Toggle Edit / Play | `Tab` |
| Grid-fill (editor) | `G` |
| Load map | `F5` |
| Save | `F6` |
| Load (quick) | `F9` |

Note this build **changed** dig/place/attack from the previous playable-now doc
(2026-08-17): that build used `Mouse Left`/`Mouse Right` for break/place with no
separate attack key. This build split it — `B`/`N` dig/place, `Mouse Left`/`Mouse
Right` (held) attack/block. If you played the 08-17 build, don't reach for
Mouse-Left-to-mine out of habit.

## What we actually saw, driving it

This report used **headless single-frame captures** (`VOXELFORGE_SHOT=path.png`,
in-engine screenshot at 3.2s, auto-exit at 4.4s) rather than a full hand-driven video
session, to keep the proof-of-boot turnaround tight. Two runs, two different launch
modes, both real (not mocked) world state:

1. `--play` (boots into whatever `quest_save.json` left off — mid-dialogue at the
   campfire): booted clean, loaded story (`act1.json`, 5 quests/3 NPCs/9 regions),
   loaded map `edhari.json` (11,063 blocks, 2×2 chunks), spawned player at
   `(32.5, 2.6, 32.5)`, spawned an `ENEMY_SPAWN kind=sentinel` at `(32.0,1.0,25.0)`,
   spawned NPCs Elder Maren and Toma, resumed a saved quest (`QUEST_LOAD ok
   flags=6 quests=1`), fired the `dlg_maren_first_call` dialogue trigger, spawned
   ambient FogVolume ("god-ray medium"). 8 chunks / 16,105 quads streamed by the
   3.2s mark.
2. `--play-demo` (scripted walk+respawn proof, same map): same clean boot, and by
   3.2s had streamed **more** world than the first run — 12 chunks / 19,636 quads —
   confirming world streaming is live and responds to movement, not a static shot.

Both captures show the same dialogue moment (Elder Maren, "A Voice in the Stone" quest,
"You [?] by the fire. Stay still.") — the `[?]` is a real rendering gap: a glyph the
bundled font can't draw, not a redaction. Worth a look for whoever owns UI text/fonts.

**A concrete, log-confirmed finding, not a guess:** both boots printed
`BLOCK_ART tile_px=64 != 16 — file set ignored, procedural tiles kept`. This binary
predates the fix — the next commit after this build (`55e08e3`, "the manifest decides
the tile size, so the 64px art can reach a frame") is what closes it. That's why the
blocks in the screenshots below look like flat procedural red/orange/tan placeholder
tiles, not the authored 64px PBR art — this is expected for this binary, not a bug in
what you're about to play.

## Evidence

Screenshots (`docs/assets/playable/`), both from real boots of this exact binary, no
demo/mock data beyond the launch mode itself:

- `boot-proof-2026-08-19.png` — `--play` boot, Elder Maren dialogue box open, 8 chunks
  streamed, procedural placeholder block art visible.
- `boot-proof-outdoor-2026-08-19.png` — `--play-demo` boot, same dialogue moment, 12
  chunks / 19,636 quads streamed (world-streaming confirmation).

## What's newer than this binary (don't expect these in the .exe you just ran)

9 commits sit between the build baseline (`d7f95bd`, 2026-08-18 08:01) and current
`HEAD` on `poppy/native-only` (`13419e6`, 2026-08-19 15:37) — **all of today's
sky/water/palette work**, exactly as flagged before this task started:

- **64px authored PBR block set actually reaching the render path** — `55e08e3` fixes
  the tile-size manifest bug this binary is still hitting (see finding above), then
  `76ee173`/`1fd5145`/`896c80f` land the matmaps rig, and `4d279af`/`13419e6` land the
  authored 64px art itself + a beach-dusk rig + a CEO sunset reference frame. None of
  this is in the binary you just ran — the blocks you saw are the old procedural
  placeholder tiles.
- **River water + metal blocks** (`2b64299`) — real surface mesh, PBR, and map wiring
  for water and metal block types. This binary has no water/metal blocks at all.
- **Art-gap grading against a CEO reference** (`51dfed2`) — a numeric rubric that can
  now score sky, not in this binary.

Everything from the prior playable-now doc (2026-08-17) that was already newer than
that build — enemy AI, VFX pass, audio pass, look/lighting grading — **is already
baked into this binary** (that gap has closed): this run's own logs show a live
`ENEMY_SPAWN kind=sentinel`, working quest dialogue/save-load, item-bag hotbar, and a
FogVolume god-ray pass, none of which existed in the 08-17 binary.

## Process cleanup

Both `voxelforge.exe` runs auto-exited on their own (headless shot mode has a built-in
4.4s exit) — confirmed with `tasklist` afterward: no `voxelforge.exe` process left
running.
