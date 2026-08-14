"""Prove the sheet's panels are the files its captions name.

Cheap to skip, expensive to get wrong: a gap sheet whose column 2 is actually
scene 3 is worse than no sheet. This pulls each column's before/after strip
back out of the shipped PNG and matches it against every candidate frame -
the caption is only honest if the named file is the BEST match, not merely
a close one.
"""
import os
import sys

import numpy as np
from PIL import Image

LOOK = os.path.join('docs', 'assets', 'look')
SCENES = ['outdoor-noon', 'evening-raking', 'night-firelit']
W, PAD, GAP = 2560, 34, 26
CW = (W - PAD * 2 - GAP * 2) // 3
BIG_H = int(720 * CW / 1280)
Y0 = 154


def sig(im, n=48):
    a = np.asarray(im.convert('RGB').resize((n, n), Image.LANCZOS)).astype(np.float32)
    a -= a.mean()
    return a / (np.linalg.norm(a) + 1e-6)


def main(sheet_path):
    sheet = Image.open(sheet_path).convert('RGB')
    cand = {'%s_%s' % (s, v): sig(Image.open(os.path.join(LOOK, '%s_%s.png' % (s, v))))
            for s in SCENES for v in ('before', 'after')}
    ok = True
    for i, s in enumerate(SCENES):
        x = PAD + i * (CW + GAP)
        panel = sheet.crop((x, Y0, x + CW, Y0 + BIG_H))
        ps = sig(panel)
        scores = sorted(((float((ps * c).sum()), k) for k, c in cand.items()), reverse=True)
        want = '%s_after' % s
        got, best = scores[0][1], scores[0][0]
        mark = 'OK ' if got == want else 'BAD'
        if got != want:
            ok = False
        print('%s col%d big panel -> %-24s r=%.4f   (caption says %s)   runner-up %s r=%.4f'
              % (mark, i + 1, got, best, want, scores[1][1], scores[1][0]))
    print('VERDICT:', 'every panel matches its caption' if ok else 'MISMATCH - do not ship')
    return 0 if ok else 1


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1] if len(sys.argv) > 1
                          else 'docs/assets/flamingo-beauty-gap-v4-2026-08-15.png'))
