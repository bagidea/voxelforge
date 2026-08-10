# BUILD LOCK — target-flamingo release (2026-08-07 ~02:30)

**Rose is running the SOLE release build into `target-flamingo/`** to refresh
`target-flamingo/release/voxelforge.exe` for Poppy's new shot set.

**Do NOT launch a concurrent `cargo build`** while this is running. The ship
`profile.release` is fat-LTO + codegen-units=1, so the link step spikes RAM.
Two fat-LTO links at once → `link.exe 0xc0000142 / STATUS_DLL_INIT_FAILED`
(everyone dies). This was already happening at 02:20–02:24 (three concurrent
builds — I cleared them).

- Build: `cargo build --release -p voxelforge --bin voxelforge --target-dir target-flamingo -j 2`
- Expected wall-time: ~20 min (the LTO link).
- I will confirm green here (exe mtime moved + `grep '^error'` clean) before
  declaring the lock released.

If you need a build before then, ping me rather than stacking. — Rose
