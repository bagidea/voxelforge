# Husk AI — BEFORE / AFTER, captured (Flamingo, 2026-08-14)

`docs/assets/ai/` was **empty**: `docs/enemy-behavior.md` §7 describes the proof
run, but no frame from it had ever been written to disk. This note is the run
that fills it, and the honest reading of what the resulting pictures do and do
not prove.

## What each side actually is

| | BEFORE | AFTER |
|---|---|---|
| behaviour | the **shipped** Guard Husk AI, `client/src/combat.rs` §4.1 | rose's new archetype AI, `client/src/enemy_ai.rs` |
| binary | `voxelforge` (the real game) | `voxelforge_enemyai_proof` |
| how it is driven | `VOXELFORGE_PLAY=1 VOXELFORGE_AI_DEMO=1` — `husk_ai_demo` moves only the *player* through the ranges that light each state; the husk AI itself is untouched | the bin's own scripted 4-phase route (stand → approach → flee → cornered) |
| cast | 3-husk squad (`spawn_ai_squad`, demo-only top-up) | Reaver ×2 (Swarm), Sentinel (Bruiser), Stalker (Pouncer) |
| stills | one per husk state, the first frame that state appears (`VOXELFORGE_AI_SHOTS`) | every 2nd frame (`VOXELFORGE_AIFRAMES`), beats then picked from the run's own `trace.csv` |
| numeric trace | `VOXELFORGE_AI_TRACE` → `t,entity,x,z,state,dist,evading` | `trace.csv` → `frame,enemy,state,archetype,x,z,dist` |

Both binaries are built into `target-flamingo/` for this run, so no capture can
come from a stale exe. Each sheet carries its own exe path + byte size +
mtime in the footer.

## The honest caveat, stated up front

The two sides are **not the same scene**. BEFORE stands in the real game world
(Edhari terrain, the shipped Guard Husk body); AFTER stands in the proof bin's
flat dark arena with Monanisa's three new bodies. So the stills differ in
*look* for reasons that have nothing to do with AI, and a reader who only
compares the two contact sheets will over-read the difference.

That is exactly why the third artifact exists: `husk-ai-pursuit-plot.png` draws
both runs' **positions** top-down from each run's own CSV, at a marked world
scale. Path shape is the one thing measured identically on both sides — it is
where the behaviour difference is real and not a lighting change.

## Method (reproducible)

```bash
# both steps are owned by one detached driver so a teardown cannot orphan them
bash scripts/_flamingo_ai_ba_driver.sh          # build → capture → build → capture → sheets
python scripts/_flamingo_ai_ba_sheet.py         # sheets only, from frames already on disk
```

The sheet script never draws a panel it does not have a real frame for: a beat
that never appears in the trace is printed as missing and dropped, and the
script exits **2** ("refused to measure") rather than 1 when a side has no
capture at all — the office's unmeasurable-vs-failed convention.

## Results

_(filled in below by the run — see the artifact list.)_
