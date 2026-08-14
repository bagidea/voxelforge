#!/usr/bin/env python3
"""A2, re-scored on a mask that can actually carry the signal, for every sweep row.

`grade_g7.axis_a2` averages the top 18% of rows against the bottom 18%. In THIS
framing the top 18% is ~half SKY -- and sky is the one region distance fog can
never touch, because there is no geometry behind it for the fog to sit in front
of (the same argument the G7 record already makes about the sky mask hiding the
haze's COST, applied to the other side of the ledger). Every sky pixel enters the
far mean as a hard zero, so the gate's far number is diluted by whatever fraction
of the frame is sky -- a framing property, not a render property.

This re-scores the identical metric with sky excluded, and adds the two numbers
the ratio decomposes into so the failure is attributable rather than merely
observed:

  * a_meas -- the alpha the shader really applied, per band (read off the
    uniform-alpha ladder; see _flamingo_g7_alpha.py and its selftest).
  * swing  -- |off - uni100|, the display travel that band has at alpha = 1,
    i.e. how loud one unit of alpha is there. Near-field ground is deep shadow
    sitting under a haze colour many times its own radiance, so its swing is
    enormous; far-field sunlit stone is already close to the haze colour and its
    swing is small. delta ~ alpha x swing, and the ratio the gate reads is the
    product of BOTH -- which is why a 7x alpha ratio scores 2.3x on screen.

EXIT CODE (added 2026-08-14): the per-row `PASS`/`fail` column was printed and
then thrown away -- the script returned 0 whether every row passed, every row
failed, or (worse) every row's PNG was missing and the `except FileNotFoundError:
continue` swallowed it and printed an empty table under a green exit. Now:

    0 = every row in ROWS was scored and every one PASSed (near/far ratio >= 2.5
        AND far >= 6.0, sky excluded)
    1 = at least one row scored fail
    2 = the table is incomplete -- some or all sweep PNGs were missing, so the
        rows that were never opened cannot be reported as passing
"""
import sys

from PIL import Image

G7 = "_flamingo_g7"
PR = "_flamingo_g7probe"
STEP = 2
ROWS = ["ship", "h0068", "h0090", "h0120", "hd20", "hd60", "hg045", "hg085", "hwarm"]


def load(p):
    return Image.open(p).convert("RGB")


def main():
    # The two BASELINES. Every row below is a |baseline - row| difference, so a
    # missing baseline means nothing can be measured at all -- exit 2. Without
    # this guard PIL raised FileNotFoundError, Python exited 1, and a run that
    # measured NOTHING was indistinguishable from a run that scored every row and
    # failed it. A crash must never be able to pose as a verdict.
    try:
        off_img = load(f"{G7}/hazeoff-nohud2.png")
        full = load(f"{PR}/uni100-nohud2.png").load()
    except FileNotFoundError as e:
        print(f"A2 MASK RESCORE: no baseline to measure against ({e.filename}) "
              f"-- nothing scored (exit 2)", file=sys.stderr)
        return 2
    W, H = off_img.size
    off = off_img.load()

    def is_sky(p):
        return p[2] > 150 and p[2] > p[0] + 30 and p[1] > p[0]

    n_far = range(0, int(H * 0.18), STEP)
    n_near = range(H - int(H * 0.18), H, STEP)

    def band(ys, on, skip_sky):
        s = k = 0
        for y in ys:
            for x in range(0, W, STEP):
                if skip_sky and is_sky(off[x, y]):
                    continue
                p, q = off[x, y], on[x, y]
                s += max(abs(p[i] - q[i]) for i in range(3))
                k += 1
        return (s / k if k else 0.0), k

    tot_far = sum(1 for y in n_far for x in range(0, W, STEP))
    sky_far = sum(1 for y in n_far for x in range(0, W, STEP) if is_sky(off[x, y]))
    print(f"# {W}x{H}   far band is {100 * sky_far / tot_far:.1f}% sky "
          f"({sky_far}/{tot_far} sampled px) -- that fraction of the far mean is a "
          f"structural zero\n")

    sf, _ = band(n_far, full, True)
    sn, _ = band(n_near, full, True)
    print(f"## swing at alpha=1 (|off - uni100|, sky excluded): "
          f"far {sf:.1f}   near {sn:.1f}   near/far {sn / sf:.2f}x")
    print("   -> one unit of alpha is "
          f"{sn / sf:.2f}x louder in the near band than the far band\n")

    print(f"{'row':7s} {'far(gate)':>10s} {'near':>7s} {'ratio':>6s}  |  "
          f"{'far(nosky)':>11s} {'near':>7s} {'ratio':>6s}  verdict")
    scored, bad, missing = 0, 0, []
    for r in ROWS:
        try:
            on = load(f"{G7}/{r}-nohud2.png").load()
        except FileNotFoundError:
            missing.append(r)
            continue
        gf, _ = band(n_far, on, False)
        gn, _ = band(n_near, on, False)
        hf, _ = band(n_far, on, True)
        hn, _ = band(n_near, on, True)
        gr = gf / gn if gn else 0
        hr = hf / hn if hn else 0
        passed = hr >= 2.5 and hf >= 6.0
        scored += 1
        bad += not passed
        print(f"{r:7s} {gf:10.2f} {gn:7.2f} {gr:6.2f}  |  "
              f"{hf:11.2f} {hn:7.2f} {hr:6.2f}  {'PASS' if passed else 'fail'}")

    if missing:
        print(f"\n! {len(missing)} row(s) had no PNG and were not scored: "
              f"{', '.join(missing)}")
    if not scored:
        print("A2 MASK RESCORE: nothing scored -- no sweep PNG present (exit 2)")
        return 2
    if missing and not bad:
        # a partial table cannot say PASS for the rows it never opened.
        print(f"\nA2 MASK RESCORE: INCOMPLETE ({scored}/{len(ROWS)} rows scored, "
              f"none failed) -> exit 2")
        return 2
    if bad:
        print(f"\nA2 MASK RESCORE: FAIL ({bad}/{scored} row(s) below "
              f"ratio>=2.5 / far>=6.0) -> exit 1")
        return 1
    print(f"\nA2 MASK RESCORE: PASS ({scored}/{scored} row(s)) -> exit 0")
    return 0


if __name__ == "__main__":
    sys.exit(main())
