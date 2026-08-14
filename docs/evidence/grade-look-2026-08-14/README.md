# grade_look.py exit-code evidence — 2026-08-14

`42b0c20` gave `scripts/grade_look.py` one verdict instead of twelve printed
PASS/FAILs and an unconditional exit 0. Its commit message names three frames as
the proof: two that must exit 1, one that must exit 0. The exit-0 one lived at
`_poppy_shotset/after/gate3-combat-nohud2.png`, which `.gitignore:237`
(`/_poppy_shotset/*`, an allowlist that admits three small text files and no
PNG) swallows — so a fresh clone could reproduce both FAILs and not the PASS.
That is the same scan `dac7e8d` closed for the sky audit: the proof was in an
ignored dir. The frame is now here, tracked, byte-identical.

## Run it

From the repo root, on a clean checkout — nothing to build, the graders read
pixels off disk:

    python scripts/grade_look.py docs/assets/grade-vista-2026-08-05-nohud2.png            # exit 1
    python scripts/grade_look.py docs/assets/wide-hero-final-nohud2.png                   # exit 1
    python scripts/grade_look.py docs/evidence/grade-look-2026-08-14/gate3-combat-nohud2.png  # exit 0

| frame | measured gates red | exit | tracked as |
|---|---|---|---|
| `docs/assets/grade-vista-2026-08-05-nohud2.png` | G3, G4a (3px), G5 | **1** | already tracked; worktree md5 `2436573f…` == HEAD blob |
| `docs/assets/wide-hero-final-nohud2.png` | G4a (4px) | **1** | already tracked; worktree md5 `ad81abd7…` == HEAD blob |
| `gate3-combat-nohud2.png` (this dir) | none — all five green, penumbra 8px | **0** | added by this commit, md5 `c08a37f78509a74c30a4a199e6e9d966` |

Only `G2/G3/G4a/G5/G6` vote. `G1` and `G4b` print and are explicitly not scored,
so they cannot move the exit code in either direction.

Both red frames were already in the repo and match HEAD byte-for-byte, so
nothing was dragged in for them; only the green one was missing.

## Where the green frame came from

`_poppy_shotset/manifest.json` (also ignored, so the provenance is copied here):

    key    gate3-combat — "husk 2.2 blocks ahead, player closing at t=3.2s"
    env    VOXELFORGE_COMBAT_DEMO=1 VOXELFORGE_LOOK_QUALITY=high
    exe    target-flamingo/release/voxelforge.exe
           sha256 CC414845…65E521B, 2026-08-07T05:38:59+07:00
    commit fc329c183
    size   1280x640

It exits 0 on two consecutive runs from this tracked path (2026-08-14, Python
3.13.2) — the same re-run check the sky evidence uses, because a PASS that only
happens once is not a control.

## What this dir does NOT claim

It is not the only tracked frame that reaches the exit-0 branch. Running
`grade_look.py` over all 19 `*-nohud2.png` files tracked at HEAD, two others
already exit 0:

    0  docs/assets/gate3-a6-2026-08-10/gate3-after-walk-nohud2.png        G4a 5px
    0  docs/assets/gate3-a6-teal-zfix-2026-08-11/gate3-after-boot-nohud2.png  G4a 6px
    1  the other 17, including docs/assets/gate3/gate3-after-combat-nohud2.png

So the branch was reachable from a clean clone before this commit — the gap was
narrower than "the positive control is unreproducible", and saying otherwise
would overstate it. What was unreproducible is the specific claim `42b0c20`
makes: *this* frame, at 8px penumbra. The two frames above sit at 5px and 6px,
and 5px is the threshold itself (`need >= 5`), which is a thin thing to rest a
gate's positive direction on. That is why the 8px frame is the one carried in.

Note the near-namesake trap: `docs/assets/gate3/gate3-after-combat-nohud2.png`
is a *different* shot (2026-08-05, md5 `d1b6e5f1…`) and it exits **1** on G3+G6.
It is not a substitute for this file.

The `-nohud2.png` suffix is load-bearing: `scripts/nohud2_guard.py` refuses any
other name with exit 2 before a single number is printed, so this file keeps the
name it was captured under.
