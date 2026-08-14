"""Flamingo · look-v4 grading probe.

Pulls the per-shot LOOK config out of docs/assets/look/_shoot.log so a
before/after caption can be checked against what the engine actually ran,
instead of against the filename. (Scar: a caption that names a lever it
never proves is worth nothing.)
"""
import os
import sys

LOG = os.path.join('docs', 'assets', 'look', '_shoot.log')


def blocks(path):
    lines = open(path, encoding='utf-8', errors='replace').read().split('\n')
    out, cur = [], None
    for l in lines:
        if l.startswith('CINE eye'):
            cur = []
            out.append(cur)
        if cur is not None:
            cur.append(l)
    return out


def shot_name(b):
    for l in b:
        if l.startswith('SHOT saved'):
            return os.path.basename(l.replace(chr(92), '/').strip())
    return '?'


def main():
    for b in blocks(LOG):
        print('==', shot_name(b))
        for l in b:
            s = l.strip()
            if s.startswith(('LOOK', 'CINE eye')) or 'FogVolume' in s or 'aerial' in s:
                print('   ', s)


if __name__ == '__main__':
    sys.exit(main())
