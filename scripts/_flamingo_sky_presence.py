#!/usr/bin/env python3
"""How much of the sky plate's "sky" is actually sky — and which binary shot it.

`_poppy_sky_after_plate.sky_mask` calls a pixel sky when it is `R>=250 & B>=170`
in the top 45% of the frame. That predicate cannot tell a dome from a sunlit
sandstone wall, and every frame in play here stands INSIDE a house in Edhari
(spawn (32.5, 2.6, 32.5), campfire at (32.5, 1.0, 29.5)); even the `--play-demo`
walk, 10 blocks north, never leaves the room. So the question gets settled
against the renderer, not against a colour range.

TWO DOME-ONLY LEVERS, AND A CONTROL FOR EACH.

  `VOXELFORGE_LOOK_SKYPROBE=1` (look.rs:1988) swaps the dome material for
  `base_color: BLACK, unlit: true`. The BLACK is what does the work, not the
  `emissive` beside it — an unlit fragment writes base_color verbatim
  (`pbr.wgsl:80-84`, the bug 5eba1e8 fixed). Every dome pixel on screen goes
  near-black; nothing else moves.

  `VOXELFORGE_LOOK_SKYHOR=1,0,1` paints the dome horizon magenta. It needs no
  pair at all — magenta pixels are counted in ONE frame — so it is the only
  reading here that a moved camera cannot fake.

  `VOXELFORGE_LOOK_SKYGAIN` is NOT usable: `haze_color()` folds `sky_gain` in
  and is also the geometry fog colour, so it repaints the world.

EVERY LEVER READING IS PRINTED BESIDE ITS NO-LEVER CONTROL — the same frame
shot twice off the same binary with nothing changed. That control is the whole
point of this script's second half: on the `--play` frames it is 0.15 levels and
a lever reading of 0.00 means something; on `--play-demo` it is ~10 levels
because `screenshot_once` fires on wall-clock `time.elapsed_secs() > 3.2`, so a
lever reading of 19.82 there means NOTHING and must not be published as a dome
measurement. Numbers whose control is bigger than they are get printed as
UNDETERMINED, not as a result.

WHERE THE EVIDENCE LIVES. Under `docs/evidence/sky-2026-08-14/`, tracked, NOT
in a scratch `_dir/` — `.gitignore:166` swallows any `_*/`, and this script used
to read its provenance frames out of `_flamingo_shots/`, print "skipped" when
they were absent, and still let the doc quote the numbers. From a fresh clone
that made section 1 unreproducible. Missing evidence is now a FAILURE, not a
skip. Override the directory with FLAMINGO_SKY_EVIDENCE=<dir> if you re-shoot.

  r1/, r2/    two full `prove_playable.sh` runs, release exe, nothing changed
  s0618/      the 06:18 frames as published (frozen copies of docs/assets/*)
  dbg/        the same shot off the STALE 08-11 debug exe — negative control
  probe/      the dome levers + a same-camera repeat of the A/B frame

Capture (release exe — the one `prove_playable.sh` must be given via BIN):

    BIN=./target/release/voxelforge.exe bash scripts/prove_playable.sh
    BIN=./target/release/voxelforge.exe bash scripts/_poppy_sky_gain_ab.sh
    # then one probe per frame, same binary, same env, plus the lever:
    VOXELFORGE_LOOK_SKYPROBE=1 VOXELFORGE_MAP_LOAD=maps/edhari.json \
      VOXELFORGE_SHOT=docs/assets/_poppy_sky_probe_edhari-load.png \
      ./target/release/voxelforge.exe --play
    VOXELFORGE_LOOK_SKYPROBE=1 \
      VOXELFORGE_SHOT=docs/assets/_poppy_sky_probe_playable-walk-after.png \
      ./target/release/voxelforge.exe --play-demo
    VOXELFORGE_LOOK_SKYHOR=1,0,1 \
      VOXELFORGE_SHOT=docs/evidence/sky-2026-08-14/probe/walk-after-skyhor.png \
      ./target/release/voxelforge.exe --play-demo

    POPPY_SKY_SET=ab    python scripts/_flamingo_sky_presence.py
    POPPY_SKY_SET=proof python scripts/_flamingo_sky_presence.py

Exit 1 if any frame's mask is mostly geometry, or if the evidence needed to
answer "which binary shot this" is not on disk.
"""
import importlib.util
import os
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = Path(os.environ.get(
    "FLAMINGO_SKY_EVIDENCE", ROOT / "docs" / "evidence" / "sky-2026-08-14"))

