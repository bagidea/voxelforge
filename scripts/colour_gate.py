#!/usr/bin/env python3
"""Colour gate for gameplay frames — the checks the Gate 3 shoot could not see.

Implements the three framing-independent checks proposed in
`docs/gate3-colour-review-2026-08-01.md` §3, so "renders successfully" and
"renders correctly" stop being the same result:

  Gate A  magenta fraction   — lit pixels whose G sits below BOTH R and B.
                               Scene- and framing-independent. Measured 0.00%
                               on every approved frame and 16.4-73.7% on the
                               broken ones, so the 2% threshold has 8.2x
                               headroom below the mildest real failure and the
                               approved frames do not merely pass, they read
                               exactly zero.  [FATAL]
  Gate B  sky ordering       — the flat `ClearColor` sky must still leave the
                               post stack ordered B > G > R. This is the check
                               that caught the bug: a flat clear is not a lit
                               surface, so nothing in the scene can excuse it
                               changing rank. SKIPPED (not failed) when the
                               frame has too little sky to measure.  [FATAL]
  Gate B  sky gain order     — the per-channel LINEAR gain from the ClearColor
                               parsed out of client/src/main.rs to the measured
                               sky must NOT show the magenta chromatic-adaptation
                               signature: red lifted over unity while green is
                               crushed onto blue (G_gain <= B_gain). That pair is
                               the whole 2026-08-01 review turned into a hard
                               check — the matrix that builds magenta always
                               leaves it on the flat clear, where a sunlit patch
                               may or may not. SKIPPED with no sky.  [FATAL]
  Gate C  sunlit ordering    — the brightest lit (non-sky) surface must not read
                               the magenta signature: red dominant (R leads G by
                               R_DOMINANT_MARGIN) with green crushed onto blue
                               (G ~= B). It no longer demands "warm golden" of
                               every frame, so it does not false-FAIL clean green
                               outdoor shots (grass has G as the max channel)
                               while still catching the cast a magenta wash leaves
                               on lit geometry. The R-dominant margin also prevents
                               mid-gray surfaces (R~G~B all within 2 points) from
                               tripping on accidental channel ordering.  [FATAL]

Gate C was advisory through commit 10c335f for two stated reasons: it
false-PASSed magenta (R>B reads "warm") and it false-FAILed clean green frames
(grass G>R>B). Both are fixed by switching it from "must be R>G>B" to "must not
be red-dominant with G~=B": the magenta signature fails, amber (G clearly > B)
passes, and green (G is max, R not dominant) passes. The magenta wash is now
caught three ways — A (fraction), B (sky order), B (sky gain) — with C as the
lit-surface backstop, so no single gate's blind spot on a given frame lets the
cast through.

Sky is found as the largest flat colour cluster in the top third of the frame
(the sky is a constant clear, so it is by far the biggest uniform area up
there) rather than by "is it blue", which would presuppose the answer.

Usage:
    python scripts/colour_gate.py frame.png [more.png ...]
    python scripts/colour_gate.py --tsv frame.png ...   # machine-readable table
Exit code 0 only if every frame passes every applicable FATAL gate.
"""
import os
import re
import sys

import numpy as np
from PIL import Image

MAGENTA_MAX_FRACTION = 0.02  # Gate A
LIT_MIN_MEAN = 25  # ignore near-black when judging hue
CHANNEL_MARGIN = 8  # how far G must sit below R and B to count as magenta
SKY_MIN_FRACTION = 0.02  # below this much sky, Gate B skips instead of failing
# A large flat region at the top of an INDOOR frame is a ceiling or a lit wall,
# not a sky, and grading its channel order against a sky rule is meaningless —
# both hero interiors trip it otherwise. Two discriminators, neither of which
# presupposes the sky is blue (that is the thing under test):
#   * bright — daylight sky sits near the top of the frame's range;
#   * it owns the top EDGE — sky runs off the top of frame, a wall does not.
# Measured on the approved set: top-2-row coverage is 33-66% on the three outdoor
# frames and 5-24% on the four interiors, so 30% separates them. Caveat stated
# rather than hidden: the luminance floor also skips a genuine night sky, and the
# combined rule is deliberately conservative — a frame that wrongly SKIPs Gate B
# is still caught by Gate A, which is the primary check.
SKY_MIN_LUM = 100.0
SKY_MIN_TOP_EDGE = 0.30

