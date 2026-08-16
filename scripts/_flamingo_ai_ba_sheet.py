#!/usr/bin/env python
"""Assemble the husk-AI BEFORE/AFTER contact sheets from REAL captured frames.

BEFORE = the shipped Guard Husk AI (`combat.rs` §4.1) shot through the real
         game by `VOXELFORGE_AI_DEMO`, which snaps one still the first frame
         each husk state appears (`_fl_ai_before/ai_<state>.png`).
AFTER  = rose's archetype AI (`enemy_ai.rs`) shot by `voxelforge_enemyai_proof`
         (`_fl_ai_after/f%04d.png` + `trace.csv`).

Beat picking for AFTER is driven by trace.csv — the CSV the sim writes itself —
not by eyeballing, so the panel labelled "alert" really is the frame where the
state column first says alert.  Nothing here draws a frame: if a beat has no
captured frame it is reported missing and the panel is dropped, never faked.
"""
import csv
import os
import sys
import time
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
AFTER_DIR = os.path.join(ROOT, "_fl_ai_after")
BEFORE_DIR = os.path.join(ROOT, "_fl_ai_before")
OUT_DIR = os.path.join(ROOT, "docs", "assets", "ai")
os.makedirs(OUT_DIR, exist_ok=True)

PANEL_W, PANEL_H = 640, 360
PAD, CAP_H, HEAD_H, FOOT_H = 10, 34, 58, 30
INK = (233, 237, 245)
DIM = (150, 160, 178)
BG = (14, 16, 20)


def font(sz, bold=False):
    for p in (r"C:\Windows\Fonts\segoeuib.ttf" if bold else r"C:\Windows\Fonts\segoeui.ttf",
              r"C:\Windows\Fonts\arialbd.ttf" if bold else r"C:\Windows\Fonts\arial.ttf"):
        try:
            return ImageFont.truetype(p, sz)
        except OSError:
            continue
    return ImageFont.load_default()


def stamp(path):
    """Provenance line: which exe/file, how big, when — so a frame can never be
    silently re-attributed to a different binary later."""
    try:
        st = os.stat(path)
        return "%s  %d bytes  %s" % (
            os.path.relpath(path, ROOT).replace("\\", "/"), st.st_size,
            time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(st.st_mtime)))
    except OSError:
        return "%s  MISSING" % path


