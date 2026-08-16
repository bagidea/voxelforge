#!/usr/bin/env python3
"""Compose the character/equipment contact sheets from raw charshot plates.

Two sheets come out of here:

  * a one-row sheet of the four gear sets (bare / villager / adventurer /
    warplate), which is the "does the gear go on and does it read" deliverable;
  * a two-row before/after sheet, top = ``VOXELFORGE_SCULPT=0`` plates, bottom =
    the sculpt pass, at the same camera from the same binary.

Rules this script follows, all of them scars from earlier passes in this repo:

  * **Every caption is checked against the file it names.** A panel is only
    labelled after its source path has been read and its size recorded, and the
    manifest printed at the end lists path + size + mean luma per panel. A sheet
    whose caption does not match its pixels is worse than no sheet.
  * **No resizing between the two rows.** If a before plate and an after plate
    are not the same size the script refuses, rather than silently scaling one
    and turning a framing bug into a "look how much better" artefact.
  * **It fails loudly on a missing plate** instead of quietly emitting a shorter
    sheet, because a 3-panel sheet still looks complete.
"""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

PAD = 18
LABEL_H = 62
TITLE_H = 74
BG = (16, 14, 12)
FG = (238, 232, 220)
DIM = (168, 158, 146)


def font(size: int):
    for name in ("segoeuib.ttf", "arialbd.ttf", "segoeui.ttf", "arial.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def load(path: Path) -> Image.Image:
    if not path.exists():
        sys.exit(f"MISSING PLATE: {path} — refusing to build a short sheet")
    return Image.open(path).convert("RGB")


def mean_luma(im: Image.Image) -> float:
    g = im.convert("L")
    return sum(g.getdata()) / (g.width * g.height)


def sheet(rows, title: str, out: Path) -> None:
    """rows = [(row_caption, [(panel_caption, Path), ...]), ...]"""
    loaded = [(cap, [(c, p, load(p)) for c, p in panels]) for cap, panels in rows]

    sizes = {im.size for _, panels in loaded for _, _, im in panels}
    if len(sizes) != 1:
        sys.exit(f"PLATE SIZE MISMATCH {sizes} — refusing to scale between panels")
    w, h = sizes.pop()
    cols = max(len(panels) for _, panels in loaded)

    row_h = h + LABEL_H
    W = PAD + cols * (w + PAD)
    H = TITLE_H + len(loaded) * (row_h + PAD)
    sheet_im = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(sheet_im)
    d.text((PAD, 22), title, font=font(34), fill=FG)

    manifest = []
    y = TITLE_H
    for row_cap, panels in loaded:
        for i, (cap, path, im) in enumerate(panels):
            x = PAD + i * (w + PAD)
            sheet_im.paste(im, (x, y))
            d.rectangle([x - 1, y - 1, x + w, y + h], outline=(52, 46, 40))
            d.text((x + 6, y + h + 8), cap, font=font(26), fill=FG)
            d.text((x + 6, y + h + 36), path.name, font=font(17), fill=DIM)
            manifest.append((row_cap, cap, path, im.size, mean_luma(im)))
        if row_cap:
            d.text(
                (W - PAD - d.textlength(row_cap, font=font(24)), y + h + 10),
                row_cap,
                font=font(24),
                fill=DIM,
            )
        y += row_h + PAD

    out.parent.mkdir(parents=True, exist_ok=True)
    sheet_im.save(out)
    print(f"SHEET {out}  {sheet_im.width}x{sheet_im.height}")
    for row_cap, cap, path, size, luma in manifest:
        print(f"  PANEL row={row_cap or '-':10s} cap={cap:14s} {size[0]}x{size[1]} luma={luma:6.2f}  {path}")


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    out_dir = root / "docs" / "assets" / "characters"
    sets = ["bare", "villager", "adventurer", "warplate"]
    caps = {
        "bare": "1 · bare body",
        "villager": "2 · villager set",
        "adventurer": "3 · adventurer set",
        "warplate": "4 · warplate + hammer",
    }

    after = [(caps[s], out_dir / f"gear-ladder-b-after-{i + 1}-{s}.png") for i, s in enumerate(sets)]
    before = [(caps[s], out_dir / f"gear-ladder-a-before-{i + 1}-{s}.png") for i, s in enumerate(sets)]

    sheet(
        [("", after)],
        "Voxelforge — head / torso / legs / hands / weapon / back on ONE entity, one camera, one sun",
        out_dir / "gear-ladder-sheet.png",
    )
    sheet(
        [("BEFORE — VOXELFORGE_SCULPT=0", before), ("AFTER — sculpt pass", after)],
        "Voxelforge — sculpt pass before/after · same binary, same camera, same sun",
        out_dir / "gear-ladder-before-after.png",
    )


if __name__ == "__main__":
    main()
