#!/usr/bin/env python3
"""WEB-vs-NATIVE look parity grader (docs/web-parity-checklist.md).

┌─────────────────────────────────────────────────────────────────────────────┐
│ RETIRED FROM NATIVE REGRADE PIPELINE — 2026-08-06, Sun                     │
│                                                                             │
│ This script REQUIRES a full web build pipeline that is NOT part of the      │
│ native-only look regrade workflow:                                          │
│                                                                             │
│   1. Rust wasm target (wasm32-unknown-unknown) + wasm-bindgen + trunk       │
│   2. Chrome/Edge with WebGPU (Dawn) — NOT WebGL2 fallback                   │
│   3. A console log with AdapterInfo `backend: BrowserWebGpu` line (W0-A)    │
│   4. Matching native build on the same locked tilt-down recipe (W0-C/D)     │
│   5. Paired captures via scripts/web-verify.mjs                             │
│                                                                             │
│ None of these prerequisites are satisfied by the native `cargo build`       │
│ workflow that the regrade pipeline (`scripts/regrade.py`) orchestrates.     │
│ The script itself is CORRECT and WELL-TESTED — it produced the CEO-         │
│ approved web parity report on 2026-07-29 — but it belongs to a SEPARATE     │
│ GATE (web store submission / cross-platform QA), not to the per-commit      │
│ look regrade that runs on native frames only.                               │
│                                                                             │
│ When web parity IS needed (Steam Deck verified, web store, browser QA):     │
│   1. Build wasm: `cargo build --release --target wasm32-unknown-unknown`   │
│   2. Bundle: `wasm-bindgen + trunk build` with the tilt-down recipe URL     │
│   3. Capture: `node scripts/web-verify.mjs` → web.png + web.console.txt     │
│   4. Native control: `bash scripts/render_wide_hero.sh` → native.png        │
│   5. Grade: `python scripts/grade_web_parity.py --web web.png \             │
│              --native native.png --console web.console.txt`                 │
│                                                                             │
│ DO NOT DELETE — the script is correct and will be needed again for          │
│ cross-platform parity gates. It just isn't part of the native regrade.      │
└─────────────────────────────────────────────────────────────────────────────┘

Parity is a DIFFERENTIAL test, not an absolute one: the web build renders the same
scene as native, so the question is "did the web backend change the look", not
"does this frame hit the golden-ref numbers". Absolute rubric targets
(grade_axes.py) are calibrated on the 1:1 golden hero framing and do NOT transfer
to an arbitrary framing/aspect -- so here they are used as DELTAS between a matched
web/native pair. The three tone-based gates (grade_gate.py G3/G5/G6) auto-locate
their sample points and ARE framing-robust, so those stay absolute on the web frame.

The metric definitions are imported from grade_axes.py / shelled out to
measure_penumbra.py on purpose -- this script must never become a second source of
truth for a number that already has one.

BASELINE (2026-07-29): the CEO-approved TILT-DOWN establishing hero,
docs/assets/wide-hero-final.png, recipe scripts/render_wide_hero.sh. The axes are
framing-sensitive, so the native control must be THAT framing -- a near-level
control would make every delta meaningless. W0-D enforces it.

  usage: grade_web_parity.py --web <web.png> --native <native.png>
                             [--console <log.txt>] [--baseline <png>] [--json out.json]

The console log is REQUIRED, not optional: it is the only place two of the three
admissibility gates can be proven -- `AdapterInfo { backend: … }` for W0-A, and the
`VOXELFORGE_URL` the page was opened with for W0-C (the wasm build takes its whole
recipe from the query string). A missing log is exactly the state in which the last
round silently graded a WebGL2 fallback. No log (and no `<web>.console.txt`
sibling) => exit 2, never a warning on top of a PASS.

exit 0 = PASS (incl. PASS-WITH-EXPECTED-WEB-DEVIATION), 1 = FAIL, 2 = not gradeable
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))
from grade_axes import measure  # noqa: E402  single source of truth for the 6 axes
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, called in main())
from hero_recipe import (  # noqa: E402  the recipe lives in render_wide_hero.sh, not here
    BASELINE_PNG, diff_query, load_recipe, query_string,
)

# ---------------------------------------------------------------------------
# W0-D -- "is the native control the approved TILT-DOWN baseline?"
#
# grade_axes' own header says the axes are measured on a fixed 1024x1024 resample
# with fixed fg/bg boxes: change the camera and every number moves. So a control
# frame shot on a different framing produces deltas that describe the CAMERA, not
# the web backend. These bounds are the render-to-render noise a re-run of
# render_wide_hero.sh may legitimately show (TAA jitter + randomised dust motes),
# NOT a look allowance -- anything past them is a different frame.
#
# MEASURED, not guessed (2026-07-29): the baseline was re-rendered from current
# source after the env->Cfg refactor (docs/assets/archive/tiltdown-baseline-
# reverify.png). It is not byte-identical -- the dust motes are randomised -- but
# every axis landed within 0.15 of the locked frame (warmth +0.15, blue -0.01,
# sat +0.01, p95 +0.00, micro -0.00). The tolerances below are ~10-25x that, so a
# clean re-shoot always passes while a different recipe does not: the previous
# key-lit frame on the SAME camera (wide-tiltB.png) trips it at warmth +36.8.
# ---------------------------------------------------------------------------
BASELINE_TOL = {"warmth": 2.0, "blue": 1.0, "sat": 1.5, "p95": 4.0, "micro": 0.4}

# ---------------------------------------------------------------------------
# TOLERANCES -- web build vs its matched native control frame.
#
# CALIBRATED 2026-07-29 on the first pair that ever cleared W0 (wasm-hero-v3.png vs
# native-control-v3.png, both on the locked tilt-down recipe): the WHOLE observed web
# drift was warmth +0.17, blue -0.01, sat +0.01, p95 +0.00, micro +0.03. Same engine,
# same recipe, deterministic scene -- so the honest bar is ~10x that, not the rubric
# headroom the provisional numbers were guessed from (warmth +/-8 would have let a
# real 5-unit regression through unnoticed).
#
# Measured on: GTX 1060 / Chrome+Dawn (BrowserWebGpu) vs Vulkan native. A first
# capture from a DIFFERENT GPU or browser that misses by less than 2x a tolerance is
# a calibration row for docs/web-parity-checklist.md §6.2, not a condemnation -- the
# verdict text says so itself.
# ---------------------------------------------------------------------------
AXIS_TOL = {
    # key:    (max |delta|, human label, why this number)
    "warmth": (2.0, "warmth R-B (mid)", "measured web drift +0.17; ~12x it, still 4x under the rubric headroom"),
    "blue":   (1.0, "blue B (mid)",     "measured -0.01; 1.0 is the smallest bound worth trusting on 8-bit output"),
    "sat":    (1.0, "saturation (mid)", "measured +0.01"),
    "p95":    (2.0, "highlight p95",    "measured +0.00; 2.0 keeps a real tonemap shift visible inside the 35-wide band"),
}
# micro-contrast is asymmetric: dropping PCSS HARDENS shadow edges, which ADDS
# hi-freq energy. A rise is expected; a fall means the web frame lost real detail.
# (Measured: +0.03. PCSS-off cost nothing here -- hero.rs is right that the penumbra
# comes from Temporal+TAA, not PCSS -- so the asymmetry is kept but tightened.)
MICRO_DROP_MAX = 0.4      # web may not be more than this BELOW native
MICRO_RISE_FLAG = 1.0     # a rise beyond this is aliasing/noise, not shadow hardening

# DOF is the Bokeh/DUAL_SOURCE_BLENDING suspect from docs/look-webgpu-risk.md.
DOF_REL_PASS = 0.85       # web/native fg:bg ratio at or above this = parity
DOF_REL_FAIL = 0.60       # below this = the bokeh pass is effectively gone

# Penumbra: the PCSS-off waiver zone. hero.rs:591-599 records that the ~6px penumbra
# comes from ShadowFilteringMethod::Temporal + TAA + the 4K shadow map, NOT from
# PCSS (whose blur_size is clamped to the 0.5 floor in a room this small). So
# removing PCSS is PREDICTED to cost <=~1px. A bigger collapse is a different bug.
PEN_PASS_PX = 3.0         # rubric G4's own bar: shadow transition >= 3px
PEN_WAIVE_PX = 2.0        # 2.0-3.0px: accepted FOR WEB ONLY as the PCSS-off deviation
PEN_UNEXPLAINED_DROP = 1.5  # native-web beyond this is NOT explainable by PCSS removal

DEAD_FRAME_L = 2.0        # luminance below this counts as "black"
DEAD_FRAME_FRAC = 0.97    # >= this fraction black = dead frame (the v1 black-canvas bug)
DEAD_FRAME_COLORS = 64    # fewer distinct colors than this = nothing was rendered


def load_stats(path):
    im = Image.open(path).convert("RGB")
    a = np.asarray(im).astype(np.float32)
    L = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
    return {
        "size": im.size,
        "aspect": im.size[0] / im.size[1],
        "black_frac": float((L < DEAD_FRAME_L).mean()),
        "colors": len(Image.open(path).convert("RGB").getcolors(maxcolors=1 << 20) or []),
    }


def run(cmd):
    p = subprocess.run([sys.executable, *cmd], capture_output=True, text=True)
    return p.returncode, p.stdout + p.stderr


def penumbra(path):
    _, out = run([str(SCRIPTS / "measure_penumbra.py"), str(path)])
    m = re.search(r"mean penumbra = ([\d.]+)px", out)
    return (float(m.group(1)) if m else None), out


def gates(path):
    """Absolute G3/G5/G6 on the web frame -- framing-robust, so no native pair needed."""
    rc, out = run([str(SCRIPTS / "grade_gate.py"), str(path)])
    res = {}
    for g in ("G3", "G5", "G6"):
        m = re.search(rf"## {g}\b.*?-> (PASS|FAIL)", out, re.S)
        res[g] = m.group(1) if m else "?"
    # A non-zero exit means grade_gate.py REFUSED to grade (its nohud2 guard, a
    # missing file, a decode error) -- it printed no gate lines at all. The regex
    # above then leaves every gate "?", and the caller's `if v != "PASS"` turns
    # each one into a reported FAIL. That is the exact failure mode this whole
    # guard exists to prevent, one layer up: three invented FAILs with no reason
    # attached. So a refusal is a hard stop that carries grade_gate's own words.
    if rc != 0 and any(v == "?" for v in res.values()):
        print("\n[STOP] grade_gate.py refused this frame -- no absolute gate was measured:")
        for line in (out.strip() or "(no output)").splitlines():
            print(f"       {line}")
        print("\n=> NOT GRADEABLE. Nothing below would mean anything; fix the frame and re-run.")
        sys.exit(2)
    return res, out


def backend_gate(console_path):
    """W0-A: which backend did wgpu ACTUALLY open?

    The only authoritative read is bevy_render's own AdapterInfo line
    (`backend: BrowserWebGpu` vs `backend: Gl`). The in-page `#gpu` HUD and
    navigator.gpu.requestAdapter() only prove the BROWSER exposes WebGPU -- a wasm
    that falls back to WebGL2 boots happily while that HUD still reads
    "WebGPU adapter OK" (scripts/web-verify.mjs:195-199 documents the same trap).
    A WebGL2 frame is a different feature set, so parity numbers taken off it are
    meaningless: this is a hard stop, not a FAIL.

    Returns (backend|None, reason_it_is_unknown|None).
    """
    if not console_path:
        return None, ("no console log supplied -- pass --console <log>, or drop it next to the "
                      "web png as <web>.console.txt. Without it W0-A is UNVERIFIED and the "
                      "frame may be a silent WebGL2 fallback.")
    try:
        txt = Path(console_path).read_text(encoding="utf-8", errors="replace")
    except OSError as e:
        return None, f"could not read {console_path}: {e}"
    line = next((l for l in txt.splitlines() if "AdapterInfo {" in l), "")
    m = re.search(r"backend:\s*(\w+)", line)
    if not m:
        return None, (f"no AdapterInfo `backend:` line in {console_path} -- bevy_render never "
                      "logged it, so W0-A cannot be proven. Do NOT fall back to the HUD line.")
    return m.group(1), None


URL_IN_LOG = re.compile(r"https?://[^\s\"'<>]+\?[^\s\"'<>]+")


def recipe_gate(console_path):
    """W0-C: did the LOCKED tilt-down recipe actually reach the renderer?

    On wasm every look knob comes from the query string (client/src/main.rs, wasm
    `read_cfg`), so the URL the page was opened with IS the recipe -- and it is
    evidence only if it comes out of the capture itself. scripts/web-verify.mjs
    seeds the console dump with `[harness] VOXELFORGE_URL <url>` for exactly this.
    A frame shot on `?hero` alone renders the BAKED DEFAULT (narrow, near-level),
    which is a different framing from the baseline: the deltas would then measure
    the camera, not the browser.

    Returns (url|None, [problems]).
    """
    txt = Path(console_path).read_text(encoding="utf-8", errors="replace")
    urls = [u for u in URL_IN_LOG.findall(txt) if "hero" in u]
    if not urls:
        return None, ["no capture URL in the console log -- scripts/web-verify.mjs writes "
                      "`[harness] VOXELFORGE_URL <url>` as its first line; a log without it "
                      "cannot prove which recipe was rendered"]
    return urls[0], diff_query(load_recipe(), urls[0])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--web", required=True)
    ap.add_argument("--native", required=True)
    ap.add_argument("--baseline", default=str(BASELINE_PNG),
                    help="the approved TILT-DOWN frame the native control must match "
                         "(default docs/assets/wide-hero-final.png). Pass a different file "
                         "ONLY when the CEO has re-locked the framing.")
    ap.add_argument("--console", help="the web capture's console log (REQUIRED -- it carries "
                                      "the AdapterInfo line that is the only proof of W0-A); "
                                      "auto-detected from <web>.console.txt if that file exists")
    ap.add_argument("--json")
    a = ap.parse_args()

    # HARD GUARD — the pair being compared, not --baseline. Every delta in
    # section C is |web - native| over whole-frame percentiles, so a HUD on ONE
    # side is a difference the other side does not have: the parity report would
    # blame the web backend for the game's own UI. --baseline is deliberately
    # NOT guarded — it is the CEO-approved golden docs/assets/wide-hero-final.png,
    # curated artwork rather than a capture (see nohud2_guard.EXEMPT for the same
    # reasoning applied to grade_ref.py).
    require_nohud2([a.web, a.native], tool="grade_web_parity.py")

    if not a.console:
        d = Path(a.web + ".console.txt")
        if d.exists():
            a.console = str(d)

    fails, waived, notes = [], [], []
    report = {"web": a.web, "native": a.native}

    # ---- A. admissibility -------------------------------------------------
    print("=" * 72)
    print("A. ADMISSIBILITY  (is this pair gradeable at all?)")
    ws, ns = load_stats(a.web), load_stats(a.native)
    report["admissibility"] = {"web": ws, "native": ns}
    print(f"   web    {ws['size'][0]}x{ws['size'][1]}  black {ws['black_frac']*100:5.1f}%  colors {ws['colors']}")
    print(f"   native {ns['size'][0]}x{ns['size'][1]}  black {ns['black_frac']*100:5.1f}%  colors {ns['colors']}")
    hard_stop = []

    # W0-A: which backend did wgpu ACTUALLY open? The HUD's "WebGPU adapter OK" is
    # navigator.gpu.requestAdapter() -- browser capability, not the opened backend
    # (web-verify.mjs:195-199 documents the same trap). bevy_render's AdapterInfo is
    # the only authoritative line. A WebGL2 fallback silently drops SSAO / DOF /
    # Atmosphere / OIT, so every look axis measured on it is meaningless.
    #
    # UNVERIFIED == NOT GRADEABLE, never a warning stapled to a PASS. docs §3 says
    # "ตก W0-A/B/C ข้อใดข้อหนึ่ง = NOT GRADEABLE", and W0-A cannot be *passed* by
    # the absence of evidence -- a log-less run is indistinguishable from the Gl
    # fallback that killed the previous round. There is deliberately no override
    # flag: the only way to clear this gate is to produce the log.
    backend, why_unknown = backend_gate(a.console)
    report["backend"] = backend
    if backend is None:
        print("   [STOP] backend UNVERIFIED (W0-A has no evidence)")
        hard_stop.append(
            f"W0-A could not be proven: {why_unknown} No log = no verdict; "
            "the frame is treated as a possible WebGL2 fallback, not as a pass.")
    elif backend != "BrowserWebGpu":
        print(f"   [STOP] wgpu opened backend={backend}, not BrowserWebGpu")
        hard_stop.append(
            f"backend is {backend} (WebGL2 fallback), not BrowserWebGpu -- the capture "
            "silently lost SSAO/DOF/Atmosphere; look axes measured here grade the "
            "fallback, not the web target. Ignore any HUD 'WebGPU adapter OK'.")
    else:
        print("   [OK] wgpu opened BrowserWebGpu")

    # W0-B companion: the engine's own feature-drop lines prove which passes never
    # loaded. Only PCSS is a declared deviation; anything else here means the frame
    # is a different renderer, not a slightly different look.
    if a.console:
        log = Path(a.console).read_text(encoding="utf-8", errors="replace")
        dropped = [n for n, pat in (
            ("SSAO", "ScreenSpaceAmbientOcclusionPlugin not loaded"),
            ("DOF", "Disabling depth of field"),
            ("Atmosphere", "AtmospherePlugin not loaded"),
            ("OIT", "OrderIndependentTransparencyPlugin not loaded"),
        ) if pat in log]
        if dropped:
            report["dropped_passes"] = dropped
            print(f"   [STOP] render passes never loaded: {', '.join(dropped)}")
            hard_stop.append(
                f"render passes missing from this build: {', '.join(dropped)} -- only PCSS "
                "is a declared deviation; these are not, so the frame cannot be graded for look")

    # W0-C: the recipe. Only meaningful with a log -- when there is none, W0-A above
    # already stopped the run, so don't pile on a second copy of the same complaint.
    if a.console:
        url, problems = recipe_gate(a.console)
        report["capture_url"] = url
        if problems:
            print("   [STOP] W0-C: the captured URL does not carry the locked tilt-down recipe")
            for p in problems:
                print(f"          - {p}")
            hard_stop.append(
                "W0-C recipe mismatch (" + "; ".join(problems) + "). Expected `?"
                + query_string(load_recipe()) + "`. A frame rendered on a different "
                "recipe/framing cannot be differenced against the tilt-down baseline.")
        else:
            print(f"   [OK] W0-C recipe matches the locked tilt-down set ({url.split('?')[0]}?...)")

    # W0-D: is the native control actually the approved TILT-DOWN baseline? Without
    # this the whole differential is unanchored -- someone hands in a near-level
    # control and every axis delta then describes the camera move, not the browser.
    mn = measure(a.native)
    same_file = Path(a.native).resolve() == Path(a.baseline).resolve()
    if same_file:
        print("   [OK] W0-D native control IS the baseline file itself")
        report["baseline"] = {"file": a.baseline, "same_file": True}
    else:
        mb = measure(a.baseline)
        drift = {k: mn[k] - mb[k] for k in BASELINE_TOL}
        off = {k: d for k, d in drift.items() if abs(d) > BASELINE_TOL[k]}
        report["baseline"] = {"file": a.baseline, "same_file": False,
                              "drift": drift, "off": list(off)}
        shown = "  ".join(f"{k} {d:+.2f}" for k, d in drift.items())
        if off:
            print(f"   [STOP] W0-D native control != tilt-down baseline: {shown}")
            hard_stop.append(
                "native control drifts from the approved tilt-down baseline on "
                + ", ".join(f"{k} {off[k]:+.2f} (tol +/-{BASELINE_TOL[k]:g})" for k in off)
                + f" -- re-shoot it with `bash scripts/render_wide_hero.sh` (baseline "
                f"{a.baseline}). Deltas against a different framing measure the camera, "
                "not the web backend.")
        else:
            print(f"   [OK] W0-D native control matches the tilt-down baseline ({shown})")

    for name, s in (("web", ws), ("native", ns)):
        if s["black_frac"] >= DEAD_FRAME_FRAC or s["colors"] < DEAD_FRAME_COLORS:
            hard_stop.append(f"{name} frame is DEAD (black canvas / nothing rendered)")
    if abs(ws["aspect"] - ns["aspect"]) > 0.01:
        hard_stop.append(f"aspect mismatch web {ws['aspect']:.3f} vs native {ns['aspect']:.3f} "
                         "-- reshoot at a matched resolution, the axes are framing-sensitive")
    if hard_stop:
        for h in hard_stop:
            print(f"   [STOP] {h}")
        print("\n=> NOT GRADEABLE. Fix the capture, then re-run.")
        report["verdict"] = "NOT_GRADEABLE"
        report["hard_stop"] = hard_stop
        if a.json:
            Path(a.json).write_text(json.dumps(report, indent=2), encoding="utf-8")
            print(f"(json -> {a.json})")
        sys.exit(2)
    print("   [OK] both frames alive, aspect matched")

    # ---- B. absolute gates on the web frame -------------------------------
    print("\nB. ABSOLUTE GATES on the WEB frame (grade_gate.py -- framing-robust)")
    gt, graw = gates(a.web)
    report["gates_web"] = gt
    for g, v in gt.items():
        print(f"   [{v:>4}] {g}")
        if v != "PASS":
            fails.append(f"{g} FAIL on web (tone gate -- PCSS has nothing to do with this)")
    print("   note: G1/G2/G4 are visual checks -- grade_gate.py does not score them.")

    # ---- C. axis parity ---------------------------------------------------
    print("\nC. AXIS PARITY  (grade_axes metrics, |web - native|)")
    mw = measure(a.web)  # mn was measured for W0-D above -- one read, one number
    report["axes"] = {"web": {k: v for k, v in mw.items()}, "native": {k: v for k, v in mn.items()}}
    print(f"   {'axis':<20}{'web':>9}{'native':>9}{'delta':>9}   tol")
    for key, (tol, label, _why) in AXIS_TOL.items():
        d = mw[key] - mn[key]
        ok = abs(d) <= tol
        print(f"   [{'PASS' if ok else 'FAIL'}] {label:<14}{mw[key]:9.2f}{mn[key]:9.2f}{d:+9.2f}   +/-{tol:g}")
        if not ok:
            near = abs(d) <= 2 * tol
            fails.append(f"{label} drifted {d:+.2f} (tol +/-{tol:g})"
                         + (" -- inside 2x tol, CALIBRATE before condemning" if near else ""))

    dmic = mw["micro"] - mn["micro"]
    if dmic < -MICRO_DROP_MAX:
        print(f"   [FAIL] micro-contrast {mw['micro']:9.2f}{mn['micro']:9.2f}{dmic:+9.2f}   >= -{MICRO_DROP_MAX:g}")
        fails.append(f"micro-contrast fell {dmic:+.2f} -- web lost real surface detail")
    elif dmic > MICRO_RISE_FLAG:
        print(f"   [WARN] micro-contrast {mw['micro']:9.2f}{mn['micro']:9.2f}{dmic:+9.2f}   rise > {MICRO_RISE_FLAG:g}")
        notes.append(f"micro-contrast rose {dmic:+.2f}: more than shadow-hardening explains -- "
                     "check for aliasing / missing TAA, not a win")
    else:
        print(f"   [PASS] micro-contrast {mw['micro']:9.2f}{mn['micro']:9.2f}{dmic:+9.2f}   "
              f"(rise expected: harder shadow edges add hi-freq)")

    rel = mw["dof"] / max(mn["dof"], 1e-6)
    print(f"\n   DOF fg:bg  web {mw['dof']:.2f}  native {mn['dof']:.2f}  web/native {rel:.2f}")
    report["dof_rel"] = rel
    if rel < DOF_REL_FAIL:
        fails.append(f"DOF fg:bg is {rel:.0%} of native -- the Bokeh pass is effectively gone "
                     "(see look-webgpu-risk.md: DUAL_SOURCE_BLENDING). NOT covered by the PCSS waiver")
        print(f"   [FAIL] below {DOF_REL_FAIL:.0%}")
    elif rel < DOF_REL_PASS:
        notes.append(f"DOF fg:bg degraded to {rel:.0%} of native -- bokeh weakened but present")
        print(f"   [WARN] below {DOF_REL_PASS:.0%}")
    else:
        print(f"   [PASS] at/above {DOF_REL_PASS:.0%}")

    # ---- D. shadow softness: the PCSS-off waiver zone ----------------------
    print("\nD. SHADOW SOFTNESS  (the deliberate PCSS-off deviation)")
    pw, _ = penumbra(a.web)
    pn, _ = penumbra(a.native)
    report["penumbra"] = {"web": pw, "native": pn}
    if pw is None or pn is None:
        print("   [SKIP] no clear shadow edge found in the floor band on "
              f"{'web' if pw is None else 'native'} -- re-run measure_penumbra.py with "
              "an explicit row band, or grade G4 by eye at 400%")
        notes.append("penumbra not measurable on this framing -- G4 needs a human 400% check")
    else:
        print(f"   web {pw:.2f}px   native {pn:.2f}px   drop {pn-pw:+.2f}px")
        if pw >= PEN_PASS_PX:
            print(f"   [PASS] web still clears the rubric G4 bar (>= {PEN_PASS_PX:g}px)")
        elif pw >= PEN_WAIVE_PX:
            print(f"   [WEB-WAIVED] {PEN_WAIVE_PX:g}-{PEN_PASS_PX:g}px: accepted for web ONLY as the PCSS-off cost")
            waived.append(f"penumbra {pw:.2f}px (below the {PEN_PASS_PX:g}px G4 bar, above the "
                          f"{PEN_WAIVE_PX:g}px floor) -- expected deviation, web only")
        else:
            print(f"   [FAIL] below {PEN_WAIVE_PX:g}px -- reads as hard 1px PCF, the look goes flat")
            fails.append(f"penumbra {pw:.2f}px < {PEN_WAIVE_PX:g}px: shadows are hard, not soft")
        if (pn - pw) > PEN_UNEXPLAINED_DROP:
            print(f"   [FLAG] drop of {pn-pw:.2f}px exceeds the ~1px PCSS removal predicts")
            notes.append(f"penumbra dropped {pn-pw:.2f}px -- hero.rs:591-599 says the soft edge comes from "
                         "ShadowFilteringMethod::Temporal + TAA, not PCSS. A drop this large points at "
                         "TAA/Temporal NOT surviving the web build. Investigate; do not waive as PCSS.")

    # ---- E. verdict -------------------------------------------------------
    print("\n" + "=" * 72)
    if fails:
        print("VERDICT: FAIL")
        for f in fails:
            print(f"  - {f}")
    elif waived:
        print("VERDICT: PASS WITH EXPECTED WEB DEVIATION")
        for w in waived:
            print(f"  ~ {w}")
    else:
        print("VERDICT: PASS (full parity)")
    for n in notes:
        print(f"  ! {n}")
    report["verdict"] = "FAIL" if fails else ("PASS_WITH_WEB_DEVIATION" if waived else "PASS")
    report["fails"], report["waived"], report["notes"] = fails, waived, notes
    if a.json:
        Path(a.json).write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(f"\n(json -> {a.json})")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