_spec = importlib.util.spec_from_file_location(
    "after_plate", Path(__file__).with_name("_poppy_sky_after_plate.py"))
plate = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(plate)

# Per frame: the magenta-dome shot, and a same-camera repeat with NO lever that
# gives that frame its own floor. `None` = not shot; the row then says so.
SKYHOR_OF = {
    "edhari-load.png": EVIDENCE / "probe" / "edhari-skyhor.png",
    "playable-walk-after.png": EVIDENCE / "probe" / "walk-after-skyhor.png",
    "_poppy_sky_ab_fixed.png": EVIDENCE / "probe" / "ab-skyhor.png",
}
REPEAT_OF = {
    "edhari-load.png": (EVIDENCE / "r1" / "edhari-load.png",
                        EVIDENCE / "r2" / "edhari-load.png"),
    "playable-walk-after.png": (EVIDENCE / "r1" / "playable-walk-after.png",
                                EVIDENCE / "r2" / "playable-walk-after.png"),
    "_poppy_sky_ab_fixed.png": (plate.AFTER_DIR / "_poppy_sky_ab_fixed.png",
                                EVIDENCE / "probe" / "ab-fixed-rerun.png"),
}


def load(p: Path) -> np.ndarray:
    return np.asarray(Image.open(p).convert("RGB")).astype(np.int16)


def after_path(name: str) -> Path:
    """The published AFTER frame — the frozen copy if there is one.

    `docs/assets/playable-*.png` is an OUTPUT: the next `prove_playable.sh` run
    overwrites it, and on 2026-08-14 the working-tree copy was already ahead of
    HEAD. Reading it would mean this audit's numbers change under a re-run and
    do not reproduce from a clean checkout at all. `s0618/` is the same bytes,
    committed and frozen, so prefer it and say when the live copy has drifted.
    """
    frozen = EVIDENCE / "s0618" / name
    live = plate.AFTER_DIR / name
    if not frozen.exists():
        return live
    if live.exists() and live.read_bytes() != frozen.read_bytes():
        print(f"  ! {live} has drifted from the frozen 06:18 copy — using the "
              f"frozen one, which is what the numbers in the doc were measured on")
    return frozen


def guard(b: np.ndarray, a: np.ndarray) -> float:
    """The plate's angle guard verbatim: mean |b-a| over non-sky, non-HUD."""
    off = ~plate.sky_mask(b)
    off[:plate.HUD_ROWS, :] = False
    return float(np.abs(b - a).mean(axis=2)[off].mean())


def pinned(px: np.ndarray) -> tuple:
    return (100.0 * float((px[:, 0] >= 250).mean()),
            100.0 * float((px >= 255).all(axis=1).mean()))


def magenta(a: np.ndarray) -> int:
    """Pixels the SKYHOR=1,0,1 dome paints and nothing else in this world does.

    Single-frame, so a moved camera cannot manufacture one. Validated below
    against a frame whose dome pixel count is known from SKYPROBE.
    """
    r, g, b = a[:, :, 0], a[:, :, 1], a[:, :, 2]
    return int(((r > g + 30) & (b > g + 30)).sum())


def delta(x: np.ndarray, y: np.ndarray, m: np.ndarray) -> tuple:
    """(in-mask mean, in-mask px>100, frame px>DOME_RESPONSE_MIN)."""
    d = np.abs(x - y).mean(axis=2)
    return (float(d[m].mean()), int((d[m] > 100).sum()),
            int((d > plate.DOME_RESPONSE_MIN).sum()))


