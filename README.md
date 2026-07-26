# Voxelforge

A voxel game built on **Rust + Bevy 0.19 + wgpu**, aiming for an AAA-grade
lit look while keeping the hard-edged voxel identity. The repo is a Cargo
workspace with three crates plus a full render/grade toolchain for the
"beauty shot" the art direction is graded against.

## Workspace layout

| Crate / dir | What it is |
|--|--|
| `sim/` | Pure voxel simulation — chunks, blocks, worldgen (no rendering) |
| `client/` | Bevy app: greedy-meshed chunks, the full post stack, fly camera. Two binaries — `voxelforge` (the game) and `voxelforge_shot` (isolated hero-shot renderer) |
| `server/` | Headless authoritative world + netcode |
| `scripts/` | Render drivers (`render_*.sh`) and pixel-measuring graders (`grade_*.py`) |
| `maps/` | Hand-authored JSON maps (see `maps/FORMAT.md`) |
| `tools/mapgen/` | Map generator |
| `docs/` | Look Bible, acceptance rubric, golden reference, architecture notes |

> `src/` at the repo root is the legacy Phase-0 single-crate spike, kept for
> history. The live client is `client/`.

## Build

```bash
cargo build --release                    # whole workspace
cargo build --release --bin voxelforge   # just the game client
cargo build --release --bin voxelforge_shot  # just the hero-shot renderer
```

## Run — native game

```bash
cargo run --release --bin voxelforge          # free fly, FPS overlay
VOXELFORGE_GRID=8 cargo run --release --bin voxelforge   # 8×8 chunks
VOXELFORGE_BENCH=1 cargo run --release --bin voxelforge  # ramp benchmark, prints BENCH_RESULT
```

Controls: **click** = capture mouse / look · **WASD** = move · **Space/Shift**
= up/down · **Ctrl** = boost · **Esc** = release mouse.

## Run — web (WebGPU)

```bash
trunk serve --release    # builds the `webgpu` feature, serves on 127.0.0.1:8088
node scripts/measure-web.mjs http://127.0.0.1:8088 web-shot.png   # headless FPS read
```

Requires a WebGPU-capable Chrome (stable ≥ 113 on a real GPU).

## Render a beauty shot

`voxelforge_shot` renders the lit hero scene only (no game loop), so the
golden shot can be re-rendered while the game code is mid-edit. Everything is
driven by env vars — no recompile needed to retune the grade:

```bash
# render the hero shot to a PNG and exit
VOXELFORGE_SHOT=shot.png ./target/release/voxelforge_shot

# wide establishing angle + grade overrides (example, see scripts/render_wide_shot.sh)
VOXELFORGE_WIDE=1 \
VOXELFORGE_CAM=9.0,6.2,-6.5,6.8,2.6,8.0,58 \
VOXELFORGE_EXPOSURE=8.75 VOXELFORGE_BLUESCALE=0.67 \
VOXELFORGE_GRADE=-0.01,0.88,1.10 \
VOXELFORGE_SHOT=wide-A.png ./target/release/voxelforge_shot
```

Key env knobs read by the renderer: `VOXELFORGE_SHOT` (output path),
`VOXELFORGE_CAM` (eye+target+fov), `VOXELFORGE_DOF`, `VOXELFORGE_EXPOSURE`,
`VOXELFORGE_BLUESCALE`, `VOXELFORGE_GRADE` (`temp,sat,contrast`),
`VOXELFORGE_AMBCOLOR`, `VOXELFORGE_AMBIENT`, `VOXELFORGE_WIDE`. The ready-made
drivers in `scripts/` (e.g. `render_wide_shot.sh`, `render_hero_final.sh`)
wrap these.

## Look Bible & golden reference

The art target lives in `docs/`:

- **`docs/look-bible.md`** — the look direction (9-pass render intent) + `docs/assets/moodboard.png`
- **`docs/golden-beauty-shot.md`** + **`docs/assets/golden-beauty-shot-ref.png`** — the canonical golden the graders measure against
- **`docs/look-acceptance-rubric.md`** — the full acceptance rubric (gate + AAA score)
- `hero-golden-a/b/c.png` (repo root) — the honey-tone hero targets the wide-shot compares track

## Acceptance gate (G1–G6)

Every candidate frame must pass **all six** binary gates — failing any one
fails the whole frame, regardless of the 100-pt AAA score. Full detail +
calibration log in `docs/look-acceptance-rubric.md`.

| Gate | Checks | Pass condition |
|--|--|--|
| **G1** | Voxel hard-edge geometry | Object edges stay 90° cube-blocks — not beveled/rounded away |
| **G2** | Directional key light (pass 1) | One clear sun direction + a window-light bar cast on floor/wall |
| **G3** | Bounce/shade not black, not blue (pass 2) | Interior `p05` luminance ≥ 8%, darkest shade still reads as wood, warm (R ≥ G ≥ B) |
| **G4** | Soft shadow + contact AO (pass 4) | Cast-shadow edge transitions ≥ 3px (not 1px hard) + contact AO under objects |
| **G5** | Tone-map not blown out (pass 6) | Brightest window px has G or B ≤ 245 (not flat 255,255,255) + 3-pt vertical gradient survives |
| **G6** | Warm golden tone (pass 1+9) | Sunlit wood patch reads R > G > B with R−B in 40–210 |

Machine-graded axes (a pre-filter for the gate) run via:

```bash
python scripts/grade_axes.py <frame.png>   # per-axis PASS/FAIL vs golden ref
python scripts/grade_gate.py <frame.png>    # measurable gates G3/G5/G6
```

## Known spike-level shortcuts

- Merged greedy quads sample a single atlas tile stretched across the quad
  (the classic greedy-mesh/atlas tension). Production fix: a texture array or
  per-voxel tile indices.
- One chunk = one draw call — no frustum-culling tuning, LOD, or async meshing yet.
