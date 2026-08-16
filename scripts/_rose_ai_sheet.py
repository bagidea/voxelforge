"""Contact sheet for the enemy-AI proof clip (Rose, 2026-08-14).

Reads the trace.csv the proof bin itself wrote, finds the app-frame where each
of the four headline states FIRST appears (patrol, alert, pursuit, telegraphed
strike — per the archetype that best shows it), then composes those four
captured PNGs into one labelled 2x2 sheet.

Frame pickers (all from CSV facts, not eyeballing):
  patrol    — earliest frame whose state == 'patrol' (id that later leaves it)
  alert     — earliest 'alert'
  pursuit   — earliest frame dist<10 while state in {chase, stalk} (closing in)
  strike    — earliest frame with TELEGRAPH->COMMIT in the runlog, i.e. the
              frame just after windup ends (strike/pounce state), where the
              locked lunge is visible mid-travel

Usage: python scripts/_rose_ai_sheet.py <trace.csv> <runlog.txt> <frames_dir> <out.png>
"""
import csv
import os
import re
import sys
from PIL import Image, ImageDraw

WANT = ["patrol", "alert", "pursuit", "strike"]


def pick(trace_path, runlog_path):
    rows = []
    with open(trace_path, encoding="utf-8") as f:
        for r in csv.DictReader(f):
            r["frame"] = int(r["frame"])
            r["dist"] = float(r["dist"])
            rows.append(r)

    def first(pred):
        for r in rows:
            if pred(r):
                return r
        return None

    out = {"patrol": first(lambda r: r["state"] == "patrol"),
           "alert": first(lambda r: r["state"] == "alert"),
           "pursuit": first(lambda r: r["dist"] < 10.0 and r["state"] in ("chase", "stalk"))}

    # strike: frame of the FIRST telegraph->commit line in the bin's own log
    commit = None
    with open(runlog_path, encoding="utf-8") as f:
        for line in f:
            if "TELEGRAPH->COMMIT" in line:
                m = re.search(r"id=(\d+)", line)
                commit = m and m.group(1)
                break
    if commit:
        out["strike"] = first(lambda r: r["enemy"] == commit and r["state"] in ("strike", "pounce"))
    else:
        out["strike"] = first(lambda r: r["state"] in ("strike", "pounce"))
    return out


def main():
    trace, runlog, frames_dir, out_png = sys.argv[1:5]
    picks = pick(trace, runlog)
    cell_w, cell_h, pad, cap = 640, 360, 12, 34
    img = Image.new("RGB", (cell_w * 2 + pad * 3, (cell_h + cap) * 2 + pad * 3), "#0c0e14")
    d = ImageDraw.Draw(img)

    for i, name in enumerate(WANT):
        r = picks.get(name)
        col, row = i % 2, i // 2
        x = pad + col * (cell_w + pad)
        y = pad + row * (cell_h + cap + pad)
        label = f"{i+1}. {name.upper()}"
        if r is None:
            d.text((x + 8, y + 10), label + "  (state never appeared — MISSING)", fill="#ff6b6b")
            continue
        label += f"   frame {r['frame']}  {r['archetype']} #{r['enemy']}  dist {r['dist']:.1f}"
        png = os.path.join(frames_dir, f"f{r['frame']:04d}.png")
        # capture is every 2nd frame from `start`; fall back to neighbours
        for cand in (png, png.replace(f"f{r['frame']:04d}", f"f{r['frame'] + 1:04d}"),
                     png.replace(f"f{r['frame']:04d}", f"f{r['frame'] - 1:04d}")):
            if os.path.exists(cand):
                png = cand
                break
        if os.path.exists(png):
            cell = Image.open(png).resize((cell_w, cell_h))
            img.paste(cell, (x, y))
        else:
            d.text((x + 8, y + 10), f"{label}  (frame PNG missing)", fill="#ff6b6b")
        d.text((x + 4, y + cell_h + 6), label, fill="#e8e8f0")

    img.save(out_png)
    print(f"wrote {out_png} ({img.size[0]}x{img.size[1]}, {os.path.getsize(out_png)} bytes)")
    for name in WANT:
        r = picks.get(name)
        print(f"  {name:8s} -> {r['frame'] if r else 'MISSING'}")


if __name__ == "__main__":
    main()