def frame_section(name: str, bad: list) -> None:
    after = after_path(name)
    before = plate.before_path(name)
    probe = plate.probe_path(name)
    print()
    print("=" * 74)
    print(f"{name}")
    print("=" * 74)
    if not probe.exists():
        print(f"  no probe frame ({probe.name}) — see the capture commands above")
        bad.append(f"{name}: no probe")
        return

    a, p, b = load(after), load(probe), load(before)
    m = plate.sky_mask(b)
    dome = np.abs(a - p).mean(axis=2) > plate.DOME_RESPONSE_MIN
    inter, rest = m & dome, m & ~dome
    purity = inter.sum() / max(m.sum(), 1)

    # --- the control FIRST, so no lever number can be read without it --------
    floor = None
    r_a, r_b = REPEAT_OF.get(name, (None, None))
    if r_a and r_a.exists() and r_b and r_b.exists():
        floor = delta(load(r_a), load(r_b), m)
        print(f"  NO-LEVER CONTROL   same exe, same command, nothing changed:")
        print(f"    in-mask mean {floor[0]:6.2f} levels   px>100 {floor[1]:>6}"
              f"   frame px>{plate.DOME_RESPONSE_MIN:.0f} {floor[2]:>6}")
    else:
        print("  NO-LEVER CONTROL   MISSING — every lever number below is unreadable")
        bad.append(f"{name}: no repeat-run control ({r_b})")

    lev = delta(a, p, m)
    print(f"  SKYPROBE=1         in-mask mean {lev[0]:6.2f} levels   px>100 "
          f"{lev[1]:>6}   frame px>{plate.DOME_RESPONSE_MIN:.0f} {lev[2]:>6}")
    if floor is None:
        verdict = "UNREADABLE (no control)"
    elif lev[0] <= floor[0] + 0.5:
        verdict = "at the floor -> NO dome pixel in this frame"
    elif floor[0] > 1.0:
        verdict = (f"UNDETERMINED — the floor is {floor[0]:.2f}; this camera "
                   f"moves between runs, so the excess is not attributable")
    else:
        verdict = "above the floor -> dome present"
    print(f"    -> {verdict}")

    hor = SKYHOR_OF.get(name)
    if hor and hor.exists():
        h = load(hor)
        print(f"  SKYHOR=1,0,1       magenta px {magenta(h):>6} "
              f"(single frame — camera-proof)   dome px per SKYPROBE {int(dome.sum()):>6}")
    else:
        print("  SKYHOR=1,0,1       not shot for this frame")

    print()
    print(f"  plate mask                {int(m.sum()):>6} px")
    print(f"  dome pixels in the frame  {int(dome.sum()):>6} px "
          f"({100 * dome.mean():.2f}% of frame)"
          + ("   <- CONTAMINATED by camera drift, see control above"
             if floor and floor[0] > 1.0 else ""))
    print(f"  mask that IS dome         {int(inter.sum()):>6} px "
          f"-> purity {100 * purity:.1f}%")
    print(f"  mask that is NOT          {int(rest.sum()):>6} px "
          f"-> {100 * (1 - purity):.1f}% of the published sky is geometry")
    print(f"  angle guard before/after  {guard(b, a):.2f} levels")

    # The split that matters: geometry cannot unpin, so leaving it in the
    # mask can only ever drag the published percentage toward "unchanged".
    print()
    print("  BEFORE -> AFTER, same numbers the plate publishes, measured on:")
    for tag, sel in (("the mask as published", m),
                     ("ONLY the dome pixels", inter),
                     ("ONLY the rest", rest)):
        if not sel.sum():
            print(f"    {tag:<24} (empty)")
            continue
        pb, wb = pinned(b[sel])
        pa, wa = pinned(a[sel])
        print(f"    {tag:<24} n={int(sel.sum()):>6}  R>=250 {pb:5.1f}% -> {pa:5.1f}%"
              f"   pure-white {wb:6.3f}% -> {wa:6.3f}%"
              f"   mean R {b[sel][:, 0].mean():5.1f} -> {a[sel][:, 0].mean():5.1f}")

    if purity < plate.PURITY_MIN:
        bad.append(f"{name}: mask is {100 * (1 - purity):.1f}% geometry")


