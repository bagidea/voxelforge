# Rose 2026-08-16 — numeric search for the most visually-different state-valid
# frame pairs, scored at quarter-res on the lifted frames. Constraints come
# from trace.csv states (no free-form frame picking): strike b must be a
# strike frame, detect b an alert frame, retreat b a recover frame, pouncer
# a a long-run stalk frame and b a pounce frame at contact range.
import csv, re
import numpy as np
from PIL import Image
from datetime import datetime

ts = {}
pat = re.compile(r'Screenshot saved to docs/assets/ai/_frames/f(\d+)\.png')
tpat = re.compile(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+)')
for line in open('runlog.txt', encoding='utf-8', errors='ignore'):
    m = pat.search(line)
    if m:
        t = tpat.search(line)
        if t:
            ts[int(m.group(1))] = datetime.fromisoformat(t.group(1)).timestamp()
def sec(f): return ts[f]
print("wall span f0060..f1318: %.1fs (%d frames, %.0f ms/frame)" % (
    sec(1318)-sec(60), 1318-60, (sec(1318)-sec(60))/(1318-60)*1000))

rows = {(int(r['frame']), int(r['enemy'])): r for r in csv.DictReader(open('trace.csv'))}
FR = {}
def small(f):
    if f not in FR:
        FR[f] = np.asarray(Image.open(f"_frames_lifted/f{f:04d}.png").convert('RGB').resize((320, 180))).astype(np.int16)
    return FR[f]
def score(a, b):
    return (np.abs(small(a)-small(b)).max(axis=2) > 10).mean() * 100
def st(f, e):
    r = rows.get((f, e)); return r['state'] if r else None
def dd(f, e):
    r = rows.get((f, e)); return float(r['dist']) if r else None
used = {60, 284, 82, 180, 274, 148, 160}

print("\n-- strike (e2): b=f0130 strike; a even in chase/windup 60..126 --")
for s, a in sorted(((score(a, 130), a) for a in range(60, 127, 2)
                    if st(a, 2) in ('chase', 'windup') and a not in used), reverse=True)[:5]:
    print(f"  a=f{a:04d} {st(a,2)} d={dd(a,2):.2f} -> diff {s:.1f}%")

print("\n-- detect (e3): b alert evens 114/116; a even patrol 60..112 --")
for b in (114, 116):
    for s, a in sorted(((score(a, b), a) for a in range(60, 113, 2)
                        if st(a, 3) == 'patrol' and a not in used), reverse=True)[:3]:
        print(f"  b=f{b:04d} alert d={dd(b,3):.2f} | a=f{a:04d} patrol d={dd(a,3):.2f} diff {s:.1f}%")

print("\n-- retreat (e2): a=f0148 strike; b even recover 152..160 --")
for s, b in sorted(((score(148, b), b) for b in range(152, 161, 2)), reverse=True):
    print(f"  b=f{b:04d} recover d={dd(b,2):.2f} diff {s:.1f}%")

print("\n-- pouncer (e4): a even in long stalk runs; b even pounce d<=1.6, b-a>=40 --")
stalk = [a for run in (range(92, 150, 2), range(190, 296, 2), range(330, 404, 2)) for a in run]
B = [b for b in range(60, 1319, 2) if st(b, 4) == 'pounce' and dd(b, 4) <= 1.6]
for s, a, b in sorted(((score(a, b), a, b) for a in stalk for b in B if b-a >= 40), reverse=True)[:6]:
    print(f"  a=f{a:04d} stalk d={dd(a,4):.2f} -> b=f{b:04d} pounce d={dd(b,4):.2f} dt={sec(b)-sec(a):.1f}s diff {s:.1f}%")

print("\n-- bruiser b: f0424 vs f0428 (a=f0082) --")
print(f"  f0424 d={dd(424,3):.2f}: {score(82,424):.1f}%   f0428 d={dd(428,3):.2f}: {score(82,428):.1f}%")
