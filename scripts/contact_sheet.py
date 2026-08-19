#!/usr/bin/env python3
"""Compose before/after image pairs into one labeled contact-sheet PNG.

Commit hashes are NEVER hand-typed into captions. For each side you give either:
  --before-file <tracked path>   resolves the real hash via `git log -1 --format=%h -- <path>`
  --before-hash <hash>           uses this hash, but only after `git cat-file -e <hash>^{commit}`
                                  proves it exists in the repo
If neither is given, the caption has no commit tag (a warning is printed so nobody mistakes
a plain note for a verified one). A hash that fails verification aborts the whole run --
no image is written.

Usage (single pair, quick check):
    python scripts/contact_sheet.py --out out.png \
        --before _grassleaves2_BEFORE.png --before-label "BEFORE" \
        --before-file assets/textures/blocks/grass_top.png --before-caption "pre-repaint" \
        --after  _grassleaves2_AFTER.png  --after-label  "AFTER" \
        --after-file  assets/textures/blocks/grass_top.png --after-caption "flower accent removed"

Usage (many pairs, one sheet):
    python scripts/contact_sheet.py --out out.png --manifest pairs.json

manifest.json is a JSON list of rows (any of before_file/before_hash, same for after):
    [
      {
        "before_path": "a_before.png", "before_label": "grass_side",
        "before_file": "assets/textures/blocks/grass_side.png", "before_caption": "pre-repaint",
        "after_path": "a_after.png", "after_label": "grass_side",
        "after_file": "assets/textures/blocks/grass_side.png", "after_caption": "flower accent removed"
      },
      ...
    ]
"""
import argparse
import json
import subprocess
import sys
from datetime import datetime
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

THUMB_W = 640
GAP = 24
MARGIN = 32
LABEL_H = 34
CAPTION_H = 26
SRC_LINE_H = 20
CAPTION_WRAP_LINES = 3
ROW_GAP = 40
FOOTER_H = 28
BG = (24, 24, 28)
LABEL_BG = (40, 100, 60)
LABEL_FG = (255, 255, 255)
CAPTION_FG = (210, 210, 210)
COMMIT_FG = (140, 200, 255)
SRC_FG = (140, 140, 148)
DIVIDER = (70, 70, 78)
FOOTER_FG = (120, 120, 128)

FONT_CANDIDATES = [
    r"C:\Windows\Fonts\segoeuib.ttf",
    r"C:\Windows\Fonts\arialbd.ttf",
]
CAPTION_FONT_CANDIDATES = [
    r"C:\Windows\Fonts\consola.ttf",
    r"C:\Windows\Fonts\segoeui.ttf",
]


class CommitResolutionError(Exception):
    pass


def load_font(candidates, size):
    for path in candidates:
        if Path(path).exists():
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def wrap_text(draw, text, font, max_width):
    words = text.split()
    lines, cur = [], ""
    for w in words:
        trial = (cur + " " + w).strip()
        if draw.textlength(trial, font=font) <= max_width:
            cur = trial
        else:
            if cur:
                lines.append(cur)
            cur = w
    if cur:
        lines.append(cur)
    return lines[:CAPTION_WRAP_LINES]


def thumb(path, width):
    im = Image.open(path).convert("RGB")
    w, h = im.size
    new_h = max(1, round(h * (width / w)))
    return im.resize((width, new_h), Image.LANCZOS)


def _git(repo_dir, *args):
    return subprocess.run(
        ["git", "-C", str(repo_dir), *args],
        capture_output=True, text=True,
    )


