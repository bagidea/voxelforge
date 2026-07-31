# Gate 3 — LookPlugin visual gate (before → after)

**What Gate 3 proves:** The `LookPlugin` post stack (`client/src/look.rs`) is
worn by the playable game and every pixel changes from it is intentional — the
three frames below are the reviewer's sign-off surface. They replace the
now-deleted hero.rs beauty shots (which lived in a binary nobody plays) with
frames captured from the *real* game through the real controller.

## Before / after — side-by-side

| Frame   | Before (no LookPlugin or old build)      | After (LookPlugin @ High, release build)    | What changed |
|---------|------------------------------------------|---------------------------------------------|--------------|
| **Boot**  | `playable-boot.png` (876 KB)            | `gate3-after-boot.png`                      | Tonemap (AcesFitted), ColorGrading (temperature +0.10, sat +2%, midtone/highlight contrast ×1.30), Bloom (0.26 intensity), DOF (f/4.0 bokeh, focus tracked to boom), TAA,  PCSS temporal soft shadows, medium SSAO, distance haze |
| **Walk**  | `playable-walk-before.png` (877 KB)     | `gate3-after-walk.png`                      | Same post stack + avatar has walked and camera orbited from `--play-demo` driving real `ButtonInput` through the controller |
| **Combat**| *(no before — first combat frame with look)* | `gate3-after-combat.png`               | Full LookPlugin stack over the `--combat-demo` scene (husk + HUD bars visible) |

## Run commands (inside `scripts/gate3_shoot.sh`)

All three shots use the same binary (`target/release/voxelforge.exe`) and env
vars — no CLI flag leakage between invocations:

```bash
# Boot — spawn into the world, no input.
VOXELFORGE_PLAY=1 \
VOXELFORGE_LOOK_QUALITY=high \
VOXELFORGE_SHOT=docs/assets/gate3-after-boot.png \
./target/release/voxelforge.exe

# Walk — --play-demo drives W, D, mouse grab + orbit through the controller.
VOXELFORGE_PLAY_DEMO=1 \
VOXELFORGE_LOOK_QUALITY=high \
VOXELFORGE_SHOT=docs/assets/gate3-after-walk.png \
./target/release/voxelforge.exe

# Combat — --combat-demo boots to AppState::Play with husk + HUD.
VOXELFORGE_COMBAT_DEMO=1 \
VOXELFORGE_LOOK_QUALITY=high \
VOXELFORGE_SHOT=docs/assets/gate3-after-combat.png \
./target/release/voxelforge.exe
```

## How the look stack reaches the game

1. `main.rs:442` — `.add_plugins(look::LookPlugin)` is wired into the editor
   shell app builder (the path `--play`/`--combat-demo` takes).
2. The plugin inserts `apply_look_to_cameras` + `apply_look_to_sun` systems,
   both gated by `look_enabled` → `cfg.play == true` (`look.rs:175`).
3. On the first frame, `apply_look_to_cameras` finds the existing 3D camera
   (spawned by `main.rs::setup`) and inserts the tier's post components onto
   it — no new camera is spawned, no gameplay state is overwritten.
4. `apply_look_to_sun` does the same for the directional light: PCSS penumbra
   width is set at High/Ultra; `VolumetricLight` is only inserted at Ultra.
5. Default tier is `High`; `VOXELFORGE_LOOK_QUALITY` overrides it.
6. F7 cycles Low→Medium→High→Ultra→Low at runtime (manual test hook).

### What each tier enables (Highlights from `insert_stack`, `look.rs:185`)

| Effect                  | Low | Medium | High (default) | Ultra |
|-------------------------|:---:|:------:|:--------------:|:-----:|
| AcesFitted tonemap      | ✓   | ✓      | ✓              | ✓     |
| ColorGrading (golden)   | ✓   | ✓      | ✓              | ✓     |
| Bloom (0.26, NATURAL)   | ✓   | ✓      | ✓              | ✓     |
| Hardware 2×2 shadows    |     | ✓      |                |       |
| TAA                     |     | ✓      | ✓              | ✓     |
| SSAO                    |     | Low    | Medium         | Ultra |
| ShadowFilteringMethod   |     | Hard   | Temporal       | Temporal |
| Depth-of-Field (f/4.0)  |     |        | ✓              | ✓     |
| DistanceFog             |     | ✓      | ✓              | ✓     |
| VolumetricFog (god rays)|     |        |                | ✓     |

## How screenshots work

`screenshot_once` (`main.rs:2310`):
- Reads `VOXELFORGE_SHOT=<path>` → stored in `Bench::shot`
- At `t = 3.2 s` → spawns `Screenshot::primary_window()` → observes `save_to_disk(path)`
- At `t = 4.4 s` → writes `AppExit::Success` → game closes cleanly

A shot that fires before the look stack is applied (frame 0) would show the
bare scene — the `Update` schedule runs `apply_look_to_cameras` on frame 1 and
the 3.2-second settle guarantees the stack is on for every captured frame.

## Gate (mechanical)

Each shot passes when ALL of:
- exit code 0
- PNG on disk ≥ 2 KiB
- `SHOT saved to <path>` printed in stdout
- No `panicked` / `B0001` / `thread ... panicked` in stderr

The script (`scripts/gate3_shoot.sh`) gates all three shots mechanically —
zero by-eye grading.

## Running the full shoot

```bash
# Wait for Poppy's fat-LTO build to land, then shoot:
bash scripts/gate3_shoot.sh

# Override quality to Ultra (full hero stack):
QUALITY=ultra bash scripts/gate3_shoot.sh

# Test against a debug binary:
BIN=./target/debug/voxelforge.exe QUALITY=medium bash scripts/gate3_shoot.sh
```

The script waits up to 10 minutes (configurable via `WAIT_TIMEOUT`) for the
binary's mtime to pass "right now", polls every 5 seconds, then fires all three
shots back-to-back.
