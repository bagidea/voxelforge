# Voxelforge — what's actually playable right now (2026-08-17)

Ran the existing release binary on the dev machine, drove it with real keyboard/mouse
input (not a script/demo flag), and captured the result. No build was triggered —
Poppy owns the build lane today.

**Binary tested:** `target/release/voxelforge.exe` — 80,784,384 bytes, built 2026-08-14 13:29.
It booted clean on the first try (no panic, no crash). `target/debug/voxelforge.exe`
was not needed as a fallback.

Binary source snapshot: commit `eea8bdd` (the nearest commit before the 13:29 build
timestamp). Current `HEAD` (`poppy/native-only`) is 77 commits ahead of that — see
"What's newer than this binary" below.

## Run it yourself

```powershell
cd E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
.\target\release\voxelforge.exe --play
```

`--play` drops you straight into grounded walk mode on the Edhari map instead of the
block editor. Omit it and you land in the editor (`Tab` swaps Edit/Play at any time).

Useful launch flags/env vars this binary understands (from `client/src/main.rs` @ `eea8bdd`):

| Flag / env var | Effect |
|---|---|
| `--play` / `VOXELFORGE_PLAY=1` | boot straight into Play (grounded walk) |
| `--play-demo` / `VOXELFORGE_PLAY_DEMO=1` | scripted walk+respawn proof, no input needed |
| `--combat-demo` / `VOXELFORGE_COMBAT_DEMO=1` | scripted combat proof |
| `--quest-demo` / `VOXELFORGE_QUEST_DEMO=1` | scripted quest proof |
| `VOXELFORGE_SHOT=path.png` | headless: take one screenshot ~3.2s in, exit ~4.4s |
| `VOXELFORGE_NOHUD=1` | hide all screen-space HUD (for clean captures) |
| `VOXELFORGE_MAP_LOAD=path.json` | load a specific map instead of the default |

## Controls

The game has a rebindable input map (`client/src/input_map.rs`); these are the factory
defaults it ships with (also what `editor_config.json` resets to if deleted).

| Action | Key |
|---|---|
| Move | `W` `A` `S` `D` |
| Jump (Play mode) | `Space` |
| Sprint / fast-fly | hold `Ctrl` |
| Fly up / down (noclip fly) | `Space` / `Shift` (after toggling fly) |
| Toggle fly / walk | `F` |
| Look | mouse (click into the window first — cursor auto-locks) |
| Break block | `Mouse Left` |
| Place block | `Mouse Right` |
| Select hotbar slot 1–4 | `1` `2` `3` `4` |
| Toggle Edit / Play | `Tab` |
| Undo / Redo | `Z` / `Y` |
| Toggle grid overlay | `G` |
| Box-fill (editor) | `G` (second press after a corner is set) |
| Quick-save world | `F5` |
| Quick-load world | `F9` |
| Release cursor / back out | `Esc` (in Play with cursor unlocked, this can drop you into Editor — see note below) |

There is no dedicated "attack" key in this build — `Mouse Left` doubles as break/attack
depending on what's targeted. There's also no in-session screenshot hotkey; stills here
were taken with an external screen-capture, not the engine's own `Screenshot` component
(that path only fires from the headless `VOXELFORGE_SHOT` env var).

## What we actually saw, driving it live

Boot log (clean, no errors): loaded `assets/story/act1.json` (5 quests, 3 NPCs, 9
regions), loaded map `maps/edhari.json` (9,537 blocks), spawned the player at
`(32.5, 2.6, 32.5)`, spawned NPC "Elder Maren", spawned a `husk` enemy nearby, entered
Play, started 3 ambient audio loops (wind/campfire/village), lit VFX at the campfire.

Driving it with real input (not a demo flag) confirmed working, in this order:

1. **Dialogue** — walked into the `spawn_shelter` quest trigger, got a real dialogue box
   from Elder Maren ("You — by the fire. Stay still."), advanced it with `Space`.
2. **Look + walk + jump** — mouse-look turned the camera, `W` moved the player forward
   (chunk count and streamed quad count went up as we moved — real world streaming, not
   a static shot), `Space` jumped mid-walk (stamina dropped from the jump).
3. **Mine + place** — held `Mouse Left` on a stone block near the campfire, it broke
   (confirmed by a visible flying-debris cube caught on camera); `Mouse Right` placed a
   block back.
