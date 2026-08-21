# Handoff — `client/src/main.rs` uncommitted work (for Shiba, + the feel lane)

Date: 2026-08-21 · Worktree: `_poppy_hero_wt` · Branch: `poppy/hero-rig`

## What was in the working tree

`git diff client/src/main.rs` vs HEAD was **+390 / −28**. It is **not one lane's
work** — it is three, interleaved in the same file:

| # | Lane | What it adds |
|---|------|--------------|
| 1 | **camera-spring (Shiba)** | `OrbitCam::smoothed_pivot` / `pivot_vel`, the spring-damped chase in `fly_camera`, FOV walk/run lerp + look-ahead (`CAM_SPRING_FREQ/DAMP`, `CAM_FOV_*`, `CAM_LOOKAHEAD_*`), `use bevy::camera::Projection` |
| 2 | **player-feel / tuning** | `pub(crate) mod player_tuning;` + `client/src/player_tuning.rs` (untracked), `FlyCam::time_since_grounded` / `time_since_jump_pressed` (coyote time + jump buffer), split `GRAVITY_RISE`/`GRAVITY_FALL`, accel/decel-curved horizontal `vel`, and the re-export of every bare const (`BOOM_DIST`, `PIVOT_UP`, …) through `player_tuning::` |
| 3 | **hero-rig film (poppy — mine)** | `Cfg::feel_demo`, `feel_demo_film_dir()`, `feel_demo_shots_dir()`, `FEEL_DEMO_SHOT_BEATS`, `FEEL_FILM_DT/FROM/QUIT_MARGIN`, `FeelDemoShots`, `feel_demo_shots()`, the `TimeUpdateStrategy::ManualDuration` film clock, and the three film guards (skip `play_map`, force `NOHUD`, skip `spawn_encounter`) |

**Full text of all three is preserved at**
`_poppy_hero_wt/_poppy_foreign_camspring.patch` (29,373 bytes, 569 lines) —
`git apply` it onto a clean `HEAD:client/src/main.rs` to get the tree back.

## What I changed in main.rs

**Nothing.** Zero lines.

I reverted the file (`git checkout -- client/src/main.rs`) to check whether the
hero-rig lane needed it, found that lane 3 above **is my own** film hook and that
`scripts/_poppy_herofilm_shoot.ps1` gates the binary on the
`VOXELFORGE_FEEL_DEMO_FILM` / `FEEL_FILM done` strings, then re-applied the patch
**verbatim** (`git apply` reported "Applied patch cleanly", exit 0). The tree is
byte-identical to how I found it. No borrow fix was needed — it compiles as-is.

The rig lane's own change is entirely inside `client/src/anim.rs` (+23):
a `VOXELFORGE_ANIM_RIG_OFF` `OnceLock` lever and one early `return` in
`attach_rigs`. It needs **no** main.rs hook of its own — the placeholder capsule
the BEFORE plate shows is what `main::setup` already spawns when no rig dresses
the avatar.

## One thing that is NOT mine and NOT Shiba's spring — please claim it

`client/src/water.rs` (+10 / −4) is a **Bevy 0.19 compile fix**:
`drive_water_from_sun` took `ambient: Option<Res<AmbientLight>>`, but in 0.19
`AmbientLight` is a per-camera *component* (the resource is `GlobalAmbientLight`,
which this renderer does not use). It is now
`Query<&AmbientLight, With<crate::OrbitCam>>` — the light `look.rs` actually
writes. I left it untouched because reverting it makes the crate fail to build.
Whoever owns the water lane should commit it.