# --- Gate B (gain): the magenta chromatic-adaptation signature (HARD FAIL) ---
# A white-balance matrix pushed warm (the bug) lifts RED above unity and crushes
# GREEN and BLUE *together* — the off-diagonal LMS terms bleed G and B into R as
# one pair, so G_gain ~= B_gain (both < 1) while R_gain soars. Measured on the
# known-magenta fixture (git db25758, before the TEMPERATURE 0.10->0.02 fix):
#   sky gain  R x1.98  G x0.54  B x0.55   ->  R lifted, G crushed onto B  [FAIL]
# and on every approved frame:
#   sky gain  R x0.81  G x0.66  B x0.65   ->  no red lift                  [PASS]
# A correct grade never lifts the unlit sky's red above unity (AcesFitted + the
# sectional curve only compress a bright clear), so RED_LIFT=1.0 is the
# physically-meaningful "a warm matrix intervened" line, with ~0.2 headroom below
# the approved frames and ~1.0 below the failure. A genuinely warm tint leaves G
# clearly above B (amber #F4B860 = 1.00/0.75/0.39), so "red lifted AND G not
# above B" is the bug, never the look. This is the deterministic catcher Gate C
# cannot be: the flat ClearColor sky always carries the matrix, where a sunlit
# patch only shows it when the brightest lit surface happens to land magenta.
SKY_GAIN_RED_LIFT = 1.0  # R_gain above authored unity => a warm matrix intervened

# --- Gate C: the sunlit-patch magenta signature (HARD FAIL; was advisory) ---
# Magenta's sunlit patch reads RED dominant with GREEN crushed *onto* BLUE — the
# db25758 walk shot logged sunlit 211.3/154.4/154.4 ("G and B equal to a tenth").
# A real warm surface keeps G clearly above B (amber 244/184/96, G-B = +88); a
# green outdoor surface has G as the MAX channel (grass 121/199/112), so it never
# trips a red-dominant rule. FAIL only when R is the max channel AND G has not
# stayed a clear margin above B — that single change fixes both of the old
# advisory's faults at once: it no longer false-PASSes magenta (G~=B under red)
# and no longer false-FAILs clean green frames (G>R, R not dominant). Measured
# G-B on the approved set is +57..+70, so SUN_GB_MIN=20 sits in a wide gap.
SUN_GB_MIN = 20.0
# R must lead G by at least this many points (in 0-255 sRGB) to count as red-dominant.
# Below this margin a mid-gray surface (R~G~B ±2) whose R happens to be 1-2 points ahead
# is not meaningfully "red" — it is neutral gray that tripped an accidental ordering.
# Measured on the approved set: every real warm surface has R-G ≥ 56. The edhari
# top-down false positive had R-G = 0.6, so 5.0 sits in a wide gap.
R_DOMINANT_MARGIN = 5.0

_MAIN_RS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "client", "src", "main.rs")
_CLEARCOLOR_RE = re.compile(r"ClearColor\s*\(\s*Color::srgb\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)")
AUTHORED_FALLBACK = (0.53, 0.72, 0.92)

# --- Allowlist ------------------------------------------------------------------
# Format: one entry per line  <glob>  <reason>  [gates]
# gates is an optional comma-separated list of gate letters (A, B, C) to skip.
# If omitted, ALL gates are skipped for matching files.
# Glob matching: fnmatch against the relative path. First match wins.

_ALLOW_GATE_MAP = {"A": "a", "B": "b", "C": "c"}


