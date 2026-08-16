"""Offline fixture test for _rose_ai_pairs.py (Rose). Builds a synthetic
trace.csv + fake frame PNGs under scripts/_rose_test_fixture/, runs the
picker, checks the expected pairs come out. Delete after use."""
import csv
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FIX = os.path.join(ROOT, "scripts", "_rose_test_fixture")
FRAMES = os.path.join(FIX, "frames")
OUT = os.path.join(FIX, "out")
os.makedirs(FRAMES, exist_ok=True)
os.makedirs(OUT, exist_ok=True)
for d in (FRAMES, OUT):
    for f in os.listdir(d):
        os.remove(os.path.join(d, f))

# fake PNGs: one per even frame 2..200 (capture cadence = every 2nd app frame)
png_header = bytes.fromhex("89504e470d0a1a0a") + b"\x00" * 32  # >0 bytes, not a real PNG
for f in range(2, 201, 2):
    with open(os.path.join(FRAMES, "f%04d.png" % f), "wb") as fh:
        fh.write(png_header)

def state_for(t):
    # enemy 2 does the full arc; enemy 3 alerts a bit later; enemy 1 just patrols
    if t < 71: return "patrol"
    if t < 101: return "alert"
    if t < 150: return "chase"
    if t < 160: return "windup"
    if t < 170: return "strike"
    if t < 190: return "recover"
    return "patrol"

def dist_for(t):
    if t < 101: return 22.0
    if t < 140: return max(4.0, 22.0 - (t - 101) * 0.4)
    return 3.0

rows = []
for t in range(1, 201):
    rows.append(dict(frame=t, enemy="2", state=state_for(t), archetype="swarm", x=-t * 0.1, z=1.0, dist=dist_for(t)))
    rows.append(dict(frame=t, enemy="3", state="patrol" if t < 90 else "alert", archetype="bruiser", x=2.0, z=2.0, dist=30.0))
    rows.append(dict(frame=t, enemy="1", state="patrol", archetype="pouncer", x=0.0, z=-5.0, dist=40.0))

trace = os.path.join(FIX, "trace.csv")
with open(trace, "w", newline="", encoding="utf-8") as fh:
    w = csv.DictWriter(fh, fieldnames=["frame", "enemy", "state", "archetype", "x", "z", "dist"])
    w.writeheader()
    w.writerows(rows)

r = subprocess.run([sys.executable, os.path.join(ROOT, "scripts", "_rose_ai_pairs.py"), trace, FRAMES, OUT],
                   capture_output=True, text=True)
print(r.stdout)
print(r.stderr, file=sys.stderr)
expect = ["ai-a-before-detect.png", "ai-b-after-detect.png",
          "ai-a-before-chase.png", "ai-b-after-chase.png",
          "ai-a-before-strike.png", "ai-b-after-strike.png",
          "ai-a-before-retreat.png", "ai-b-after-retreat.png"]
bad = [n for n in expect if not os.path.exists(os.path.join(OUT, n)) or os.path.getsize(os.path.join(OUT, n)) == 0]
print("exit=%d missing_or_empty=%s" % (r.returncode, bad or "none"))
if r.returncode != 0 or bad:
    sys.exit(1)

# ---- negative case: same trace minus every strike/pounce/recover/windup row
# -> only 2 pairs possible -> the picker MUST exit 1 and report the missing
# behaviours (never silently pass on an incomplete trace)
neg_lines = [l for l in open(trace, encoding="utf-8-sig")
             if not any(s in l for s in (",strike,", ",pounce,", ",recover,", ",windup,", ",crouch,"))]
neg_trace = os.path.join(FIX, "trace_neg.csv")
open(neg_trace, "w", encoding="utf-8", newline="").writelines(neg_lines)
neg_out = os.path.join(FIX, "out_neg")
os.makedirs(neg_out, exist_ok=True)
for f in os.listdir(neg_out):
    os.remove(os.path.join(neg_out, f))
rn = subprocess.run([sys.executable, os.path.join(ROOT, "scripts", "_rose_ai_pairs.py"), neg_trace, FRAMES, neg_out],
                    capture_output=True, text=True)
print(rn.stdout)
print(rn.stderr, file=sys.stderr)
neg_ok = rn.returncode == 1 and "MISSING strike" in rn.stdout and "MISSING retreat" in rn.stdout
neg_files = [f for f in os.listdir(neg_out) if f.startswith("ai-")]
print("negative: exit=%d (want 1) missing-reported=%s files=%s" % (rn.returncode, neg_ok, sorted(neg_files)))
sys.exit(0 if neg_ok else 1)
