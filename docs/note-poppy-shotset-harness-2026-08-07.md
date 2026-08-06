# Shotset harness — one command for the whole canonical plate set

**Poppy · 2026-08-07** · to Rose (build), Sun (regrade), Yamamoto (anim), Director

The next capture round is one command, not eight by hand:

```powershell
# 1. baseline — off the FROZEN 19:53 exe. Already run; re-run only if the map/content moves.
powershell -File scripts/_poppy_shotset.ps1 -BeforeSide -NoRegrade

# 2. the real pass — the moment Rose says green. Pairs + sheets + regrades on its own.
powershell -File scripts/_poppy_shotset.ps1 -Exe target-flamingo/release/voxelforge.exe
```

Step 2 writes `_poppy_shotset/before/` and `_poppy_shotset/after/` with matching
`<key>-nohud2.png` names, so `regrade.py` eats them verbatim — and runs it for you:

```
python scripts/regrade.py --before _poppy_shotset/before/ --after _poppy_shotset/after/ \
       --profile gameplay --json _poppy_shotset/regrade.json
```

Files: `scripts/_poppy_shotset.ps1` (capture) · `scripts/_poppy_shotset_sheet.py` (sheet).

## The 8 plates

| key | argv / env | client | camera |
|-----|-----------|--------|--------|
| `gate3-boot` | `VOXELFORGE_PLAY=1`, quality high | 1280x720 | gameplay boom |
| `gate3-combat` | `VOXELFORGE_COMBAT_DEMO=1`, high | 1280x720 | gameplay boom |
| `gate3-walk` | `VOXELFORGE_PLAY_DEMO=1`, high | 1280x720 | gameplay boom |
| `grade-vista` | `VOXELFORGE_LOOK_CAM=35,-18,26`, ultra | 1280x720 | canonical vista boom |
| `s1-vista` | `--play` | 1600x900 | CINE `32,20,54 → 33,5,20` |
| `hero` | `--play` | 1600x900 | CINE `34.2,2.9,28.8 → 32.5,2.0,32.4` |
| `s4-raking` | `--play`, `VOXELFORGE_LOOK_SUN=6,140,9000` | 1600x900 | CINE `54,12,44 → 26,3,20` |
| `s3-clash` | `--combat-demo`, `VOXELFORGE_ANIM_POSE=clash` | 1600x900 | CINE `34.9,3.4,31.4 → 33.4,1.7,26.4` |

Every CINE line is copied from the `CINE eye … aim …` the engine printed into that shot's
own `.log`, recorded in `_poppy_beauty/final/SHOTS.md` — not retyped from memory. The
harness echoes the engine's `CINE` line back on every run, so a plate that silently fell
back to the gameplay boom shows up instead of shipping as a "cinematic".

`-Only gate3-boot,hero` shoots a subset. `-ClashEnv "K=V"` merges extra knobs into
`s3-clash`. `-DryRun` resolves the whole plan and writes runlogs without launching
anything.

## Two things I had to fix before any of this could be trusted

### 1. Not one published baseline is pairable — the map moved

Measured today, **same exe** (`sha BF344EE7…`, 19:53:36), same camera, `s1-vista`:

| pair | mean \|Δ\| R / G / B | pixels moved |
|---|---|---|
| run 1 vs run 2 of this harness | 0.39 / 0.52 / 0.18 | 2.6 % |
| published 19:53 frame vs run 1 | **32.1 / 16.0 / 7.6** | **81.5 %** |

The harness is deterministic; the 32-unit gap is real. The engine logs say why:

```
_poppy_beauty/final/s1-village-wide.log   MAP_LOAD ok path=maps/edhari.json blocks=8513
_poppy_shotset_before/raw/s1-vista.log    MAP_LOAD ok path=maps/edhari.json blocks=8838
```

`maps/edhari.json` gained 325 blocks after those frames were taken. Pairing them against
a new binary bills a **content** change as a **look** change — 32 R-units of it, larger
than most axis moves the grade is looking for. `docs/assets/gate3/*` is from 2026-08-01,
so it is the same problem one map-edit further back.