def sheet(panels, title, subtitle, footer, out):
    """panels = [(PIL.Image, caption)] laid out 2x2."""
    if not panels:
        print("SKIP %s — no panels" % out)
        return None
    cols = 2 if len(panels) > 1 else 1
    rows = (len(panels) + cols - 1) // cols
    W = PAD + cols * (PANEL_W + PAD)
    H = HEAD_H + PAD + rows * (PANEL_H + CAP_H + PAD) + FOOT_H
    img = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(img)
    d.text((PAD + 2, 10), title, font=font(24, True), fill=INK)
    d.text((PAD + 2, 36), subtitle, font=font(13), fill=DIM)
    for i, (frame, cap) in enumerate(panels):
        cx = PAD + (i % cols) * (PANEL_W + PAD)
        cy = HEAD_H + PAD + (i // cols) * (PANEL_H + CAP_H + PAD)
        img.paste(frame.resize((PANEL_W, PANEL_H), Image.LANCZOS), (cx, cy))
        d.rectangle([cx, cy, cx + PANEL_W - 1, cy + PANEL_H - 1], outline=(52, 58, 70))
        d.text((cx + 2, cy + PANEL_H + 7), cap, font=font(15, True), fill=INK)
    d.text((PAD + 2, H - FOOT_H + 6), footer, font=font(12), fill=DIM)
    img.save(out)
    print("WROTE %s  %d bytes  %dx%d" % (out, os.path.getsize(out), W, H))
    return img


# ---------------------------------------------------------------- AFTER ----
def build_after():
    trace = os.path.join(AFTER_DIR, "trace.csv")
    if not os.path.isfile(trace):
        print("AFTER: no trace.csv at %s" % trace)
        return None
    rows = []
    with open(trace, newline="") as fh:
        for r in csv.DictReader(fh):
            try:
                rows.append((int(r["frame"]), r["state"].strip().lower(),
                             r["archetype"].strip().lower(), float(r["dist"])))
            except (ValueError, KeyError, TypeError):
                continue
    if not rows:
        print("AFTER: trace.csv has no rows")
        return None
    have = {}
    for f in os.listdir(AFTER_DIR):
        if f.startswith("f") and f.endswith(".png"):
            try:
                have[int(f[1:-4])] = os.path.join(AFTER_DIR, f)
            except ValueError:
                pass
    print("AFTER: %d trace rows, %d captured frames" % (len(rows), len(have)))
    if not have:
        return None

    def first_frame(match):
        for fr, st, arch, dist in rows:
            if match(st, arch):
                return fr, st, arch, dist
        return None

    beats = [
        ("patrol", lambda st, a: st.startswith("patrol"), "Patrol — wander around home, unaware"),
        ("alert", lambda st, a: st.startswith("alert"), "Alert — freeze and FACE the player (the tell)"),
        ("pursuit", lambda st, a: st.startswith(("chase", "advance", "stalk")),
         "Pursuit — 3 shapes at once: zigzag / relentless / orbit"),
        ("commit", lambda st, a: st.startswith(("strike", "pounce", "windup", "crouch")),
         "Telegraph -> COMMIT — strike direction locked at windup end"),
    ]
    panels, notes = [], []
    used = set()
    for key, match, cap in beats:
        hit = first_frame(match)
        if hit is None:
            print("AFTER: beat %s never appears in trace" % key)
            continue
        fr, st, arch, dist = hit
        cands = sorted(have, key=lambda n: (abs(n - fr), n))
        pick = next((n for n in cands if n not in used), None)
        if pick is None:
            continue
        used.add(pick)
        panels.append((Image.open(have[pick]).convert("RGB"),
                       "%s   [frame %d · state=%s · %s · dist %.1f]" % (cap, pick, st, arch, dist)))
        notes.append("%s@f%d(%s/%s)" % (key, pick, st, arch))
    exe = os.path.join(ROOT, "target-flamingo", "debug", "voxelforge_enemyai_proof.exe")
    sheet(panels,
          "AFTER — new enemy AI (enemy_ai.rs · Swarm / Bruiser / Pouncer)",
          "Real capture from voxelforge_enemyai_proof · beats picked from the sim's own trace.csv · "
          + " ".join(notes),
          "exe: " + stamp(exe) + "   |   trace: " + stamp(trace),
          os.path.join(OUT_DIR, "after-enemy-ai.png"))
    return panels


# --------------------------------------------------------------- BEFORE ----
BEFORE_BEATS = [
    ("ai_patrol.png", "Patrol — the lone husk walks its post"),
    ("ai_alert.png", "Alert — it heard you"),
    ("ai_chase.png", "Chase — one straight line at the player"),
    ("ai_telegraph.png", "Telegraph — windup before the swing"),
    ("ai_attack_swing.png", "Attack — the swing"),
    ("ai_reposition.png", "Reposition"),
]


def build_before():
    if not os.path.isdir(BEFORE_DIR):
        print("BEFORE: %s does not exist" % BEFORE_DIR)
        return None
    panels = []
    for fn, cap in BEFORE_BEATS:
        p = os.path.join(BEFORE_DIR, fn)
        if os.path.isfile(p) and os.path.getsize(p) > 1024:
            panels.append((Image.open(p).convert("RGB"), "%s   [%s]" % (cap, fn)))
        else:
            print("BEFORE: missing %s" % fn)
        if len(panels) == 4:
            break
    print("BEFORE: %d panels" % len(panels))
    exe = os.path.join(ROOT, "target-flamingo", "debug", "voxelforge.exe")
    sheet(panels,
          "BEFORE — shipped Guard Husk AI (combat.rs §4.1)",
          "Real capture from the real game: VOXELFORGE_PLAY=1 VOXELFORGE_AI_DEMO=1 — "
          "one still the first frame each husk state appears",
          "exe: " + stamp(exe),
          os.path.join(OUT_DIR, "before-husk-ai.png"))
    return panels


# ------------------------------------------------- top-down pursuit plot ----
TRACK_COLORS = [(255, 122, 122), (122, 200, 255), (168, 255, 150), (255, 205, 120),
                (216, 150, 255), (140, 240, 220)]
PLAYER_COLOR = (255, 214, 130)


def read_before_tracks():
    """`t,entity,x,z,state,dist,evading` — combat.rs's own AI_TRACE dump."""
    p = os.path.join(BEFORE_DIR, "trace.csv")
    if not os.path.isfile(p):
        return None
    tracks = {}
    with open(p, newline="") as fh:
        for r in csv.DictReader(fh):
            try:
                tracks.setdefault(r["entity"], []).append((float(r["x"]), float(r["z"])))
            except (ValueError, KeyError, TypeError):
                continue
    return tracks or None


def read_after_tracks():
    """`frame,enemy,state,archetype,x,z,dist` — the proof bin's own trace."""
    p = os.path.join(AFTER_DIR, "trace.csv")
    if not os.path.isfile(p):
        return None
    tracks, arch = {}, {}
    with open(p, newline="") as fh:
        for r in csv.DictReader(fh):
            try:
                k = r["enemy"]
                tracks.setdefault(k, []).append((float(r["x"]), float(r["z"])))
                arch[k] = r["archetype"]
            except (ValueError, KeyError, TypeError):
                continue
    if not tracks:
        return None
    return {("%s (%s)" % (k, arch.get(k, "?"))): v for k, v in tracks.items()}


def draw_panel(img, box, tracks, title, sub):
    """Equal-aspect top-down draw of every track, auto-fitted to the box."""
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    d.rectangle(box, fill=(10, 12, 16), outline=(52, 58, 70))
    d.text((x0 + 8, y0 + 6), title, font=font(17, True), fill=INK)
    d.text((x0 + 8, y0 + 28), sub, font=font(12), fill=DIM)
    pts = [p for v in tracks.values() for p in v]
    if not pts:
        return
    minx = min(p[0] for p in pts); maxx = max(p[0] for p in pts)
    minz = min(p[1] for p in pts); maxz = max(p[1] for p in pts)
    span = max(maxx - minx, maxz - minz, 1.0) * 1.08
    cx, cz = (minx + maxx) / 2.0, (minz + maxz) / 2.0
    pad_t = 52
    side = min(x1 - x0 - 24, y1 - y0 - pad_t - 34)
    ox = x0 + (x1 - x0 - side) / 2.0
    oy = y0 + pad_t + (y1 - y0 - pad_t - 34 - side) / 2.0
    sc = side / span

    def to_px(p):
        return (ox + side / 2.0 + (p[0] - cx) * sc, oy + side / 2.0 + (p[1] - cz) * sc)

    # 5-unit scale bar — the two panels cover different arenas, so the shapes
    # are only comparable with the world scale drawn on each.
    bar = 5.0 * sc
    by = y1 - 20
    d.line([(x0 + 12, by), (x0 + 12 + bar, by)], fill=DIM, width=2)
    d.text((x0 + 16 + bar, by - 8), "5 units", font=font(11), fill=DIM)

    keys = sorted(tracks)
    ly = y0 + 46
    for i, k in enumerate(keys):
        pl = tracks[k]
        col = PLAYER_COLOR if k.lower().startswith("player") else TRACK_COLORS[i % len(TRACK_COLORS)]
        px = [to_px(p) for p in pl]
        step = max(1, len(px) // 4000)
        px = px[::step]
        if len(px) > 1:
            d.line(px, fill=col, width=3 if k.lower().startswith("player") else 2, joint="curve")
            d.ellipse([px[0][0] - 4, px[0][1] - 4, px[0][0] + 4, px[0][1] + 4], outline=col, width=2)
            d.ellipse([px[-1][0] - 5, px[-1][1] - 5, px[-1][0] + 5, px[-1][1] + 5], fill=col)
        d.text((x1 - 168, ly), "%s  (%d pts)" % (k[:18], len(pl)), font=font(11), fill=col)
        ly += 15


def build_plot():
    b, a = read_before_tracks(), read_after_tracks()
    if not b and not a:
        print("PLOT: no traces on either side")
        return
    W, H, P = 1360, 700, 12
    img = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(img)
    d.text((P + 2, 8), "Pursuit shape, top-down — same measurement on both sides (each run's own CSV trace)",
           font=font(19, True), fill=INK)
    d.text((P + 2, 33), "hollow ring = where the body started · filled dot = where it ended · "
                        "paths are raw x/z from the sim, nothing smoothed", font=font(12), fill=DIM)
    pw = (W - 3 * P) // 2
    box_h = H - 62 - P
    draw_panel(img, (P, 58, P + pw, 58 + box_h), b or {},
               "BEFORE — shipped Guard Husk AI",
               "combat.rs §4.1 · VOXELFORGE_AI_TRACE (player row included)")
    draw_panel(img, (2 * P + pw, 58, 2 * P + 2 * pw, 58 + box_h), a or {},
               "AFTER — enemy_ai.rs archetypes",
               "voxelforge_enemyai_proof trace.csv · enemies only (no player row in this trace)")
    out = os.path.join(OUT_DIR, "husk-ai-pursuit-plot.png")
    img.save(out)
    print("WROTE %s  %d bytes" % (out, os.path.getsize(out)))


def main():
    before = build_before()
    after = build_after()
    build_plot()
    a = os.path.join(OUT_DIR, "before-husk-ai.png")
    b = os.path.join(OUT_DIR, "after-enemy-ai.png")
    if os.path.isfile(a) and os.path.isfile(b):
        ia, ib = Image.open(a), Image.open(b)
        W = max(ia.width, ib.width)
        H = ia.height + ib.height + 12
        c = Image.new("RGB", (W, H), BG)
        c.paste(ia, (0, 0))
        c.paste(ib, (0, ia.height + 12))
        out = os.path.join(OUT_DIR, "husk-ai-before-after.png")
        c.save(out)
        print("WROTE %s  %d bytes" % (out, os.path.getsize(out)))
    for f in sorted(os.listdir(OUT_DIR)):
        p = os.path.join(OUT_DIR, f)
        if os.path.isfile(p):
            print("ARTIFACT %s  %d bytes" % (p, os.path.getsize(p)))
    return 0 if (before and after) else 2


if __name__ == "__main__":
    sys.exit(main())