def resolve_commit(repo_dir, commit_file=None, explicit_hash=None):
    """Return {"hash": short_hash, "author": name} or None. Raises CommitResolutionError
    on anything that would otherwise put a fake/unverifiable hash on the sheet."""
    if explicit_hash:
        chk = _git(repo_dir, "cat-file", "-e", f"{explicit_hash}^{{commit}}")
        if chk.returncode != 0:
            raise CommitResolutionError(
                f"hash '{explicit_hash}' does not exist in repo '{repo_dir}' "
                f"(git cat-file -e failed): {chk.stderr.strip()}"
            )
        short = _git(repo_dir, "rev-parse", "--short", explicit_hash)
        hash_to_use = short.stdout.strip() or explicit_hash
    elif commit_file:
        log = _git(repo_dir, "log", "-1", "--format=%h", "--", commit_file)
        hash_to_use = log.stdout.strip()
        if not hash_to_use:
            raise CommitResolutionError(
                f"no commit touches '{commit_file}' in repo '{repo_dir}' "
                f"(untracked, no history, or wrong path)"
            )
    else:
        return None

    author = _git(repo_dir, "log", "-1", "--format=%an", hash_to_use).stdout.strip()
    return {"hash": hash_to_use, "author": author}


def build_caption(resolved, note):
    if resolved:
        tag = f"{resolved['hash']} {resolved['author']}"
        return (tag + ("  \u2014 " + note if note else "")), True
    return note, False


def render_pair(before, after):
    """Return (image, block_height) for one before/after row."""
    label_font = load_font(FONT_CANDIDATES, 20)
    cap_font = load_font(CAPTION_FONT_CANDIDATES, 15)
    src_font = load_font(CAPTION_FONT_CANDIDATES, 12)

    b_img = thumb(before["path"], THUMB_W)
    a_img = thumb(after["path"], THUMB_W)
    img_h = max(b_img.height, a_img.height)

    tmp = Image.new("RGB", (10, 10))
    tmp_draw = ImageDraw.Draw(tmp)
    b_cap_lines = wrap_text(tmp_draw, before.get("caption", ""), cap_font, THUMB_W - 16)
    a_cap_lines = wrap_text(tmp_draw, after.get("caption", ""), cap_font, THUMB_W - 16)
    cap_lines = max(len(b_cap_lines), len(a_cap_lines), 1)
    cap_block_h = cap_lines * (CAPTION_H - 6) + 10 + SRC_LINE_H

    block_h = img_h + LABEL_H + cap_block_h
    block_w = THUMB_W * 2 + GAP

    canvas = Image.new("RGB", (block_w, block_h), BG)
    draw = ImageDraw.Draw(canvas)

    for i, (im, item, cap_lines_i) in enumerate(
        [(b_img, before, b_cap_lines), (a_img, after, a_cap_lines)]
    ):
        x = i * (THUMB_W + GAP)
        canvas.paste(im, (x, 0))
        draw.rectangle([x, img_h, x + THUMB_W, img_h + LABEL_H], fill=LABEL_BG)
        label = item.get("label", "")
        draw.text((x + 10, img_h + 6), label, font=label_font, fill=LABEL_FG)
        cy = img_h + LABEL_H + 6
        cap_color = COMMIT_FG if item.get("caption_verified") else CAPTION_FG
        for line in cap_lines_i:
            draw.text((x + 8, cy), line, font=cap_font, fill=cap_color)
            cy += CAPTION_H - 6
        src_name = Path(item["path"]).name
        draw.text((x + 8, cy + 4), f"src: {src_name}", font=src_font, fill=SRC_FG)

    return canvas, block_h


