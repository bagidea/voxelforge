# Rose 2026-08-16 — round-2 targeted search: the winning pair from round 1 is
# only acceptable if the caption's subject is what actually changed on screen.
# detect: subject must be the enemy whose distance moved the most between A
# and B (that enemy gets the caption), A must be a quiet all-patrol scene.
# pouncer: A must show the stalker stalking with NO enemy point-blank (<2.5)
# so the stalker isn't upstaged, and the frame must not be near-black.
import csv, re
import numpy as np
from PIL import Image
from datetime import datetime

ts, pat, tpat = {}, re.compile(r'_frames/f(\d+)\.png'), re.compile(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+)')
for line in open('runlog.txt', encoding='utf-8', errors='ignore'):
    m = pat.search(line)
    if m:
        t = tpat.search(line)
        if t:
            ts[int(m.group(1))] = datetime.fromisoformat(t.group(1)).timestamp()

rows = {(int(r['frame']), int(r['enemy'])): r for r in csv.DictReader(open('trace.csv'))}
def st(f, e):
    r = rows.get((f, e)); return r['state'] if r else None
def dd(f, e):
    r = rows.get((f, e)); return float(r['dist']) if r else 99.0

FR = {}
def arr(f):
    if f not in FR:
        FR[f] = np.asarray(Image.open(f"_frames_lifted/f{f:04d}.png").convert('RGB')).astype(np.int16)
    return FR[f]
def score(a, b):
    return (np.abs(arr(a)-arr(b)).max(axis=2) > 8).mean() * 100
def bright(f):
    return int((arr(f).mean(axis=2) > 70).sum())

print("-- detect: A = quiet all-patrol even frame 66..80 ; B = even 86..96 --")
print("   subject = enemy with the largest dist change; must be its detect arc")
for a in range(66, 81, 2):
    if a == 82 or any(st(a, e) != 'patrol' for e in (1, 2, 3, 4)):
        continue
    for b in range(86, 97, 2):
        movers = sorted(((dd(a, e)-dd(b, e), e, st(b, e)) for e in (1, 2, 3, 4)), reverse=True)
        dm, e, sb = movers[0]
        print(f"  A=f{a:04d} B=f{b:04d} diff={score(a,b):5.1f}%  top-mover e{e} {st(a,e)}->{sb} d{dd(a,e):.2f}->{dd(b,e):.2f} (delta {dm:.2f})")

print("\n-- pouncer: A = stalker stalk 6..9.5, all others >=2.5, bright>=2500, --")
print("--          >=8 frames from any used frame ; B = pounce d<=1.1, b-a>=40 --")
used = {60, 284, 82, 424, 180, 274, 114, 130, 148, 160, 62, 64, 66, 68, 70, 74, 76, 78}
okA = []
for a in range(92, 407, 2):
    if st(a, 4) != 'stalk' or not (6.0 <= dd(a, 4) <= 9.5):
        continue
    if any(dd(a, e) < 2.5 for e in (1, 2, 3)):
        continue
    if any(abs(a-u) < 8 for u in used):
        continue
    if bright(a) < 2500:
        continue
    okA.append(a)
print("   candidate A frames:", [f"f{a:04d}(d{dd(a,4):.2f},br{bright(a)})" for a in okA[:12]])
B = [b for b in range(160, 1319, 2) if st(b, 4) == 'pounce' and dd(b, 4) <= 1.1]
res = sorted(((score(a, b), a, b) for a in okA for b in B if b-a >= 40), reverse=True)[:6]
for s, a, b in res:
    print(f"  A=f{a:04d} stalk d={dd(a,4):.2f} (others {dd(a,1):.1f}/{dd(a,2):.1f}/{dd(a,3):.1f}) "
          f"-> B=f{b:04d} pounce d={dd(b,4):.2f}  dt={ts[b]-ts[a]:.1f}s diff={s:.1f}%")
