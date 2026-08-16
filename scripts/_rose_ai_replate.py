# Rose 2026-08-16 — re-plate the AI-proof evidence stills (v2).
# Review verdict on v1: pairs were 2-4 frames apart (0.6-2.5% px diff) so the
# stills showed no behaviour, and ai-arch-swarm-a-before was byte-identical
# to ai-a-before-chase. v2 rebuilds all 14 stills from raw _frames/ using
# pairs picked by scripts/_rose_ai_pairsearch.py under trace-state
# constraints (seconds-to-tens-of-seconds apart, state-changing), applies
# the SAME gamma-0.32 lift, prints full-res diff stats + wall-clock dt from
# runlog.txt screenshot timestamps, builds labelled A/B paircheck sheets,
# and asserts every plate is a distinct file.
# Re-run: python scripts/_rose_ai_replate.py --dir docs/assets/ai
import sys, os, re, hashlib
from datetime import datetime
import numpy as np
from PIL import Image, ImageDraw

GAMMA = 0.32
PLATES = {
    # archetype pairs — subject enemy in parens (state, dist from trace.csv)
    "ai-arch-swarm-a-before":   ("f0060", "reaver x2 patrol far (e1 16.07 / e2 15.49)"),
    "ai-arch-swarm-b-after":    ("f0284", "reaver x2 on the player (e1 strike 0.85 / e2 recover 0.61)"),
    "ai-arch-bruiser-a-before": ("f0082", "sentinel patrol (e3 13.40)"),
    "ai-arch-bruiser-b-after":  ("f0424", "sentinel strike lunge (e3 2.30); reavers on player 0.6-1.0"),
    "ai-arch-pouncer-a-before": ("f0104", "stalker stalking, pack mid-range (e4 9.33)"),
    "ai-arch-pouncer-b-after":  ("f0944", "stalker pounce contact (e4 0.90)"),
    # behaviour pairs
    "ai-a-before-detect":  ("f0086", "reaver#2 alert, just detected (e2 11.14)"),
    "ai-b-after-detect":   ("f0116", "reaver#2 committed chase (e2 4.45); sentinel alert 8.39"),
    "ai-a-before-chase":   ("f0180", "reaver #1 chase start (e1 11.59)"),
    "ai-b-after-chase":    ("f0274", "reaver #1 chase end (e1 2.26); reaver#2 strikes 0.92"),
    "ai-a-before-strike":  ("f0114", "reaver #2 closing to strike (e2 4.45)"),
    "ai-b-after-strike":   ("f0130", "reaver #2 strike point-blank (e2 0.45)"),
    "ai-a-before-retreat": ("f0148", "reaver #2 post-strike (e2 1.05)"),
    "ai-b-after-retreat":  ("f0160", "reaver #2 recover, backed off (e2 2.82)"),
}
PAIRS = [
    ("swarm",   "ai-arch-swarm-a-before",   "ai-arch-swarm-b-after"),
    ("bruiser", "ai-arch-bruiser-a-before", "ai-arch-bruiser-b-after"),
    ("pouncer", "ai-arch-pouncer-a-before", "ai-arch-pouncer-b-after"),
    ("detect",  "ai-a-before-detect",  "ai-b-after-detect"),
    ("chase",   "ai-a-before-chase",   "ai-b-after-chase"),
    ("strike",  "ai-a-before-strike",  "ai-b-after-strike"),
    ("retreat", "ai-a-before-retreat", "ai-b-after-retreat"),
]

root = sys.argv[sys.argv.index("--dir") + 1] if "--dir" in sys.argv else "."
frames = os.path.join(root, "_frames")

# wall-clock seconds per captured frame, from runlog screenshot timestamps
ts, pat, tpat = {}, re.compile(r'_frames/f(\d+)\.png'), re.compile(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+)')
for line in open(os.path.join(root, "runlog.txt"), encoding='utf-8', errors='ignore'):
    m = pat.search(line)
    if m:
        t = tpat.search(line)
        if t:
            ts[int(m.group(1))] = datetime.fromisoformat(t.group(1)).timestamp()

def lift(img):
    a = np.asarray(img.convert("RGB")).astype(np.float32) / 255.0
    return Image.fromarray((np.clip(a, 0, 1) ** GAMMA * 255.0 + 0.5).astype(np.uint8))

made = {}
for name, (frame, _cap) in PLATES.items():
    out = lift(Image.open(os.path.join(frames, frame + ".png")))
    dst = os.path.join(root, name + ".png")
    out.save(dst)
    made[name] = (frame, dst)

print(f"{'pair':9s} {'A':>5s} {'B':>5s} {'dt_wall(s)':>10s} {'%px|d|>8':>9s} {'%px|d|>24':>10s} {'mean|d|':>8s}")
for label, an, bn in PAIRS:
    fa, fb = made[an][0], made[bn][0]
    A = np.asarray(Image.open(made[an][1]).convert("RGB"), dtype=np.int16)
    B = np.asarray(Image.open(made[bn][1]).convert("RGB"), dtype=np.int16)
    d = np.abs(A - B)
    n = d[..., 0].size
    pct8 = (d.max(axis=2) > 8).sum() / n * 100
    pct24 = (d.max(axis=2) > 24).sum() / n * 100
    dt = ts[int(fb[1:])] - ts[int(fa[1:])]
    print(f"{label:9s} {fa:>5s} {fb:>5s} {dt:10.1f} {pct8:8.1f}% {pct24:9.1f}% {d.mean():8.2f}")
    H, W = A.shape[:2]
    Hbar = 26
    sheet = Image.new("RGB", (W, H * 2 + Hbar * 2 + 2), (0, 0, 0))
    dr = ImageDraw.Draw(sheet)
    sheet.paste(Image.open(made[an][1]), (0, Hbar))
    sheet.paste(Image.open(made[bn][1]), (0, Hbar * 2 + H + 2))
    dr.text((8, 6), f"A {fa} {PLATES[an][1]}", fill=(255, 255, 255))
    dr.text((8, Hbar + H + 8), f"B {fb} {PLATES[bn][1]}  (dt {dt:.1f}s)", fill=(255, 255, 255))
    sheet.save(os.path.join(root, f"_paircheck_{label}.png"))

hashes, dups = {}, []
for name, (frame, dst) in made.items():
    h = hashlib.md5(open(dst, "rb").read()).hexdigest()
    if h in hashes.values():
        dups.append((name, [k for k, v in hashes.items() if v == h][0]))
    hashes[name] = h
print("md5 uniqueness:", "FAIL " + str(dups) if dups else "PASS (14/14 distinct)")
