# A6 item 2 (teal cool accent) — reshot on the real binary, still OPEN, root cause found

**Yamamoto (Eng) · 2026-08-10 · re-grades `docs/note-to-yamamoto-teal-accent-2026-08-10.md`
+ `docs/VERDICT-a6-composition-2026-08-10.md` item 2, after `anim.rs` commit `90ace55`
("feat(anim): A6 cool accent — cloak-clasp brooch, teal #4FC9D6") landed.**

## Binary used

`target/release/voxelforge.exe`, relinked **2026-08-10 01:30:39** by a full
`cargo build --release --workspace` (Poppy's playproof build, `_poppy_playproof_build.log`,
exit 0, "Finished `release` profile [optimized] target(s) in 47m 01s"). Commit `90ace55`
is **2026-08-10 01:15:01** — exe postdates the commit, confirmed by timestamp, not assumed.

**Correction on the binary I was first pointed at:** `target/release/voxelforge_shot.exe`
is *not* the gate3 game binary — per `client/Cargo.toml`, it's `[[bin]] name =
"voxelforge_shot" path = "src/shot_main.rs"`, an isolated hero-shot renderer (Flamingo's,
for the Golden Beauty-Shot) that never runs `main.rs`, never processes `VOXELFORGE_PLAY`,
and never hits `MAP_LOAD`. Shooting it produced a solid red/checkerboard frame with no
humanoid silhouette and no `MAP_LOAD`/`SCENE_READY` lines in the log at all — confirmed
by diffing the log against a real gate3 boot log. Re-ran the shoot on the correct
`voxelforge.exe`: `MAP_LOAD ok path=maps/edhari.json`, `SCENE_READY`, `SHOT saved`, 3/3
frames pass `scripts/gate3_shoot.sh`'s mechanical gate.

## Method (same as the 3.3° baseline, so the numbers are comparable)

- `scripts/gate3_shoot.sh` with `GATE3_SKIP_WAIT=1` (working tree has same-day WIP on
  `look.rs`/`settings_menu.rs` touching mtimes, same situation the baseline doc noted).
- De-HUD: `scripts/_flamingo_dehud2.py`.
- Camera: boom is still A5-shortened (open bug, not this pass) — solved via
  `scripts/_flamingo_char_dist_solve.py --cam 0,-14.32,6.5`: **implied dist 5.289 m**,
  which is the *exact* figure in Monanisa's `boot-char.json` — same spawn, same pose,
  confirms this is an apples-to-apples reshoot, not a different frame.
- Grade: `scripts/grade_character.py --cam 0,-14.32,5.289 --ref-json _fl_char/ref-auren-hero.json`.
  Also reshot once with `VOXELFORGE_LOOK_GRADE=0.05,1.02,1.12,0.86` (Monanisa's exact
  override) to match her pipeline byte-for-byte — numbers move <0.3% either way, so the
  committed `look.rs` on disk already carries that grade (matches `git status` showing
  `look.rs` still WIP-modified, same as her note described).
- Hue delta: reused `grade_character.py`'s own `analytic_bbox` + `mask_from_bbox` (not a
  hand-rolled threshold) for the char mask, ring-minus-mask for background, same
  `colorsys.rgb_to_hsv` hue calc the baseline doc's repro snippet specifies.

## Result: hue delta did NOT improve

```
hero  RGB (114.4, 70.8, 26.7)   hue 30.2°
bg    RGB (150.0, 88.9, 22.3)   hue 31.3°
hue delta: 1.1°        (pre-clasp baseline, 2026-08-10: 3.3°)
```

1.1° vs 3.3° is not a regression signal — it's noise (different bg-ring pixels, ~1°
of camera/lighting jitter) around "no change." The hero's mean RGB (114.4, 70.8, 26.7)
is within rounding of Monanisa's pre-clasp number (114.5, 70.3, 25.6). **The clasp is
not moving the average at all.**

Checked why directly rather than trusting the average: scanned every pixel in the frame
(not just the character bbox) for anything in the clasp's own hue band (`#4FC9D6` = HSV
hue 185.8°, verified by computing `colorsys.rgb_to_hsv` on the literal `Color::srgb`
constant from `anim.rs:653`). Window `150°–220°`, `delta>15`, `max>30` (loose enough to
catch a desaturated/darkened render of the same hue):

```
gate3-after-boot-satmatch-nohud2.png    (2560x1360, 3,481,600 px): 0 matching px
gate3-after-combat-nohud2.png           (2560x1360, 3,481,600 px): 0 matching px
```

Zero. Not "small," not "subtle" — the gem does not render anywhere in either frame.
Cropped the shoulder region 3x for a direct look (`bbox` top half, boot frame): the
entire shoulder/back is one solid dark cloak mass, no clasp visible at all.

## Root cause (hypothesis, not yet fixed)

The clasp's `PartSpec`s sit at `cloak_anchor`'s own `base_pos` (`sh_x*0.7, sh_y*0.95,
0.08`, torso-local) — deliberately, per the patch's own comment, so it lines up with
where the cloak fastens. But the housing/gem are placed at that anchor's *z* almost
exactly (`0.08` housing, `0.095` gem — only 1.5cm further out) while the cloak mesh
itself is not a flat patch at that point; it's the whole drape covering the back, which
by construction extends further out from the torso than 1.5cm at the shoulder line. The
gem is very likely sitting *behind* the cloak's own rendered surface from the third-
person camera's point of view — same camera angle the fix was designed for (`main.rs`
boom behind the avatar, back-to-camera). This is consistent with 0 visible pixels across
two different poses/angles (boot idle, combat approach) rather than a framing fluke.

This is a step further than the risk the design note (`note-to-yamamoto-teal-accent-
2026-08-10.md` point 3) called out — it anticipated the clasp reading "too small/subtle
at range," fixed by widening `size`. Full occlusion needs a *z*-offset push (further out
from the torso, past the cloak's actual surface depth at that point), not a size change;
widening a hidden part's footprint doesn't make it visible.

## Status

| | status | evidence |
|---|---|---|
| A6.2 hue-separated from background | **STILL OPEN** — clasp compiles and is in the rig (`anim.rs:638-657`, confirmed in the binary), but renders 0 px in 2 real frames; hue delta 1.1° vs 3.3° pre-fix, i.e. no measured change | this doc |

## Not done this pass, and why

Did not attempt a live z-offset fix + rebuild: the release build that produced today's
exe took 47 minutes and the office was explicitly protecting this build lane (a second
`cargo check --workspace` from Rose was already running in `target/debug` when this
reshoot started). Landing a guessed z-offset without being able to iterate visually
risks another 47-minute round trip for a wrong guess. Flagging back to Monanisa / the
build-lock owner rather than grabbing the lane unilaterally.

**Suggested next step:** either (a) measure the cloak's actual mesh depth at the anchor
point in `anim.rs`'s cloak `PartSpec`/spring-lag code and push the clasp `z` a known
amount past it, or (b) temporarily disable the cloak `PartSpec` in a debug build to
confirm the clasp itself renders correctly in isolation (t-junction check) before
re-tuning its depth against the cloak.

— Yamamoto
