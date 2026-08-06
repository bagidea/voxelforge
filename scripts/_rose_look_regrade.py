#!/usr/bin/env python3
"""Rose — look re-grade harness for the post-`bb5c21e` re-capture.

`bb5c21e` lifted `ambient_lux` 1100->2200, drained ambient B 0.60->0.48, and
bumped `PCSS_WIDTH` 3->4 — the AAA scorecard's #1 (G3 crushed-shade) AND #2
(warmth) gap in one knob. That was committed to SOURCE but never re-graded,
because the only binary on disk (`_rose_rkey_proof.exe`, 06:10) predates the
commit. This harness is READY NOW so the moment Sun ships a fresh exe and the
frames are re-captured, Rose can answer "did G3 + warmth close?" in one run.

What it does
  For each given frame (a re-captured *-nohud2.png from the NEW binary):
    - runs `scripts/grade_g3.py`  -> p05-L, darkest-shade R-B, G3 verdict
    - runs `scripts/grade_axes.py`-> warmth R-B (mid), sat (mid), p95
    - diffs each number against the scorecard baseline (the 1100-lux numbers
      Sun graded on 2026-08-06, BEFORE the fix) and against the gate target
    - emits one markdown table + a verdict line per gap

It calls the canonical grade scripts (not its own maths) and only PARSES their
stdout — it never re-implements a gate, so it can't quietly soften one.

Usage
  # smoke-test on the OLD frames (reproduces the scorecard FAILs):
  python scripts/_rose_look_regrade.py docs/assets/grade-vista-2026-08-05-nohud2.png
  # real run once new frames land (e.g. in _rose_recap_20260806/):
  python scripts/_rose_look_regrade.py _rose_recap_20260806/*-new-nohud2.png > _rose_regrade.md
"""
import sys, os, re, subprocess

# Windows console defaults to cp1252, which can't encode ≥/✅/❌ — force UTF-8 so
# the table renders the same whether it goes to the terminal or a .md redirect.
for _s in (sys.stdout, sys.stderr):
    try: _s.reconfigure(encoding="utf-8")
    except Exception: pass

VF = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
G3 = os.path.join(VF, "scripts", "grade_g3.py")
AXES = os.path.join(VF, "scripts", "grade_axes.py")

# gate targets (from docs/aaa-gap-scorecard-2026-08-06.md + look-acceptance-rubric)
TARGET = {
    "g3_p05": 8.0,      # p05-L >= 8%   (G3 level clause)
    "g3_rb": 0.0,       # darkest shade R-B >= 0  (G3 hue clause, not cold)
    "warmth_mid": 110.0,  # midtone R-B >= +110
    "sat_mid": 90.0,    # midtone sat >= 90%
}

# scorecard baseline = the 1100-lux numbers Sun graded BEFORE bb5c21e (the FAILs
# this fix is supposed to close). key = stem substring.
BASELINE = {
    "grade-vista": dict(g3_p05=6.4, g3_rb=-4.0, warmth_mid=84.9, sat_mid=85.3),
    "gate3-boot":  dict(g3_p05=2.8, g3_rb=18.0, warmth_mid=82.5, sat_mid=88.6),
    "gate3-combat":dict(g3_p05=3.8, g3_rb=19.0, warmth_mid=80.4, sat_mid=90.9),
    "gate3-walk":  dict(g3_p05=2.2, g3_rb=22.0, warmth_mid=98.7, sat_mid=95.1),
}

def run(script, frame):
    p = subprocess.run([sys.executable, script, frame],
                       capture_output=True, text=True, cwd=VF)
    return p.stdout + p.stderr  # gates print to either; we grep both

def parse_g3(out):
    d = {}
    m = re.search(r"p05 L\s*=\s*([\d.]+)%", out)
    if m: d["g3_p05"] = float(m.group(1))
    m = re.search(r"darkest representative shade.*?R-B=([-\d.+]+)", out, re.S)
    if m: d["g3_rb"] = float(m.group(1))
    m = re.search(r"==\s*G3:\s*(\w+)\s*==", out)
    if m: d["g3_verdict"] = m.group(1)
    return d

