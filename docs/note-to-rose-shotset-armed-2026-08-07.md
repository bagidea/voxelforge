# Shotset is armed for the green build — one command, 8 plates

**Written:** 2026-08-07, Poppy (capture lane).
**Status:** wired and gate-proven. **No deliverable frame has been shot.** Holding for
Rose's green.

Yamamoto's two fixes are in `client/src/` and wired into the capture harness. The only exe
on disk is still the 19:53 binary (`target-flamingo/release/voxelforge.exe`,
sha `BF344EE77A050BA486391D00DF68B8F9C6561F9ABC3A3FDD17558123F5A4D3A3`), which predates
both — so a plain GO run refuses to fire.

### What HAS been run off the 19:53 exe (correcting an earlier "nothing has been shot")

Scratch runs exist and none of them produced a deliverable. Naming them because "nothing
has been shot" was too broad a claim to make about a repo several lanes are working in:

- `_poppy_shotset_before/` — the **baseline set**, all 8 plates, shot on purpose from the
  frozen 19:53 exe. The old binary is the *point* of this pass, not a mistake.
- `_poppy_shotset_probe/` — an s1-vista determinism probe off the same exe.
- `_poppy_gatetest/` — **mine, and a mistake.** I meant to check that the BEFORE pre-flight
  refuses; I passed `-AllowOldExe -AllowStale` to get past the binary gate, and once the
  baselines were all present the pre-flight passed and the run went on to shoot all 8
  plates as a null A/B. Deleted; no artifact survives. The lesson is in the harness now:
  **verify a gate with `-DryRun`, never by disarming the gate in front of it.**

All of these live under `_*/`, which `.gitignore` excludes — none of them is committed.
`_poppy_beauty/final/*.png`, the actual deliverables, are untouched.

## What changed in the harness

`scripts/_poppy_shotset.ps1`:

1. **s3-clash is in the canonical set**, no longer gated behind `-ClashEnv`. Its env is
   `VOXELFORGE_ANIM_POSE=clash` (was `attack`, which only ever posed the player).
   `-ClashEnv "K=V"` still merges extra knobs on top for one-off variants.
2. **Content gate per plate.** A saved PNG proves the swapchain was grabbed, not that the
   subject was in it. s3-clash hard-fails unless the runlog carries *both*
   `ANIM_RIG_WEAPON spawn actor=Player` and `actor=Husk` — a one-actor "clash" cannot
   pass silently. `hero`, `s1-vista`, `s4-raking` and `grade-vista` demand the Player
   marker: **hero is the plate that proves the capsule fix**, and the fix is "the player
   is `Rigged`", so an unrigged player there means the capsule is back while every other
   check (PNG saved, no panic, exit 0) still passes. It used to only print a warning.
   The three gate3 plates have no log on disk to prove the marker fires for them, so they
   get the `RIGS … capsule=` report line without a hard fail — a guessed marker that never
   fires would block the whole GO run.
3. **BEFORE side is gated, and gated up front.** Every plate's baseline is checked *before
   a single exe is launched*; any miss refuses the run. Two holes closed at once: a run
   could previously print `MISS` on all 8, write `DONE=FINISH_PARTIAL` and still **exit 0**
   (`$fails` counted shot failures only), and it only discovered the problem after burning
   8 shots. An unpaired plate now also forces exit 1. `-BeforeSide` is exempt (that mode
   *creates* the baselines); `-AllowMissingBefore` is the deliberate override.
   This matters more than it looks: the baseline set lives in `_poppy_shotset_before\`,
   which `.gitignore` excludes as `_*/`. It is scratch. It vanished and was rebuilt twice
   during one hour of this session — a GO run started in that window would have shipped
   7 of 8 pairs and reported success.
4. **`-DryRun` no longer trips the old-exe / stale / missing-baseline gates** (it never
   launches the exe), so the wiring can be proven while the build lane is busy. A live
   shoot still hard-refuses. **`-DryRun` is how you test a gate** — do not disarm a gate
   with `-AllowOldExe` to "see if the next one fires"; the run continues past it and
   shoots.
5. **Newer-exe hint.** The default `-Exe` is target-flamingo's; if a fresher release exe
   exists in another lane's target dir the script names it and prints the `-Exe` flag.
6. **`-Only a,b` fix.** Under `powershell -File` that arrived as one un-split string and
   died with "no shots selected". Split on comma now; both invocation styles behave alike.

Nothing here invokes cargo or writes inside a target dir — it takes a frozen copy of the
exe first, so it cannot hold a lock on a live linker output.

## The capsule needs nothing at capture time

`dodge_ghost_flash` skips a player carrying `anim::Rigged` (`client/src/dodge_parry.rs:284`),
so it stops un-hiding what `attach_rigs` hid. Always-on, no flag. The harness only *reports*
it (`capsule=hidden (Rigged)` vs `AT RISK -- player never rigged`).

## GO command, once the build is green

The flamingo chain (`--target-dir target-flamingo`) writes to exactly the path the shotset
defaults to, so on a green flamingo build this is the whole thing:

```powershell
powershell -File scripts\_poppy_shotset.ps1
```

It shoots all 8 plates off one binary, de-HUDs each, builds the before/after pair dirs,
and runs `regrade.py` + the contact sheet. If the green build lands in plain
`target\release` instead, the script prints the `-Exe target\release\voxelforge.exe` line
to pass. Expected new deliverables: `s2-hero-medium` (capsule gone) and `s3-husk-clash`
(blades meeting) move from re-shoot-queued to shippable — see
`_poppy_beauty/final/SHOTS.md`.

**Check `_poppy_shotset_before\after\` has 8 PNGs first.** It is gitignored scratch. If it
is thin, `-BeforeSide -NoRegrade` rebuilds it (~4 min) — and the GO run refuses until it is
whole, so a thin baseline set costs you a wait, not a bad pass.

## Proof

- Full 8-plate `-DryRun`: exit 0, all 8 plates resolve, `BEFORE pre-flight -- all 8
  baseline(s) present`, and `_poppy_shotset_dry/manifest.json` reads
  `s3-clash → VOXELFORGE_ANIM_POSE=clash`, `hero → expect ANIM_RIG_WEAPON spawn actor=Player`.
- Live-shoot attempt with the 19:53 exe and no override: exit 1, refused at the binary
  gate, no exe launched, no output tree, no stray process.
- BEFORE pre-flight, both directions: with 7 of 8 baselines missing (caught during a real
  window when another lane was mid-rebuild) → exit 1, no output tree, no `vfprobe.exe`
  copied. With one baseline hidden → same refusal, `MISS hero`, `REFUSING TO SHOOT: 1/8`.
  Baseline restored after the test.
- Content gate unit-tested against the real 19:53 `s3-husk-clash.log` (passes, rigs
  `Husk,Player`), a synthetic Player-only log (fails `missing[...actor=Husk]`), and an
  empty log (fails both) — and does not false-fail plates that declare no `Expect`.
- Exit accounting unit-tested: all-paired → `FINISH_OK` exit 0; one unpaired → exit **1**
  (was 0); `-AllowMissingBefore` and `-BeforeSide` → exit 0; a shot failure still wins.
- Evidence for which plates may carry a hard `Expect`: `actor=Player` **and** `actor=Husk`
  both appear in `_poppy_beauty/final/{s1-village-wide,s2-hero-medium,s4-vista-raking,
  s3-husk-clash}.log` and `_poppy_shotset_before/raw/grade-vista.log`. No gate3 log exists,
  hence no hard `Expect` there.