def build_sheet(pairs, out_path, title=None, footer=None):
    title_font = load_font(FONT_CANDIDATES, 26)
    footer_font = load_font(CAPTION_FONT_CANDIDATES, 13)
    blocks = [render_pair(p["before"], p["after"]) for p in pairs]
    block_w = THUMB_W * 2 + GAP

    title_h = 44 if title else 0
    footer_h = FOOTER_H if footer else 0
    total_h = (
        MARGIN * 2 + title_h + footer_h
        + sum(h for _, h in blocks) + ROW_GAP * (len(blocks) - 1)
    )
    total_w = block_w + MARGIN * 2

    sheet = Image.new("RGB", (total_w, total_h), BG)
    draw = ImageDraw.Draw(sheet)

    y = MARGIN
    if title:
        draw.text((MARGIN, y), title, font=title_font, fill=(255, 255, 255))
        y += title_h

    for canvas, h in blocks:
        sheet.paste(canvas, (MARGIN, y))
        y += h
        if y < total_h - MARGIN - footer_h:
            draw.line([(MARGIN, y + ROW_GAP // 2), (total_w - MARGIN, y + ROW_GAP // 2)], fill=DIVIDER, width=1)
        y += ROW_GAP

    if footer:
        draw.text((MARGIN, total_h - MARGIN - footer_h + 8), footer, font=footer_font, fill=FOOTER_FG)

    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path)
    return out_path


def parse_args():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", required=True, help="output PNG path")
    ap.add_argument("--title", default=None, help="optional title text at top of sheet")
    ap.add_argument("--repo", default=None, help="git repo dir for commit resolution (default: this script's repo root)")
    ap.add_argument("--manifest", help="JSON file with a list of {before_*, after_*} rows")
    ap.add_argument("--before", help="single-pair mode: before image path")
    ap.add_argument("--after", help="single-pair mode: after image path")
    ap.add_argument("--before-label", default="BEFORE")
    ap.add_argument("--after-label", default="AFTER")
    ap.add_argument("--before-caption", default="", help="free-text note (no commit hash typed here)")
    ap.add_argument("--after-caption", default="", help="free-text note (no commit hash typed here)")
    ap.add_argument("--before-file", default=None, help="tracked file to resolve BEFORE's commit from (git log -1)")
    ap.add_argument("--after-file", default=None, help="tracked file to resolve AFTER's commit from (git log -1)")
    ap.add_argument("--before-hash", default=None, help="explicit BEFORE commit hash (verified via git cat-file -e)")
    ap.add_argument("--after-hash", default=None, help="explicit AFTER commit hash (verified via git cat-file -e)")
    return ap.parse_args()


def resolve_side(repo_dir, path, label, note, commit_file, explicit_hash, side_name):
    try:
        resolved = resolve_commit(repo_dir, commit_file=commit_file, explicit_hash=explicit_hash)
    except CommitResolutionError as e:
        print(f"error: {side_name}: {e}", file=sys.stderr)
        sys.exit(2)
    if resolved is None and (commit_file is None and explicit_hash is None):
        print(f"warning: {side_name}: no --{side_name}-file/--{side_name}-hash given, "
              f"caption has no verified commit tag", file=sys.stderr)
    caption, verified = build_caption(resolved, note)
    return {"path": path, "label": label, "caption": caption, "caption_verified": verified}


def main():
    args = parse_args()
    repo_dir = Path(args.repo).resolve() if args.repo else Path(__file__).resolve().parents[1]
    if _git(repo_dir, "rev-parse", "--git-dir").returncode != 0:
        print(f"error: '{repo_dir}' is not a git repo (needed to verify commit hashes)", file=sys.stderr)
        sys.exit(2)

    pairs = []
    if args.manifest:
        rows = json.loads(Path(args.manifest).read_text(encoding="utf-8"))
        for r in rows:
            before = resolve_side(
                repo_dir, r["before_path"], r.get("before_label", "BEFORE"), r.get("before_caption", ""),
                r.get("before_file"), r.get("before_hash"), "before",
            )
            after = resolve_side(
                repo_dir, r["after_path"], r.get("after_label", "AFTER"), r.get("after_caption", ""),
                r.get("after_file"), r.get("after_hash"), "after",
            )
            pairs.append({"before": before, "after": after})
    elif args.before and args.after:
        before = resolve_side(
            repo_dir, args.before, args.before_label, args.before_caption,
            args.before_file, args.before_hash, "before",
        )
        after = resolve_side(
            repo_dir, args.after, args.after_label, args.after_caption,
            args.after_file, args.after_hash, "after",
        )
        pairs.append({"before": before, "after": after})
    else:
        print("error: pass either --manifest pairs.json or --before/--after for a single pair", file=sys.stderr)
        sys.exit(2)

    for p in pairs:
        for side in ("before", "after"):
            if not Path(p[side]["path"]).exists():
                print(f"error: {side} image not found: {p[side]['path']}", file=sys.stderr)
                sys.exit(2)

    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    footer = f"generated {timestamp}  (commits resolved/verified against {repo_dir})"
    out_path = build_sheet(pairs, args.out, title=args.title, footer=footer)
    print(str(out_path.resolve()))


if __name__ == "__main__":
    main()
