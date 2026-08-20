# Worktree snapshot — 2026-08-20

Safety net for **uncommitted work** ("floating work") in the 9 dirty trees of the
Voxelforge repo. All worktree HEADs are already reachable from origin (verified
per tree below) — this snapshot preserves only what was NOT committed: tracked
modifications plus untracked files, as one `git diff HEAD --binary` patch per
tree (untracked files included via `git add -N` before the diff; the
intent-to-add entries were removed afterwards and every tree's `git status`
was verified byte-identical to before).

**Recovery source of truth:** branch `shino/worktree-snapshot-2026-08-20` on
`origin` (github.com/bagidea/voxelforge). This on-disk folder is covered by the
repo's blanket `_*/` ignore rule, so it will never show up in `git status` —
the pushed branch is the copy that matters.

## How to restore a tree

A patch applies **only to the worktree it came from, checked out at the HEAD
recorded below** (the preimage blobs live in the shared object store):

```bash
git fetch origin
git -C <worktree-path> checkout --detach <HEAD-hash>   # already there in practice
git -C <worktree-path> apply --binary <worktree-path>/<name>.patch
```

Get the patch out of the branch (from any clone of the repo):

```bash
git show origin/shino/worktree-snapshot-2026-08-20:_worktree_snapshot_2026-08-20/<name>.patch > <name>.patch
```

## Per-tree map

| # | tree | path | HEAD | branch | covered on origin by | uncommitted | patch | bytes |
|---|------|------|------|--------|----------------------|-------------|-------|-------|
| 1 | main tree | `E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge` | `676e2c51a9a2e4df498d1b0c812112bcbb34482e` | poppy/native-only | origin/poppy/native-only | 5 M + 39 untracked (44 files) | `main-tree.patch` | 1,778,061 |
| 2 | _sun-quest-wt | `E:/Projects/bagidea-ai-agents-office/workspace/projects/_sun-quest-wt` | `b5dea479873955a38eb8e0a8eb33207bb4665d65` | detached | origin/poppy/native-only, origin/yamamoto/audio-close-out | 2 M | `_sun-quest-wt.patch` | 8,516 |
| 3 | _vf_poppy_8ba5ede | `E:/Projects/bagidea-ai-agents-office/workspace/projects/_vf_poppy_8ba5ede` | `8ba5ede3f5921cef4e414dca72fb4858dc3a3c18` | detached | origin/poppy/native-only, origin/yamamoto/audio-close-out | 1 M | `_vf_poppy_8ba5ede.patch` | 3,068 |
| 4 | _vf_poppy_dome | `E:/Projects/bagidea-ai-agents-office/workspace/projects/_vf_poppy_dome` | `bbd662414635da818aec508c68b88227f6d0fef1` | detached | origin/poppy/native-only, origin/yamamoto/audio-close-out | 1 M | `_vf_poppy_dome.patch` | 1,736 |
| 5 | _vf_poppy_head | `E:/Projects/bagidea-ai-agents-office/workspace/projects/_vf_poppy_head` | `eff6c7be88b662cc8611a363d7a3f4a90c79ec36` | detached | origin/poppy/native-only, origin/yamamoto/audio-close-out | 4 M + 3 untracked (7 files) | `_vf_poppy_head.patch` | 27,980 |
| 6 | _yamamoto_combat_proof | `E:/Projects/bagidea-ai-agents-office/workspace/projects/_yamamoto_combat_proof` | `197e5e5877c55fb984c188cf66360b245110abbc` | yamamoto/audio-close-out | origin/yamamoto/audio-close-out | 1 untracked | `_yamamoto_combat_proof.patch` | 2,968 |
| 7 | _fl_char_wt | `E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge/_fl_char_wt` | `be8333ab9eddf619883b5359c399a3f699d085c9` | detached | origin/poppy/native-only, origin/yamamoto/audio-close-out | 1 M | `_fl_char_wt.patch` | 2,053 |
| 8 | _pixel_recover_wt | `E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge/_pixel_recover_wt` | `2262f71333ce20a45d9358e1384a35d5f33f6679` | detached | origin/poppy/native-only | 1 M | `_pixel_recover_wt.patch` | 36,421 |
| 9 | _poppy_head_wt | `E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge/_poppy_head_wt` | `370c4a32ad4c5bffd98f1fc4554d8639abeac639` | detached | origin/poppy/native-only, origin/yamamoto/audio-close-out | 2 M | `_poppy_head_wt.patch` | 36,926 |

Total: 1,897,729 bytes across 9 patches. All 9 verified: size > 0, file set
identical to the tree's `git status` at capture time, and `git apply --stat`
parses each one (insertions/deletions in the table below). The main tree was
re-shot once mid-run because lanes kept writing (5 new scripts appeared); work
that appears after the last re-shoot is not covered — refresh the branch if
that matters.

## Uncommitted file lists (as captured)