def parse_axes(out):
    d = {}
    # "[FAIL] warmth R-B (mid)      84.89   target     >= 110"
    for axis, key in (("warmth R-B", "warmth_mid"), ("saturation", "sat_mid")):
        m = re.search(r"\[(\w+)\]\s*" + re.escape(axis) + r"\s*\(mid\)\s*([\d.]+)", out)
        if m:
            d[key] = float(m.group(2))
            d[key + "_verdict"] = m.group(1)
    return d

def stem_of(path):
    b = os.path.basename(path).lower()
    for k in BASELINE:
        if k in b: return k
    return None

def cell(val, target, higher_better=True):
    if val is None: return "—"
    ok = (val >= target) if higher_better else (val <= target)
    return f"{'✅' if ok else '❌'} {val:g}"

def main():
    frames = sys.argv[1:]
    if not frames:
        print(__doc__)
        print("\nno frames given — pass re-captured *-nohud2.png paths.")
        return 0
    print("# Rose look re-grade — post-`bb5c21e` (ambient_lux 2200 / B 0.48 / PCSS 4.0)")
    print(f"\n_frames graded:_ {len(frames)}  ·  _grade scripts:_ grade_g3.py + grade_axes.py (canonical, not re-implemented)\n")
    print("| frame | G3 p05-L (≥8%) | G3 shade R-B (≥0) | warmth R-B mid (≥110) | sat mid (≥90%) | G3 verdict |")
    print("|---|---|---|---|---|---|")
    closed = {"g3": 0, "warmth": 0}
    total = {"g3": 0, "warmth": 0}
    for f in frames:
        g3 = parse_g3(run(G3, f))
        ax = parse_axes(run(AXES, f))
        st = stem_of(f)
        base = BASELINE.get(st, {})
        g3v = g3.get("g3_verdict", "?")
        row = (f"| `{os.path.basename(f)}` | "
               f"{cell(g3.get('g3_p05'), TARGET['g3_p05'])} "
               f"{('_(was '+str(base.get('g3_p05'))+'_)') if 'g3_p05' in base else ''} | "
               f"{cell(g3.get('g3_rb'), TARGET['g3_rb'])} "
               f"{('_(was '+str(base.get('g3_rb'))+'_)') if 'g3_rb' in base else ''} | "
               f"{cell(ax.get('warmth_mid'), TARGET['warmth_mid'])} "
               f"{('_(was '+str(base.get('warmth_mid'))+'_)') if 'warmth_mid' in base else ''} | "
               f"{cell(ax.get('sat_mid'), TARGET['sat_mid'])} | "
               f"{g3v} |")
        print(row)
        # tally gap closure (only count frames we have a baseline for = the scorecard set)
        if st:
            if g3.get("g3_p05") is not None:
                total["g3"] += 1
                if g3["g3_p05"] >= TARGET["g3_p05"] and g3.get("g3_rb", -999) >= TARGET["g3_rb"]:
                    closed["g3"] += 1
            if ax.get("warmth_mid") is not None:
                total["warmth"] += 1
                if ax["warmth_mid"] >= TARGET["warmth_mid"]:
                    closed["warmth"] += 1
    print("\n## verdict (gap closure vs scorecard baseline)")
    if total["g3"]:
        print(f"- 🥇 **G3 (crushed shade):** {closed['g3']}/{total['g3']} frames now PASS "
              f"(p05-L ≥8% AND shade not cold). baseline was 0/4.")
    else:
        print("- 🥇 G3: no scorecard-baseline frame in this run — pass a `grade-vista`/`gate3-*` re-capture to grade gap #1.")
    if total["warmth"]:
        print(f"- 🥈 **Warmth R−B:** {closed['warmth']}/{total['warmth']} frames now ≥ +110. baseline was 0/4.")
    else:
        print("- 🥈 Warmth: no scorecard-baseline frame in this run.")
    print("\n_If G3 + warmth both close, the `bb5c21e` one-knob fix landed and gap #3 (saturation) "
          "can be judged on these same frames before touching `POST_SATURATION`._")
    return 0

if __name__ == "__main__":
    sys.exit(main())
