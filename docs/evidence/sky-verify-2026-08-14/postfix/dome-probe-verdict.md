# 4-angle dome probe — verdict (2026-08-14)

Numbers below are **measured, not asserted**. Every value was produced live this
session by `scripts/_rose_a1_dome_pixels.py` and `scripts/grade_sky.py` run against
the four captures committed in `39f1dd9`
(`docs/assets/gate3/_rose_probe_{default,camup,camup2,realdome}.png`), captured by
`scripts/_rose_cap_skyprobe.sh` on the fresh 2026-08-14 release exe. The probe
self-test (`--control`) passed all four control classes before any real frame was
graded, so the gate is trusted.

The four angles: `default` (default boom), `camup` (`VOXELFORGE_LOOK_CAM=0,0.7,14`,
pitched up), `camup2` (`0,1.0,18`, pitched up further), `realdome` (`0,0.7,14` with
the real dome, **no** green probe). `default`/`camup`/`camup2` ran with
`VOXELFORGE_LOOK_SKYPROBE=1`, which swaps the dome for a flat nuclear-green
HDR-8.0 emissive — independent of any colour tuning, so a rendered dome would show
green dominance regardless of `exp_comp`/`haze_color`.

## Measured numbers

| angle | dome probe: Gdom / %Gmax / tex90 / rough → verdict | grade_sky: %R>=250 / R-span / sky-rows → verdict |
|---|---|---|
| `default` | 0.52 / 0.0% / 36.5 / 3.19 → **UNMEASURABLE** | 1.27% / 0.55 / 19 → **FAIL** (cream) |
| `camup` (pitch 0.7) | 0.52 / 0.0% / 43.7 / 3.75 → **UNMEASURABLE** | 0.22% / 0.00 / 0 → **UNMEASURABLE** |
| `camup2` (pitch 1.0) | 0.52 / 0.0% / 43.5 / 3.75 → **UNMEASURABLE** | 0.16% / 0.00 / 0 → **UNMEASURABLE** |
| `realdome` (real dome) | 0.50 / 0.0% / 39.9 / 3.66 → **UNMEASURABLE** | 0.22% / 0.00 / 0 → **UNMEASURABLE** |

Texture thresholds: `tex90 >= 18` or `rough >= 2.2` ⇒ top band is terrain, not sky
(open-sky plates read tex90 ≤ 13.6 / rough ≤ 1.6; the 2026-08-14 no-sky canyon plate
read tex90 36.4 / rough 3.2). Every one of the four clears both thresholds (tex90
36.5–43.7, rough 3.19–3.75): **the top of every frame is terrain / a canyon wall.**

## ก. Did any of the four angles see the sky dome? — No.

None of the four captured an open-sky region. Three (`camup`, `camup2`, `realdome`)
have **zero** qualifying sky rows under grade_sky's blown-sky predicate
(%R>=250 0.16–0.22%, R-span 0.00). `default` alone has a thin 19-row cream sliver
(%R>=250 1.27%, R-span 0.55) — but the dome probe shows that sliver carries
**0.0% green** even with the HDR-8.0 emissive probe active, so it is the flat
ClearColor/background, not the dome material. Pitching the boom **up** reduced the
sky, not increased it: 19 rows → 0 rows → 0 rows. The dome is not seen in any angle.

## ข. Cause: dome not spawned/rendered, or camera never aimed at sky? — Camera (precondition), proven; dome, undecided by these frames.

The **texture guard** is the deciding evidence. It proves the top band is terrain in
all four frames — there is no sky region to grade in any of them. That is a
precondition failure, not a dome verdict (the probe docstring defines UNMEASURABLE
as "a capture problem, never a dome verdict"). So what these four frames **prove**
is narrow: the default play-path spawn is terrain-enclosed, and even at near-max
upward pitch (camup2 pitch 1.0 of the +1.20 range) the boom frames more canyon wall,
not sky. "Camera never aimed at open sky" is **proven**. Whether the dome is also
not rendering is **undecided** — there was no sky to test it against. The green
probe (HDR-8.0 emissive) could not return a verdict because it had no sky band to
read. The 2026-08-11 DEBUG-A1 finding (dome dead at fragment on a frame that *did*
show sky) remains the only evidence that addresses dome-rendering directly.

## ค. The two candidates in postfix/README.md — which is right?

Neither is proven or refuted by these four captures, because none framed open sky:

1. **Second multiplier in `look.rs` (`build_sky_dome_mesh`/`haze_color`) still
   overdriving** — undecided. Requires an open-sky frame to compare dome radiance
   against `exp_comp`; we have none.
2. **`--play-demo` path doesn't run through `5eba1e8`'s code** — undecided. The
   green-probe path was specifically meant to sidestep both candidates (a flat
   emissive bypasses `exp_comp`/`haze_color` entirely, so green ⇒ play-path renders
   the dome ⇒ candidate 2 false, suspicion shifts to colour). That test never ran:
   the terrain-enclosed spawn defeated it before it could produce a verdict.

Both candidates **fall outside** what these four frames can adjudicate. They are
still hypotheses.

## Still unprovable

- **Candidate 1 vs candidate 2** (the README's two) — needs a capture that frames
  open sky with the green probe active. None of the four did.
- **Whether `7299237` actually makes the dome render.** Ancestry check at HEAD
  `39f1dd9`: `7299237` ("A1 sky dome never rendered — spawn was missing Visibility",
  the 2026-08-11 root cause) is an ancestor of `5eba1e8` ("sky was 1513x
  overexposed"); **both are in HEAD**. So the Visibility fix shipped before the
  overexposure fix, and the binary the postfix README graded already carried both.
  This probe is the falsification gate for `7299237` — and it could not run
  (UNMEASURABLE ×4), so the Visibility fix is **still unconfirmed by capture**.

## What the next capture must do (not done here)

Change the **spawn point** to an open-sky location — pitching the boom up from the
current terrain-enclosed spawn only buys more canyon wall. Keep
`VOXELFORGE_LOOK_SKYPROBE=1` so the dome-render question is decoupled from the two
colour candidates. Only a frame whose top band passes the texture guard (tex90 < 18,
rough < 2.2) can adjudicate any of the three open questions.
