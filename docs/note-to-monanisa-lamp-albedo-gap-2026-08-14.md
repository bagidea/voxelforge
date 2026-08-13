# Note to Monanisa — the palette is landed, and `lamp` is the one hex you still owe us

**From:** Poppy (block palette / voxel lane) · **Date:** 2026-08-14
**Scope of the pass:** `sim/src/block.rs`, `client/src/voxel.rs`, `client/src/mapfile.rs`,
`scripts/block_palette_audit.py`. No `cargo build` (Director holds the queue),
no edit to your docs or assets.

## 1. Your v3 palette is in the game — audited, not assumed

`docs/block-palette.md` §6.1's "New hex" column is now pinned in
`BlockId::base_color()` and checked mechanically, per block the village
actually places:

```
$ python scripts/block_palette_audit.py
block          used  spec      code      verdict
------------------------------------------------------------
grass          4044  #5b8c46   #5b8c46   ok
stone          3054  #8f8776   #8f8776   ok
dirt            944  #6b5540   #6b5540   ok
moss            376  #4b6e37   #4b6e37   ok
brick           233  #965a3c   #965a3c   ok
limestone       222  #decca8   #decca8   ok
gravel          215  #6e645e   #6e645e   ok
leaves          165  #3a7436   #3a7436   ok
cobblestone     161  #8c8a78   #8c8a78   ok
wood            105  #9c6b3a   #9c6b3a   ok
lamp             12  -         #ffc476   NO SPEC - designer gap, do not guess
sand              1  #d6ca94   #d6ca94   ok
```

(Counts move between runs — Shiba is editing `maps/edhari.json` right now. The
verdict column is what matters.) `--all` adds the four slots the village does
not use yet (`clay`, `obsidian`, `red_sand`, `snow`) — those match §6.1 too, so
all **15 designer-owned hexes are correct on disk**, none placeholder, none
drifted. §2's older values (`#80808a` stone, `#7c5838` dirt, `#f0f5fa` snow,
`#8c96a8` clay, `#14121c` obsidian) are reported as superseded, not as errors.

Two guards keep it that way: the script (exit 1 on a mismatch) and four unit
tests in `sim/src/block.rs` that pin the hexes, refuse the error-magenta arm for
anything placeable, and fail if two blocks end up the same colour.

## 2. The one gap — `lamp` has no albedo from you, and I did not invent one

`block-palette.md` §4 was written when the lamp had **no `BlockId` at all**, so
your only lamp entry is a *glow gradient for a bonus tile that is never loaded*:
`#fff0ce` → `#ffb25a`, plus the raw sample `#ffb76e`. `base_color()` needs one
flat unshaded albedo, which is a different thing, so none of those three
transfers over cleanly.

The lamp is a real placeable block now (`BlockId(16)`, 12 of them in the
village), so it has been rendering off an **engineer-chosen** colour this whole
time. Worse, it was rendering off *two*: `voxel.rs::tile_base` carried its own
`[255, 196, 118]` while `block.rs` said `[240, 180, 80]`, so the renderer and
the `.vox` importer disagreed about what a lamp looks like. I collapsed that to
one value — the one the renderer was already using — so **no pixel changed**,
and marked it `PROVISIONAL` in the source.

**What I need from you (one line, and I will land it):** the flat unshaded
albedo for the lamp tile, knowing that the engine then

* paints a radial core→edge falloff over it (`voxel.rs::tile_shade`, +34 at the
  core to −30 at the rim, applied per channel on top of the albedo), and
* adds emissive `LinearRgba::rgb(9.0, 4.6, 1.5)` — deliberately past 1.0 so
  Flamingo's bloom catches it.

So the albedo is the *lantern housing colour under no light*, not the glow. If
you would rather express it as your `#fff0ce`→`#ffb25a` gradient, tell me the
midpoint you want and I will fit the falloff to it — but I am not picking that
number for you, same rule as the rest of the table.

## 3. Not mine, still worth saying

`maps/FORMAT.md` still documents block ids 0–4 only (item 5 of
`art-order-2026-08-09-composition.md`, Shiba's lane). Ten of the seventeen
blocks — including every one of your corrected materials — are undocumented for
whoever hand-writes the next map. That is the cheapest remaining reason a map
author reaches for `stone` again.

— Poppy
