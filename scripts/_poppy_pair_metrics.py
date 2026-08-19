# -*- coding: utf-8 -*-
"""Compare a before/after screenshot pair and build the side-by-side sheet.

Prints, for the pair:
  * md5 of each file          - two identical hashes means the env flip did
                                nothing and there is no "after" to report
  * mean |d| over RGB         - average absolute per-channel difference, 0..255
  * % pixels changed          - share of pixels differing by more than a
                                quantisation threshold (default 2/255), so PNG
                                round-tripping and dither noise are not counted
                                as a change

Usage:
  python scripts/_poppy_pair_metrics.py BEFORE.png AFTER.png [SHEET.png] \
      [--label-a TEXT] [--label-b TEXT]
"""
import hashlib
import sys

from PIL import Image, ImageDraw

THRESH = 2  # per-channel, out of 255


def md5(path):
    h = hashlib.md5()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    opts = {}
    i = 0
    while i < len(argv):
        if argv[i].startswith("--"):
            opts[argv[i][2:]] = argv[i + 1] if i + 1 < len(argv) else ""
            i += 2
        else:
            i += 1
    if len(args) < 2:
        print(__doc__)
        return 2
    a_path, b_path = args[0], args[1]
    sheet = args[2] if len(args) > 2 else None

    a = Image.open(a_path).convert("RGB")
    b = Image.open(b_path).convert("RGB")
    if a.size != b.size:
        print("FAIL: sizes differ %s vs %s" % (a.size, b.size))
        return 1

    ap, bp = a.load(), b.load()
    w, h = a.size
    total = w * h
    changed = 0
    acc = 0
    for y in range(h):
        for x in range(w):
            pa, pb = ap[x, y], bp[x, y]
            d = (abs(pa[0] - pb[0]), abs(pa[1] - pb[1]), abs(pa[2] - pb[2]))
            acc += d[0] + d[1] + d[2]
            if max(d) > THRESH:
                changed += 1

    ha, hb = md5(a_path), md5(b_path)
    print("A  %s  md5=%s" % (a_path, ha))
    print("B  %s  md5=%s" % (b_path, hb))
    print("size        %dx%d" % (w, h))
    print("md5 differ  %s" % (ha != hb))
    print("mean |d|    %.3f  (0..255 per channel)" % (acc / float(total * 3)))
    print("changed     %.2f %%  (>%d/255 on any channel)" % (100.0 * changed / total, THRESH))

    if sheet:
        pad, bar = 12, 34
        out = Image.new("RGB", (w * 2 + pad * 3, h + bar + pad * 2), (18, 18, 22))
        out.paste(a, (pad, pad + bar))
        out.paste(b, (pad * 2 + w, pad + bar))
        d = ImageDraw.Draw(out)
        d.text((pad + 4, pad + 8), opts.get("label-a", "BEFORE"), fill=(235, 235, 240))
        d.text((pad * 2 + w + 4, pad + 8), opts.get("label-b", "AFTER"), fill=(235, 235, 240))
        out.save(sheet)
        print("sheet       %s" % sheet)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