def instrument_control(bad: list) -> None:
    """The magenta detector has to be shown to FIND a dome it is aimed at.

    The A/B frame is the one frame here whose dome pixel count is known
    independently (SKYPROBE, same camera, 0.00-level floor). Point the magenta
    lever at that same camera: if the two counts do not agree, no zero it
    reports anywhere else is worth quoting.
    """
    print()
    print("=" * 74)
    print("INSTRUMENT CONTROL — does the magenta lever find a dome that IS there")
    print("=" * 74)
    fixed = after_path("_poppy_sky_ab_fixed.png")
    prb = plate.AFTER_DIR / "_poppy_sky_ab_probe.png"
    hor = EVIDENCE / "probe" / "ab-skyhor.png"
    if not (fixed.exists() and prb.exists() and hor.exists()):
        print(f"  MISSING — need {hor}")
        bad.append("magenta lever has no positive control")
        return
    dome = int((np.abs(load(fixed) - load(prb)).mean(axis=2)
                > plate.DOME_RESPONSE_MIN).sum())
    mag_on, mag_off = magenta(load(hor)), magenta(load(fixed))
    print(f"  dome px by SKYPROBE (fog off)   {dome:>6}")
    print(f"  magenta px with SKYHOR=1,0,1    {mag_on:>6}")
    print(f"  magenta px with no lever        {mag_off:>6}  (false positives)")
    ok = mag_off == 0 and abs(mag_on - dome) <= max(50, 0.02 * dome)
    print(f"  -> the two dome levers agree pixel-for-pixel: {'YES' if ok else 'NO'}")
    if not ok:
        bad.append("magenta lever failed its positive control")
    print("  CAVEAT: this control is a fog-OFF frame. Under the shipped fog the")
    print("  haze blends the dome toward `haze_color()` (which does NOT read")
    print("  SKYHOR, look.rs:1227), so a 0 on a fogged frame means 'no dome")
    print("  pixel survives the haze', not 'no dome geometry behind it'.")


def provenance(bad: list) -> None:
    """Section 1 of the doc, re-runnable: which exe shot the 06:18 frames."""
    print()
    print("=" * 74)
    print("PROVENANCE — which binary shot the published frames")
    print("=" * 74)
    r1, r2, s0618 = EVIDENCE / "r1", EVIDENCE / "r2", EVIDENCE / "s0618"
    missing = [d for d in (r1, r2, s0618) if not d.is_dir()]
    if missing:
        for d in missing:
            print(f"  MISSING {d}")
        print("  This is the evidence for 'the 06:18 frames came off the release")
        print("  exe'. Without it that claim cannot be re-run — FAILING instead of")
        print("  skipping, which is how it went unnoticed the first time.")
        bad.append(f"provenance evidence missing: {[str(d) for d in missing]}")
        return

    print(f"  {'frame':<28} {'R1 vs R2':>10} {'06:18 vs R1':>12}")
    for f in sorted(r1.glob("*.png")):
        if not (r2 / f.name).exists() or not (s0618 / f.name).exists():
            print(f"  {f.name:<28} MISSING in r2/ or s0618/")
            bad.append(f"provenance: {f.name} missing a run")
            continue
        a1 = load(f)
        print(f"  {f.name:<28} {guard(a1, load(r2 / f.name)):>10.2f} "
              f"{guard(a1, load(s0618 / f.name)):>12.2f}")
    print("  Every `--play` frame sits inside its own run-to-run floor, so 06:18")
    print("  came off the release exe. `playable-walk-after` is the `--play-demo`")
    print("  shot: `screenshot_once` fires on wall-clock `elapsed_secs() > 3.2`")
    print("  and the driven walk integrates real dt, so its floor is ~10 levels.")

    dbg = EVIDENCE / "dbg" / "edhari-load.png"
    if not dbg.exists():
        print(f"  MISSING negative control {dbg}")
        bad.append("provenance: no stale-debug-exe negative control")
        return
    d = load(dbg)
    print()
    print("  NEGATIVE CONTROL — the same shot off the STALE 08-11 debug exe:")
    print(f"    vs the 06:18 release frame  {guard(d, load(s0618 / 'edhari-load.png')):6.2f} levels")
    print(f"    vs the 05:35 'before' frame "
          f"{guard(d, load(plate.AFTER_DIR / '_poppy_sky_before_edhari-load.png')):6.2f} levels")
    print("    A stale exe lands far from 06:18 and on top of the before frame —")
    print("    which is what makes that before frame a faithful pre-fix control.")

    # ...and the control frame itself has to be provably off that exe, or the two
    # numbers above are just a picture someone said was the debug build.
    restamp = EVIDENCE / "dbg" / "edhari-load-restamp.png"
    stamp = EVIDENCE / "dbg" / "edhari-restamp.shotlog.txt"
    if restamp.exists() and stamp.exists():
        print(f"    re-shot today off {stamp.read_text(errors='ignore').splitlines()[0].strip()}")
        print(f"    and it reproduces the control frame to "
              f"{guard(load(restamp), d):.2f} levels (debug run-to-run is noisier "
              f"than release's 0.15)")
    else:
        print(f"    MISSING {restamp} — the control frame's own exe is unproven")
        bad.append("provenance: debug control has no stamped re-shoot")


