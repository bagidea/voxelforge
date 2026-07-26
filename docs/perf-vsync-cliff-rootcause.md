# Perf root-cause: the "447 → 60 fps @ 900 chunks" cliff is a VSync artifact

**Verdict:** the 60 fps reading is the monitor's 60 Hz refresh cap, **not** a VRAM/driver
throttle or a real load cliff. Under VSync the GPU finishes each frame in ~2.6–5.8 ms then
idle-waits for vblank, so the frame *presents* at 16.7 ms (= 1000/60). Turn VSync off and the
same 900-chunk load runs 170–315 fps.

## Live repro (target/release/voxelforge_bench.exe, same load & camera, START_SIDE=30)

| chunks | quads | VSync (A) | NoVSync (B) |
|-------:|------:|-----------|-------------|
| 900  | 720,481 | 59.4 fps / **16.85 ms** | 172.5 fps / **5.80 ms** |
| 961  | 769,967 | 60.2 fps / 16.62 ms | 101.2 fps / 9.89 ms |
| 1024 | 820,059 | 59.6 fps / 16.78 ms | 90.9 fps / 11.00 ms |

Run A: `VOXELFORGE_BENCH=1 VOXELFORGE_PRESENT=vsync VOXELFORGE_START_SIDE=30`
Run B: `VOXELFORGE_BENCH=1 VOXELFORGE_PRESENT=novsync VOXELFORGE_START_SIDE=30`

## Why this proves it's the 60 Hz cap, not a throttle

1. **Frame time ≪ budget.** NoVSync @900 = 5.80 ms; VSync @900 = 16.85 ms. The extra
   ~11 ms is pure vblank idle — the GPU wastes ~65% of the frame just waiting. A real
   VRAM/bandwidth throttle would show a *high* GPU frame time, not a padded one.
2. **VSync readings are FLAT across load.** Reference `cliff_vsync.out`: 16.71 / 16.92 /
   16.73 / 16.71 / 16.60 / 16.74 ms as quads go 583k → 820k. A hardware throttle scales
   with work; a refresh cap does not. Dead-flat 16.7 ms = refresh-locked.
3. **NoVSync scales with load** (172 → 101 → 91 fps as chunks grow) — that's the *real*
   GPU cost curve, and it never touches the 16.6 ms VSync wall at 900 chunks.

The "447" in the original report is the NoVSync ramp ceiling (`cliff_fullramp.out` early
phases ≈ 448 fps). "447 → 60" was never a cliff — it's just VSync-off vs VSync-on numbers
placed side by side.

> Cold-boot vs warm ramp accounts for B's 172 fps here vs ~315 fps in `cliff_immediate.out`
> (ramped from side 1, pipelines/atlas warm). Both are ≫ 60; the conclusion is unchanged.

## Proposed fixes

1. **Bench: already correct — keep NoVSync default.** `main.rs:172`
   (`_ if cfg.bench => PresentMode::AutoNoVsync`) already reads the true ceiling. No change
   needed; just don't let anyone "fix" the bench to VSync and re-introduce the phantom cliff.
2. **HUD: show frame-time ms so VSync stops hiding headroom.** In interactive mode the
   default is `AutoVsync` (`main.rs:173`), which pins the HUD at "FPS 60" and buries all
   headroom. Add median frame-time to the HUD line so 60 fps @ 2.6 ms is visibly different
   from 60 fps @ 16 ms:

   ```rust
   // in fn hud(), alongside the FPS lookup
   let ft = diagnostics
       .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
       .and_then(|d| d.smoothed())
       .unwrap_or(0.0);
   // ...
   format!("FPS {fps:.0} ({ft:.1} ms)  |  chunks {chunks}  |  quads {quads}  |  ...")
   ```

   Optional: a `~` or "vsync-capped" tag when `fps ≈ refresh && ft < budget` so nobody reads
   a 60 fps HUD as "at the limit" again.
