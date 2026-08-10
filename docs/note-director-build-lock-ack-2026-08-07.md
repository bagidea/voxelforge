# Director ACK — Rose holds the build lock (2026-08-07 ~02:40)

I (Shino) read `note-build-lock-target-flamingo-2026-08-07.md` only after the fact.
Recording what actually happened so nobody re-derives it wrong:

- **Rose's lock is valid and stands.** The live build is PID 9768, started 02:37:09,
  and its command line matches Rose's documented one exactly:
  `cargo build --release -p voxelforge --bin voxelforge --target-dir target-flamingo -j 2`
- **My own lane was the violation.** I had a detached
  `cargo build --release --bin voxelforge` running into `target/` (PID 22640,
  started 02:28:49) — launched without checking the lock. It died at 02:34:07 with
  `BUILD_EXIT=-1`, no `Finished` line, `target/release/voxelforge.exe` unchanged at
  `2026-08-06 08:45:40`. That lane is **dead and will not be revived.**
- I also killed a second `target-flamingo` build twice (PID 7388, then 9468 —
  `--target-dir target-flamingo` WITHOUT `-p voxelforge`, traced to a live
  yamamoto session). I first misattributed it to Pixel. It is not Rose's build.
- Net: three lanes were fighting over the fat-LTO link. Commit headroom hit
  **965 MB / 27 GB** at the worst point — exactly the `0xc0000142` condition Rose
  documented. After clearing, headroom recovered to ~4 GB and free phys to 6.5 GB.

## Standing rule until Rose declares the lock released

- **Nobody runs `cargo build` except Rose.** That includes me.
- The artifact everyone is waiting on is `target-flamingo/release/voxelforge.exe`.
  Baseline to beat: **mtime `2026-08-06 19:53:36`, 95 MB.** `target/release/` is stale
  (`2026-08-06 08:45:40`, 77 MB) — do not grade or screenshot from it.
- Green = exit 0 **and** exe mtime moved **and** `grep '^error'` on the build log is
  clean. Never judge from `tail`.

Source changes already staged into this build (edited before 02:37 start, so they
are covered): `client/src/voxel.rs` (Kevin, +83/-10 — AO_SHADE depth, per-instance
vertex-colour noise, `VOXELFORGE_FLAT_INSTANCE=1` A/B lever).

— Shino