def parse_allowlist(path):
    """Parse a .colour-gate-allow file into [(glob, reason, gates_set)].

    Format: <glob>  <reason text…>  [A|B|C]
    The optional trailing gate letter(s) (comma-separated, last token) select
    which gates to skip.  If the last token is not a recognised gate letter,
    it is treated as part of the reason and ALL gates are exempt.
    """
    entries = []
    with open(path, "r", encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            tokens = line.split()
            if len(tokens) < 2:
                continue
            pattern = tokens[0]
            # If the last token looks like gate letters, peel it off
            last = tokens[-1]
            gate_candidates = {g.strip().upper() for g in last.split(",")}
            if gate_candidates and gate_candidates.issubset(_ALLOW_GATE_MAP.keys()):
                gates = gate_candidates
                reason = " ".join(tokens[1:-1])
            else:
                gates = set()
                reason = " ".join(tokens[1:])
            entries.append((pattern, reason, gates))
    return entries


def _glob_to_re(pattern):
    """Compile a glob pattern to a compiled regex.  Supports ** for zero-or-more
    path segments (so ``docs/assets/**/*REJECTED*.png`` matches both top-level and
    nested files under docs/assets/)."""
    parts = []
    i = 0
    while i < len(pattern):
        c = pattern[i]
        if c == "*" and i + 1 < len(pattern) and pattern[i + 1] == "*":
            # ** — zero or more path segments (greedy)
            i += 2
            if i < len(pattern) and pattern[i] == "/":
                i += 1
            parts.append(r"(?:.+/)?")
        elif c == "*":
            parts.append(r"[^/]*")
            i += 1
        elif c == "?":
            parts.append(r"[^/]")
            i += 1
        else:
            parts.append(re.escape(c))
            i += 1
    return re.compile("^" + "".join(parts) + "$")


def match_allowlist(filepath, entries):
    """Return the set of gate letters (A, B, C) to skip for this file.
    An empty set means no exemption; None means exempt from ALL gates.
    Glob patterns support ** for recursive matching (zero or more dirs)."""
    if entries is None:
        return set()
    for pattern, reason, gates in entries:
        rx = _GLOB_CACHE.get(pattern)
        if rx is None:
            rx = _glob_to_re(pattern)
            _GLOB_CACHE[pattern] = rx
        if rx.search(filepath):
            return None if not gates else gates
    return set()


# Compiled regex cache for glob patterns.
_GLOB_CACHE = {}


def authored_clearcolor():
    """The sky as authored in source — parsed, never hand-copied."""
    try:
        with open(_MAIN_RS, "r", encoding="utf-8", errors="replace") as f:
            m = _CLEARCOLOR_RE.search(f.read())
        if m:
            return tuple(float(x) for x in m.groups())
    except OSError:
        pass
    return AUTHORED_FALLBACK


def srgb_to_linear(c):
    c = np.asarray(c, np.float64)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def sky_gain(sky_rgb):
    """Per-channel LINEAR gain applied to the authored ClearColor to land on `sky_rgb`."""
    return srgb_to_linear(np.asarray(sky_rgb, np.float64) / 255.0) / srgb_to_linear(
        np.asarray(authored_clearcolor(), np.float64))


def sky_gain_is_magenta(g):
    """True if a sky linear gain shows the magenta chromatic-adaptation signature:
    RED lifted above authored unity while GREEN is crushed onto BLUE (G_gain <=
    B_gain). See SKY_GAIN_RED_LIFT. The deterministic magenta catcher."""
    g = np.asarray(g, np.float64)
    return bool(g[0] > SKY_GAIN_RED_LIFT and g[1] <= g[2])


def sunlit_is_magenta(sun):
    """True if a sunlit patch shows the magenta signature: RED is the dominant
    channel AND GREEN has not stayed a clear margin (SUN_GB_MIN) above BLUE — the
    "G and B equal to a tenth" cast. Passes warm amber (G clearly > B) and green
    outdoor frames (G is the max channel, so R is not dominant). None -> False.

    r_dominant now requires R to lead G by R_DOMINANT_MARGIN (default 5 points),
    so a mid-gray surface whose R happens to be 1-2 points ahead does not
    accidentally read as red-dominant."""
    if sun is None:
        return False
    sun = np.asarray(sun, np.float64)
    r_dominant = sun[0] > sun[1] + R_DOMINANT_MARGIN and sun[0] > sun[2]
    return bool(r_dominant and (sun[1] - sun[2]) < SUN_GB_MIN)


def sky_patch(a):
    """Mean RGB of the flat sky region, or None if there isn't enough of one.

    Quantises the top third of the frame to a coarse 3D colour histogram and
    takes the largest bin. The sky is a flat clear, so it dominates that band;
    lit geometry is textured and scatters across many bins.
    """
    top = a[: a.shape[0] // 3].reshape(-1, 3)
    q = (top // 16).astype(np.int32)
    key = q[:, 0] * 4096 + q[:, 1] * 64 + q[:, 2]
    vals, counts = np.unique(key, return_counts=True)
    best = vals[counts.argmax()]
    frac = counts.max() / a.shape[0] / a.shape[1]
    if frac < SKY_MIN_FRACTION:
        return None, f"only {frac * 100:.1f}% flat top region"
    rgb = top[key == best].mean(0)
    if rgb @ np.array([0.2126, 0.7152, 0.0722]) < SKY_MIN_LUM:
        return None, "flat top region too dark to be sky"
    edge = (key[: a.shape[1] * 2] == best).mean()
    if edge < SKY_MIN_TOP_EDGE:
        return None, f"flat top region owns only {edge * 100:.0f}% of the top edge"
    return rgb, f"{frac * 100:.1f}% of frame"


def sunlit_patch(a, sky_rgb):
    """Mean RGB of the brightest lit surface that is NOT the sky."""
    lum = a @ np.array([0.2126, 0.7152, 0.0722])
    keep = np.ones(a.shape[:2], bool)
    if sky_rgb is not None:
        # Drop anything within a coarse bin of the sky colour.
        keep &= np.abs(a - sky_rgb).max(2) > 24
    if keep.sum() < 500:
        return None
    thr = np.percentile(lum[keep], 90)
    sel = keep & (lum >= thr)
    return a[sel].mean(0)


def order(rgb):
    return " > ".join(c for _, c in sorted(zip(rgb, "RGB"), reverse=True))


def measure(path):
    """Every number this gate knows about one frame, as a dict.

    Pre-processing, stated exactly because docs quote these figures: the PNG at
    NATIVE resolution, convert("RGB"), no resample, no crop, no HUD mask.
    `mag_frac` and `warm_frac` are over ALL pixels; the `_lit` variants are over
    lit pixels only. The all-pixel denominator is the conservative one for
    gating (a mostly-dark frame cannot trip Gate A on a small lit region); the
    share-of-lit one is the exposure-independent one for comparing frames whose
    lit fraction differs (an indoor hero shot vs a full-daylight gameplay one).
    """
    a = np.asarray(Image.open(path).convert("RGB")).astype(np.float32)
    n = float(a.shape[0] * a.shape[1])
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    lit = a.mean(2) > LIT_MIN_MEAN
    lit_n = max(float(lit.sum()), 1.0)
    magenta = lit & (G < R - CHANNEL_MARGIN) & (G < B - CHANNEL_MARGIN)
    warm = lit & (R > G) & (G > B)

    sky, sky_note = sky_patch(a)
    return {
        "path": path,
        "lit_frac": lit.sum() / n,
        "mag_frac": magenta.sum() / n,
        "mag_frac_lit": magenta.sum() / lit_n,
        "warm_frac": warm.sum() / n,
        "warm_frac_lit": warm.sum() / lit_n,
        "sky": sky,
        "sky_note": sky_note,
        "sky_gain": None if sky is None else sky_gain(sky),
        "sun": sunlit_patch(a, sky),
    }


def check(path, allowlist=None):
    m = measure(path)
    mag_frac = m["mag_frac"]
    sky, sky_note, sun = m["sky"], m["sky_note"], m["sun"]

    exempt = match_allowlist(path, allowlist) if allowlist else set()
    # None means "exempt from ALL gates"; set() means "exempt from nothing"

    rows = []
    ok_a = bool(mag_frac <= MAGENTA_MAX_FRACTION)
    rows.append(("A magenta fraction", f"{mag_frac * 100:6.2f}%", f"<= {MAGENTA_MAX_FRACTION * 100:g}%", ok_a))

    # Gate B has two halves, both FATAL: the sRGB ordering of the flat sky, and
    # the linear-gain signature of the matrix that built it. Both skip cleanly
    # (do not fail) when there is no sky to measure.
    if sky is None:
        rows.append(("B sky order", sky_note, "SKIP", None))
        rows.append(("B sky gain order", sky_note, "SKIP", None))
        ok_b = True
        ok_gain = True
    else:
        ok_b = bool(sky[2] > sky[1] > sky[0])
        rows.append((
            "B sky order",
            f"{sky[0]:5.1f},{sky[1]:5.1f},{sky[2]:5.1f} -> {order(sky)}",
            "B > G > R",
            ok_b,
        ))
        g = m["sky_gain"]
        ok_gain = not sky_gain_is_magenta(g)  # bool already (function returns Python bool)
        rows.append((
            "B sky gain order",
            (f"x{g[0]:.2f}/x{g[1]:.2f}/x{g[2]:.2f} -> {order(g)}"
             + ("" if ok_gain else f"  (R>x{SKY_GAIN_RED_LIFT:g} with G<=B)")),
            "no red lift over G<=B",
            ok_gain,
        ))

    # Gate C is now FATAL. It does not require "warm golden" of every frame; it
    # FAILs only the magenta signature on the brightest lit surface — red
    # dominant with green crushed onto blue — which neither clean amber nor green
    # outdoor frames ever show. See SUN_GB_MIN and sunlit_is_magenta().
    if sun is None:
        rows.append(("C sunlit order", "no lit surface", "SKIP", None))
        ok_c = True
    else:
        ok_c = not sunlit_is_magenta(sun)  # bool already (function returns Python bool)
        rows.append((
            "C sunlit order",
            f"{sun[0]:5.1f},{sun[1]:5.1f},{sun[2]:5.1f} -> {order(sun)}",
            "no R-dom with G~=B",
            ok_c,
        ))

    # Apply allowlist: an exempted gate that would FAIL prints EXEMPT and counts
    # as passing for the exit code.
    exempted = set()
    for i, (label, value, target, ok) in enumerate(rows):
        if ok is False:
            gate_letter = label[0]  # A, B, or C
            if exempt is None or gate_letter in exempt:
                exempted.add(gate_letter)
                # Replace the verdict so it prints EXEMPT and counts as passing
                rows[i] = (label, value, target, None)
    # For Gate B, both sub-checks (order + gain) share the B letter — if either
    # was going to fail and B is exempt, exempt BOTH halves.
    if "B" in exempted:
        for i, (label, value, target, ok) in enumerate(rows):
            if label.startswith("B ") and ok is False:
                rows[i] = (label, value, target, None)

    print(f"\n{path}")
    for label, value, target, ok in rows:
        if ok is None:
            # Distinguish SKIP (data not available) from EXEMPT (gate skipped by allowlist)
            if label[0] in exempted:
                tag = "EXEMPT"
            else:
                tag = "SKIP"
        else:
            tag = "PASS" if ok else "FAIL"
        print(f"  [{tag}] {label:<20} {value:<34} target {target}")
    if sky is not None:
        g = m["sky_gain"]
        ar, ag, ab = authored_clearcolor()
        print(f"         authored ClearColor({ar:g},{ag:g},{ab:g}) = "
              f"{ar * 255:.0f},{ag * 255:.0f},{ab * 255:.0f}"
              f"  ->  linear gain  R x{g[0]:.2f}  G x{g[1]:.2f}  B x{g[2]:.2f}")
    print(f"         warm-ordered R>G>B {m['warm_frac'] * 100:6.2f}% of frame "
          f"({m['warm_frac_lit'] * 100:.2f}% of lit)   [context, not gated]")
    frame_ok = all(ok is not False for (_, _, _, ok) in rows)
    print(f"  => {'COLOUR GATE PASS' if frame_ok else 'COLOUR GATE FAIL'}")
    return frame_ok


def main():
    # ── Parse flags ──
    allowlist_path = None
    show_tsv = False
    args = []
    i = 1
    while i < len(sys.argv):
        a = sys.argv[i]
        if a == "--allow":
            i += 1
            if i >= len(sys.argv):
                print("colour_gate.py: --allow requires a path", file=sys.stderr)
                sys.exit(2)
            allowlist_path = sys.argv[i]
        elif a == "--tsv":
            show_tsv = True
        elif not a.startswith("--"):
            args.append(a)
        i += 1

    if not args:
        print("usage: colour_gate.py [--allow <file>] [--tsv] <frame.png> [more.png ...]")
        sys.exit(2)

    allowlist = parse_allowlist(allowlist_path) if allowlist_path else None

    if show_tsv:
        print("frame\tmagenta%\tmagenta%(lit)\twarm%\twarm%(lit)\tlit%\tgateA")
        ok = True
        for p in args:
            m = measure(p)
            a_ok = m["mag_frac"] <= MAGENTA_MAX_FRACTION
            # Apply allowlist for Gate A in tsv mode
            if not a_ok and allowlist is not None:
                exempt = match_allowlist(p, allowlist)
                if exempt is None or "A" in exempt:
                    a_ok = True
            ok = ok and a_ok
            print(f"{p}\t{m['mag_frac'] * 100:.2f}\t{m['mag_frac_lit'] * 100:.2f}"
                  f"\t{m['warm_frac'] * 100:.2f}\t{m['warm_frac_lit'] * 100:.2f}"
                  f"\t{m['lit_frac'] * 100:.2f}\t{'PASS' if a_ok else 'FAIL'}")
        sys.exit(0 if ok else 1)
    ok = all([check(p, allowlist) for p in args])
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