def shift_search(bad: list) -> None:
    """Section 2: is the before/after difference on `edhari-load` a camera move?"""
    print()
    print("=" * 74)
    print("SHIFT SEARCH — a moved camera has a better integer shift; a repaint does not")
    print("=" * 74)
    a_p = after_path("edhari-load.png")
    b_p = plate.AFTER_DIR / "_poppy_sky_before_edhari-load.png"
    if not (a_p.exists() and b_p.exists()):
        print(f"  MISSING {a_p if not a_p.exists() else b_p}")
        bad.append("shift search: frames missing")
        return
    a, b = load(a_p), load(b_p)
    r = 6
    best, at = None, None
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            av = a[max(dy, 0):a.shape[0] + min(dy, 0),
                   max(dx, 0):a.shape[1] + min(dx, 0)]
            bv = b[max(-dy, 0):b.shape[0] + min(-dy, 0),
                   max(-dx, 0):b.shape[1] + min(-dx, 0)]
            v = float(np.abs(av - bv).mean())
            if best is None or v < best:
                best, at = v, (dx, dy)
            if (dx, dy) == (0, 0):
                zero = v
    print(f"  zero shift          {zero:.2f} levels")
    print(f"  best of +/-{r} both axes {best:.2f} levels at dx={at[0]}, dy={at[1]}")
    diff = (a.astype(np.int32) - b.astype(np.int32)).mean(axis=2)
    moved = np.abs(diff) > 1.0
    one_signed = 100.0 * float((diff[moved] > 0).mean()) if moved.any() else 0.0
    print(f"  mean signed difference {diff.mean():+.2f} levels, and "
          f"{one_signed:.0f}% of the moved pixels moved the SAME way")
    print("  -> a parallax shift moves edges both ways; a repaint moves one way")
    if at != (0, 0):
        print("  -> a shift beats zero: treat the difference as a camera move")


def main() -> int:
    print(f"POPPY_SKY_SET={os.environ.get('POPPY_SKY_SET', 'ab')}")
    print(f"EVIDENCE={EVIDENCE}")
    bad = []

    for name, _ in plate.FRAMES:
        frame_section(name, bad)

    instrument_control(bad)
    provenance(bad)
    shift_search(bad)

    print()
    if bad:
        print("VERDICT: the sky plate cannot stand on these frames")
        for x in bad:
            print(f"  - {x}")
        return 1
    print("VERDICT: every frame's mask is really the dome")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
