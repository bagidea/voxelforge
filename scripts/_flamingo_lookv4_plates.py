"""The plates this grade is about, pinned to the commit that shipped them.

Why this exists: `docs/assets/look/*.png` is a LIVE path. Poppy re-shot all six
plates in the working tree while this sheet was being cut, so a sheet that reads
the path would silently change meaning every time she re-renders, and nobody
could ever check a published number against the pixels it came from.

So the grade is pinned to PLATE_REV - the commit that shipped the v4 plate set.
`git show <rev>:<path>` reproduces exactly what was graded, forever, and the
md5s printed here are the ones on the sheet footer.

Working-tree plates are NOT graded here. When they differ, that is a delta to
report, not a silent re-grade.

EVERY tool that feeds a number onto the sheet must resolve its input through
`plate()` below, not through `docs/assets/look/`. Default is pinned. Set
LOOKV4_LIVE=1 to point the same code at the working tree - that is how a delta
gets measured without hand-editing a script, and `banner()` prints which set
was read so no output is ever ambiguous about its own source.
"""
import hashlib
import os
import subprocess

PLATE_REV = 'b361a9d'  # feat(look): v4 gate-green look pass + first before/after plate set
SRC = 'docs/assets/look/%s_%s.png'
CACHE = os.path.join('_fl_lookv4', 'pinned')
SCENES = ['outdoor-noon', 'evening-raking', 'night-firelit']
LIVE = os.environ.get('LOOKV4_LIVE') == '1'


def path(scene, variant):
    """Local path to the pinned plate, extracted on first use."""
    os.makedirs(CACHE, exist_ok=True)
    dst = os.path.join(CACHE, '%s_%s.png' % (scene, variant))
    if not os.path.exists(dst):
        blob = subprocess.check_output(
            ['git', 'show', '%s:%s' % (PLATE_REV, SRC % (scene, variant))])
        with open(dst, 'wb') as fh:
            fh.write(blob)
    return dst


def plate(scene, variant='after'):
    """The plate a MEASURING tool should open. Pinned unless LOOKV4_LIVE=1."""
    return (SRC % (scene, variant)) if LIVE else path(scene, variant)


def banner(tool):
    """One line naming the plate set behind everything that follows."""
    if LIVE:
        return '# %s reading LIVE working-tree plates (LOOKV4_LIVE=1) - NOT the graded set' % tool
    return '# %s reading plates pinned at %s' % (tool, PLATE_REV)


def md5(scene, variant, n=10):
    with open(path(scene, variant), 'rb') as fh:
        return hashlib.md5(fh.read()).hexdigest()[:n]


def drifted():
    """Scenes whose working-tree plate no longer matches the pinned one."""
    out = []
    for s in SCENES:
        for v in ('before', 'after'):
            live = SRC % (s, v)
            if not os.path.exists(live):
                continue
            with open(live, 'rb') as fh:
                if hashlib.md5(fh.read()).hexdigest()[:10] != md5(s, v):
                    out.append('%s_%s' % (s, v))
    return out


if __name__ == '__main__':
    for s in SCENES:
        for v in ('before', 'after'):
            print('%-16s %-7s %s  %s' % (s, v, md5(s, v), path(s, v)))
    d = drifted()
    print('\nworking tree differs from %s on: %s' % (PLATE_REV, ', '.join(d) if d else '(nothing)'))