4. **Explore** — walked/strafed further, streamed 20+ new chunks, reached open terrain
   and a night-lit village area with a rooftop/tower structure and a blocky humanoid
   NPC standing in it.
5. **Something hit us** — health dropped 100 → 65 and stamina 100 → 68 between two
   screenshots taken seconds apart, with a red damage-vignette visible on screen.
   **We can't honestly call this "combat"** — see the caveat below. Resting at the
   campfire (`E`) fully restored both bars afterward.
6. **A miss-click into the editor** — pressing `E` near the campfire once opened the
   Settings dialog instead of the "rest" prompt, and a follow-up `Esc` (meant to close
   the dialog) instead dropped the game out of Play into the block Editor (entity list +
   block palette UI). Recovered by clicking the Settings "Close" button, then the
   editor's "Play" button, which returned cleanly to grounded Play. Noting this because
   it's a real, reproducible UX rough edge, not a crash.

**Combat caveat (important, don't oversell this):** the `husk` spawned in the boot log,
but enemy AI — patrolling, pursuing, telegraphing, striking — landed in commits
`70570e7` (Rose's patrol/alert/pursuit module) and `da807dd` (Kevin's husk AI), both
**after** `eea8bdd`. In this binary the husk is almost certainly inert set-dressing. The
HP drop we captured is more likely fall damage or a stray script effect than a real
monster hit. We did not get a clean "attacked an enemy and it reacted" moment on camera.

## Evidence

- Video (55s, real input, no demo flags): `docs/assets/playable/voxelforge-playable-2026-08-17.mp4`
  — covers dialogue, walk, jump, mine, place, in one continuous take.
- Screenshots (`docs/assets/playable/`):
  - `01_intro_dialogue.png` — Elder Maren dialogue at the campfire
  - `02_walk_and_jump.png` — mid-walk, camera turned, stamina spent
  - `03_campfire_cursor_locked.png` — cursor-locked play view of the campfire
  - `04_mining_block.png` — block broken, wall opened up
  - `05_block_placed.png` — block placed back
  - `06_exploring_caves.png` — deeper terrain, more chunks streamed
  - `07_village_night_vista.png` — best-looking shot: night sky, tower, NPC, village silhouette
  - `08_damage_taken.png` — the HP-drop moment (see combat caveat above)

## What's newer than this binary (don't expect these in the .exe you just ran)

77 commits sit between the build baseline (`eea8bdd`, 2026-08-14 13:26) and current
`HEAD` on `poppy/native-only`. The ones that change what you'd actually see/play:

- **Texture/block atlas overhaul** — file-backed atlas + warm/cool palette + a
  second material pass (dirt/brick/lamp/red_sand/snow/water/metal) + a real `GLASS`
  block through the render path (`56e2c35`, `4bf9539`, `a437002`, `ad25b53`, `04ddca4`,
  `138e790`). The textures you saw in the captures above are the *old* atlas.
- **Enemy AI** — patrol/alert/pursuit/telegraphed-strike behaviour (`70570e7`), husk AI
  + a beauty-tour cinematic (`da807dd`), a readable anticipation→strike→recovery pose
  channel (`9809338`), three new enemy silhouettes (`da7022f`). None of this is in the
  binary we ran — see the combat caveat above.
- **Story/quest content** — Act II engine support + flag-driven story swaps (`6200168`),
  more Act I quest/dialogue content (`7307635`) on top of the Act I chain that already
  works in this build.
- **VFX pass** — impact/parry/stagger/dissolve pairs, a swing-trail showcase, landing
  dust (`8d2ef9a`, `7c3768e`, `c5d4886`, `e9b5cd4`).
- **Audio pass** — full footstep coverage across grass/wood surfaces, a 436-play audio
  proof run, zone-mixer fixes (`5f0d5b6`, `e16aa2b`, `b48db84`, `5a58965`, `42c7fd5`).
  The ambient wind/campfire/village loops you heard in this build are already present;
  footstep SFX are not.
- **Look/lighting grading** — several night-look passes (v4/v5/v6), ambient and EV100
  tuning, shadow/penumbra fixes, sky-dome work. The lighting in the captures above
  predates all of that.
- **Menu/save-load polish** — a fuller game-entrance + save/load loop (`1f8b692`,
  `84ab559`); this binary already has raw `F5`/`F9` quick-save/load, just not the
  polished flow.

## Process cleanup

`voxelforge.exe` (and the ffmpeg capture process) were both killed before this report
was written — nothing was left running on the machine.