### 1. main tree — 5 modified + 39 untracked (patch: +4023/−6, 8 binary blobs)

Modified:
- `assets/textures/blocks/brick.png`
- `assets/textures/blocks/brick_n.png`
- `assets/textures/blocks/brick_r.png`
- `client/src/water.rs`
- `scripts/contact_sheet.py`

Untracked:
- `_poppy_sky_build.cmd.UNGUARDED-DO-NOT-RUN`
- `_sun_build2.status`
- `client/build1.txt`, `client/check1.txt`, `client/simtest.txt`
- `docs/VERDICT-sun-skyground-ab-2026-08-20.md`
- `docs/_kevin_foliage_wire.md`
- `docs/brick-texture-before-after-2026-08-20.png`
- `docs/evidence/dirty-flag-fix-2026-08-20/` (dir, contents in patch)
- `docs/interior-blocks-contact-sheet-after-2026-08-20.png`
- `scripts/_kevin_foliage_presence.sh`, `scripts/_kevin_foliage_windphase.sh`, `scripts/_kevin_ingame_ab.sh`, `scripts/_kevin_ingame_measure.py`, `scripts/_kevin_ingame_shoot.sh`, `scripts/_kevin_sway_measure.py`
- `scripts/_monanisa_brick_regrain.py`, `scripts/_monanisa_texture_audit.py`
- `scripts/_pixel_diag_shoot.ps1`, `scripts/_pixel_hero_ladder.ps1`, `scripts/_pixel_hero_shoot.ps1`, `scripts/_pixel_sky_ladder.ps1`
- `scripts/_pixel_final_shoot.ps1`, `scripts/_pixel_hero_ladder2.ps1`, `scripts/_pixel_measure.py`, `scripts/_pixel_sky_ladder2.ps1` (appeared mid-run, caught by the re-shoot)
- `scripts/_poppy_outdoor_plates.ps1`, `scripts/_poppy_skyab2_build.cmd`, `scripts/_poppy_skyab2_shoot.ps1`, `scripts/_poppy_skyab_shoot.ps1`, `scripts/_poppy_skyafter_build.cmd`, `scripts/_poppy_wait_build_slot.ps1`
- `scripts/_shino_recount_linear.ps1`, `scripts/_shino_verify_skymask.py` (appeared mid-run, caught by the re-shoot)
- `scripts/_sun_gate_build.ps1`, `scripts/_sun_skyground_measure.py`, `scripts/_sun_skyground_shoot.ps1`

### 2. _sun-quest-wt — 2 modified (patch: +38/−13)
- `client/src/quest.rs` (Act-1-end capture hook, `VOXELFORGE_ACT1_SHOT`)
- `client/src/quest_chaos.rs`

### 3. _vf_poppy_8ba5ede — 1 modified (patch: +48/−1)
- `client/src/main.rs`

### 4. _vf_poppy_dome — 1 modified (patch: +9/−4)
- `client/src/hud.rs`

### 5. _vf_poppy_head — 4 modified + 3 untracked (patch: +90/−14, 3 binary blobs)
- `client/build.rs`, `client/src/anim.rs`, `client/src/main.rs`, `client/src/quest.rs`
- `assets/textures/block_atlas_v1_procedural_BACKUP.png` (untracked)
- `assets/textures/block_atlas_v2.png` (untracked)
- `assets/textures/block_atlas_v2_bonus_glass_lamp.png` (untracked)

### 6. _yamamoto_combat_proof — 1 untracked (patch: +69)
- `sim/examples/probe_surface.rs`

### 7. _fl_char_wt — 1 modified (patch: +24/−4)
- `client/src/scene.rs`

### 8. _pixel_recover_wt — 1 modified (patch: +440/−62)
- `client/src/anim.rs`

### 9. _poppy_head_wt — 2 modified (patch: +596/−16)
- `client/src/voxel.rs`
- `sim/src/block.rs`

## Clean trees (nothing to snapshot, verified 0 dirty entries)

- `_flamingo_beauty_wt` @ `97cb283` (flamingo/beauty-axes)
- `_poppy_sky_wt` @ `e4ceb80` (detached)
- `_poppy_skyab_wt` @ `676e2c5` (detached)

## Method & guarantees

- Patch = `git diff HEAD --binary` per tree, so tracked edits + untracked files
  (incl. binary PNGs as GIT binary patches) are all in one file.
- Trees with untracked files went through `git add -N` → diff → `git reset`
  of exactly those paths; `git status --porcelain` was captured before and
  after and verified identical — **no source tree was left modified, staged,
  committed, stashed, checked out, or cleaned by this snapshot.**
- `client/src/water.rs` and `client/src/look.rs` were only ever read.
- Snapshot branch created with plumbing (`hash-object`/`mktree`/`commit-tree`
  /`update-ref`), parent = main-tree HEAD `676e2c5` — the main tree's index,
  branch, and working files were never touched by the commit.
