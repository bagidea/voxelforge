#!/usr/bin/env python3
"""Two-panel before/after sheet for the G7 look pass, captioned with REAL grader output.

The captions are not typed in — they are read back out of `grade_gate.py`,
`grade_axes.py` and `grade_g7.py` for the exact PNG in the panel above them. A
hand-written caption on a render sheet is the easiest place in this whole
pipeline to launder a number, so the sheet refuses to carry one.

Usage: _flamingo_g7_sheet.py <before.png> <after.png> <out.png> [before-label] [after-label]

EXIT CODE (added 2026-08-14): the captions print `G3 F` / `C FAIL`, and when a
grader crashed they printed `G3 ?` / `C FAIL` from an empty stdout -- and the
script exited 0 in all three cases, so a sheet whose own caption says the frame
failed handed back a green exit.

The verdict carried is the AFTER panel's, because that is the frame being
proposed; the BEFORE panel is context and is *expected* to fail (that is the
point of a before/after sheet), so it never turns the exit red on its own.

    0 = every grader answered for both panels and the AFTER panel is all-P/PASS
    1 = graders answered and the AFTER panel carries a real FAIL
    2 = a grader printed no verdict for either panel (a `?` in a caption) --
        the sheet is still written, but `?` is not a pass
"""
import re
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).parent


def grade(png):
    def run(script, *a):
        return subprocess.run([sys.executable, str(HERE / script), *a],
                              capture_output=True, text=True).stdout

    gate = run("grade_gate.py", png)
    axes = run("grade_axes.py", "--profile", "gameplay", png)
    g7 = run("grade_g7.py", "--frame", png)

    def g(out, pat, d="?"):
        m = re.search(pat, out)
        return m.group(1) if m else d

    v = re.search(r"MEASURABLE GATES: G3=(\w)\s+G5=(\w)\s+G6=(\w)", gate)
    g3, g5, g6 = v.groups() if v else ("?", "?", "?")
    # grade_g7 prints one `=> PASS|FAIL` summary. Without it the grader never had
    # an opinion, which must read as `?`, not as a vegetation FAIL.
    veg_sum = re.search(r"=>\s+(PASS|FAIL)", g7)
    return {
        "G3": g3, "G5": g5, "G6": g6,
        "p05": g(gate, r"interior p05-L=([\d.]+)%"),
        "L": g(gate, r"golden patch .*? L=([\d.]+)"),
        "p95": g(axes, r"highlight p95\s+([\d.]+)"),
        "micro": g(axes, r"micro-contrast\s+([\d.]+)"),
        "sat": g(g7, r"saturation\s+([\d.]+)%\s+need"),
        "hue": g(g7, r"hue\s+([\d.]+)deg need"),
        "C": ("PASS" if re.search(r"->\s+PASS\s+\[", g7) else "FAIL") if veg_sum else "?",
    }


def caption(tag, m):
    return (f"{tag}   G3 {m['G3']} p05 {m['p05']}%  |  G5 {m['G5']}  |  G6 {m['G6']} L {m['L']}  |  "
            f"p95 {m['p95']}  micro {m['micro']}  |  veg sat {m['sat']}% hue {m['hue']}deg  C {m['C']}")


def main():
    before, after, out = sys.argv[1], sys.argv[2], sys.argv[3]
    tags = (sys.argv[4] if len(sys.argv) > 4 else "BEFORE (G6)",
            sys.argv[5] if len(sys.argv) > 5 else "AFTER  (G7)")
    try:
        font = ImageFont.truetype("C:/Windows/Fonts/consolab.ttf", 15)
    except OSError:
        font = ImageFont.load_default()

    # A panel PNG that is not there is unmeasurable (exit 2), not a FAIL. Without
    # this, PIL raised FileNotFoundError and Python exited 1 -- so "you gave me the
    # wrong path" was handed back in the same number as "the AFTER frame failed the
    # gates", which is the exact confusion the exit codes above exist to remove.
    for tag, path in zip(tags, (before, after)):
        if not Path(path).is_file():
            print(f"SHEET VERDICT: UNREADABLE -- no PNG for the "
                  f"{tag.strip()} panel ({path}) -> exit 2", file=sys.stderr)
            return 2

    panels, marks = [], []
    for tag, path in zip(tags, (before, after)):
        im = Image.open(path).convert("RGB")
        w, h = im.size
        bar = 24
        p = Image.new("RGB", (w, h + bar), (16, 16, 16))
        p.paste(im, (0, bar))
        m = grade(path)
        marks.append((tag, m))
        text = caption(tag, m)
        ImageDraw.Draw(p).text((7, 4), text, fill=(240, 226, 190), font=font)
        panels.append(p)
        print(text)

    w, h = panels[0].size
    sheet = Image.new("RGB", (w, h * 2 + 8), (16, 16, 16))
    for i, p in enumerate(panels):
        sheet.paste(p, (0, i * (h + 8)))
    sheet.save(out)
    print(f"OK {out} {sheet.size}")

    # --- carry the verdict (see module docstring for why AFTER only) ---------
    unknown = [f"{tag}:{k}" for tag, m in marks
               for k in ("G3", "G5", "G6", "C") if m[k] == "?"]
    if unknown:
        print(f"SHEET VERDICT: UNREADABLE -- no grader verdict for "
              f"{', '.join(unknown)} -> exit 2")
        return 2

    after_tag, after_m = marks[-1]
    bad = [k for k in ("G3", "G5", "G6") if after_m[k] != "P"]
    if after_m["C"] != "PASS":
        bad.append("vegetation")
    if bad:
        print(f"SHEET VERDICT: FAIL on the AFTER panel ({after_tag.strip()}): "
              f"{', '.join(bad)} -> exit 1")
        return 1
    print(f"SHEET VERDICT: PASS on the AFTER panel ({after_tag.strip()}) -> exit 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
