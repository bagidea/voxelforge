#!/usr/bin/env python3
"""p0env_baseline.py - baseline a plate set on the axes that judge OUTDOOR work.

Why this exists
---------------
docs/look-acceptance-rubric.md rule 7 ("no control = no verdict") also implies
"no baseline = no delta": before an axis may call tonight's frame better or
worse, somebody has to have written down what the same plate scored before.
Everything above the P0-ENV section of the rubric grades INTERIOR frames - both
calibration refs have sky 0 px - so a table of clip/micro/p95 says nothing about
grass, sky or distance. This driver fills that hole for a whole plate set at
once, on both graders that carry FAIL power outdoors.

What it runs, per plate
-----------------------
  scripts/_pixel_artgap_grade.py   P0-ENV  (sky / distance / palette / depth)
  scripts/grade_gate.py            layer A (G3 / G5 / G6)

Read-only by construction: no --oneshot, no --scoreboard, no --history, no
--mask-dir, so docs/aaa-scoreboard-live.md and docs/assets/artgap/{artgap.json,
history.jsonl} are never rewritten by a baseline sweep. Run the controls
yourself first - this driver does NOT imply them:

    python scripts/_pixel_artgap_controls.py          # must exit 0
    python scripts/grade_gate.py docs/assets/golden-beauty-shot-ref.png

Usage
-----
    python scripts/p0env_baseline.py <out-dir> <tag>=<plate-dir> [<tag>=<dir> ...]
                                     [--plates a,b,c]

    python scripts/p0env_baseline.py _fl_p0env_20260820 \
        N6=_pixel_shotset_N6/after poppy=_poppy_shotset/after

Writes <out-dir>/<tag>-<plate>.json (raw P0-ENV metrics, one per plate) and
<out-dir>/baseline.json (the joined table incl. G3/G5/G6). Exit 0 if every plate
was graded, 1 if any plate was missing or a grader refused.

Note for scripts/tests/test_nohud2_guard.py: this file measures no pixels. Every
number it reports comes back from a GUARDED grader through subprocess, so the
suffix rule is still enforced - by those tools, on the same paths.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_PLATES = ["gate3-boot", "gate3-combat", "gate3-walk", "grade-vista",
                  "hero", "s1-vista", "s3-clash", "s4-raking"]


def grade(frame: Path, out_json: Path):
    """P0-ENV metrics for one frame, or None if the grader refused."""
    rc = subprocess.run(
        [sys.executable, "scripts/_pixel_artgap_grade.py",
         str(frame.relative_to(ROOT)), "--json", str(out_json)],
        cwd=ROOT, capture_output=True, text=True).returncode
    if not out_json.is_file():
        return None, rc
    doc = json.loads(out_json.read_text(encoding="utf-8"))
    return (doc["frames"][0] if "frames" in doc else doc), rc


def gates(frame: Path):
    """G3/G5/G6 verdicts for one frame, straight out of the sole judge."""
    p = subprocess.run(
        [sys.executable, "scripts/grade_gate.py", str(frame.relative_to(ROOT))],
        cwd=ROOT, capture_output=True, text=True)
    out = {}
    for g in ("G3", "G5", "G6"):
        m = re.search(rf"^## {g}\b.*?^\s*-> (\w+)", p.stdout, re.S | re.M)
        # a refusal is not a verdict - say so instead of scoring it
        out[g] = m.group(1) if m else ("REFUSED" if p.returncode == 2 else "NO-VERDICT")
    return out, p.returncode


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir")
    ap.add_argument("sets", nargs="+", metavar="TAG=DIR")
    ap.add_argument("--plates", default=",".join(DEFAULT_PLATES))
    a = ap.parse_args()

    out = Path(a.out_dir)
    out.mkdir(parents=True, exist_ok=True)
    plates = [p.strip() for p in a.plates.split(",") if p.strip()]

    rows, bad = [], 0
    for spec in a.sets:
        tag, _, d = spec.partition("=")
        for p in plates:
            frame = ROOT / d / f"{p}-nohud2.png"
            if not frame.is_file():
                print(f"MISSING  {frame}", file=sys.stderr)
                bad += 1
                continue
            art, art_rc = grade(frame, out / f"{tag}-{p}.json")
            gate, gate_rc = gates(frame)
            if art is None or "REFUSED" in gate.values():
                bad += 1
            rows.append({"tag": tag, "plate": p, "frame": str(frame.relative_to(ROOT)),
                         "art": art, "gate": gate,
                         "art_rc": art_rc, "gate_rc": gate_rc})
            print(f"{tag}/{p:<14} artgap rc={art_rc}  gate rc={gate_rc}  "
                  f"{gate['G3'][0]}{gate['G5'][0]}{gate['G6'][0]}", flush=True)

    (out / "baseline.json").write_text(json.dumps(rows, indent=1), encoding="utf-8")
    print(f"\nwrote {out/'baseline.json'}  ({len(rows)} plate(s), {bad} incomplete)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