**Sun — one for you:** `scripts/_sun_after_scorecard.sh` maps `grade-vista`'s before side
to `docs/assets/grade-vista-2026-08-05-nohud2.png`. That file is not a frame. It is a
936x1726 **contact sheet** — three stacked renders plus black caption bands and coloured
metric text. Every `grade-vista` number that pairing has produced graded the captions.

So every baseline is now re-shot through the same table with `-BeforeSide`, and the
harness records `MAP_LOAD blocks` per plate and refuses to stay quiet if the two sides
disagree (`MAP DRIFT` warning + a red banner on the sheet row). Current baseline set:
8/8 clean, `DONE=FINISH_OK`, all at `blocks=8838`.

### 2. Frame size was inherited, not declared

The gate3 baselines are 1280x720, but Rose's vista off the *same* exe came out 2560x1440
(`_rose_look_beauty/grade-vista-new.png`). "The engine default" is not a constant, and a
size change alone moves micro-contrast, edge energy and penumbra-px. Every plate now
declares W/H, the harness forces it, and then **gates on the PNG's actual size** — the
window resize is a race against the engine's fixed t=3.2s shutter, and losing it produces
a perfectly valid PNG at the wrong size with nothing in the log saying so. A plate that
loses the race re-shoots once warm, then fails loudly.

## What the harness will not do

- **It will not shoot the after side with the 19:53 binary.** That SHA is pinned; the run
  throws. It also refuses an exe older than the newest `client/src/*.rs`. `-AllowOldExe` /
  `-AllowStale` override, and the override is stamped into every runlog.
- **It never runs cargo and never writes inside a target dir.** It takes a frozen copy of
  the exe first, so it cannot hold a lock on a linker's live output. The baseline pass
  runs `_rose_look_beauty/vfprobe_grade-vista-new.exe`, already frozen outside every
  target dir.
- **It does not claim the placeholder capsule is hidden.** An earlier revision inferred
  that from `ANIM_RIG_WEAPON spawn actor=Player`; that marker predates the fix
  (`dodge_parry.rs:291`'s `if rigged { return }`) by a long way. Proof: every baseline
  plate logs `RIGS Husk,Player` **and** the red pill is plainly in the frame
  (`_poppy_shotset_before/baseline-sheet.png`). The capsule verdict comes from the
  before/after pair, by eye, on the sheet.

## Per-plate content gate

`s3-clash` fails unless the log carries **both** `ANIM_RIG_WEAPON spawn actor=Player`
and `…actor=Husk` — a saved PNG proves the swapchain was grabbed, not that both blades
were on stage. Add `Expect=@('…')` to any plate that needs the same.

Its baseline runs `VOXELFORGE_ANIM_POSE=attack`, not `clash`: the 19:53 exe has no
`clash` arm in `pose_override()` (`anim.rs:436`), so passing it there matches nothing and
yields *no pose at all* — a state the game was never in. `attack` is what that binary
could do and what the published s3 frame used, so the pair reads old-lane → new-lane
(player posed alone → `override_husk_beat()` driving the husk too), which is the change
Yamamoto's work is meant to show.

## Status

- Baseline set: **shot, 8/8, `FINISH_OK`** → `_poppy_shotset_before/after/`
- After set: **shot, 8/8, `DONE=FINISH_OK`**, off the target-flamingo 05:38 exe
  (`sha CC414845…E521B`, commit `fc329c183`). No cargo was run for it. Both sides
  logged `blocks=8838`, so map drift is 0 across the pair.
- Numbers and the two measurers' disagreement:
  `docs/note-poppy-regrade-results-2026-08-07.md`. Reading them is Flamingo's call.
- Outputs live in `_poppy_shotset/`. Only `regrade.json`, `regrade_table.txt` and
  `regrade_gates_crosscheck.txt` are tracked — the sheet PNG and the raw console log
  stay on disk (see the allowlist block in `.gitignore`).
