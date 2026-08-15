# AI-proof evidence stills — v2 (re-plated after review)

**Owner:** rose · **Date:** 2026-08-16 · **Set:** `docs/assets/ai/` (14 plates + 7 paircheck sheets + `enemy-chase.mp4` + `trace.csv` + `runlog.txt`)

## 1. Why v2 exists — the review verdict on v1

An independent review of the v1 stills found three real defects. All three are acknowledged, not contested:

1. **Every v1 before/after pair was 2–4 frames apart** (swarm f0090↔f0092, pouncer f0158↔f0160, bruiser f0408↔f0424, and all 8 behaviour plates the same pattern). Measured pixel difference was 0.6–2.5% — the pairs did **not** show behaviour to the eye; only `trace.csv` carried the proof.
2. **`ai-arch-swarm-a-before.png` was byte-identical to `ai-a-before-chase.png`** (md5 `ba37152f…`) — one frame wearing two captions, undisclosed.
3. **The v1 report overclaimed** ("pounce shows clear displacement", "bruiser changes pose") — true only in the mp4, not in the delivered stills.

What v1 did have (confirmed by the same review): real build, real 1321-frame run, real trace with all archetype cycles, real 21s mp4 with motion, honest gamma-lift disclosure.

## 2. How v2 picks pairs (reproducible, not hand-waved)

- Frames are chosen by `scripts/_rose_ai_pairsearch.py` / `_rose_ai_pairsearch2.py` under **trace-state constraints**: the caption's named enemy must be in the caption's state at that frame, and the pair must be the strongest diff that satisfies the semantics (e.g. pouncer A = a stalk frame with *no* enemy point-blank so the stalker isn't upstaged; strike B = an actual `strike` frame).
- Wall-clock separation comes from `runlog.txt`'s own screenshot timestamps (the run's 1258 captured frames span 84.4s of real time — ~67 ms/frame), not from an assumed fps.
- Plates are rebuilt from the untouched raw `_frames/` with the **same gamma-0.32 lift** as v1 (`255·(v/255)^0.32`, verified by re-fitting the exponent against v1's lifted frame: median 0.322).
- Regenerate everything: `python scripts/_rose_ai_replate.py --dir docs/assets/ai` — it also prints the diff table below and **gates on md5 uniqueness of all 14 plates** (the exact v1 bug is now a hard FAIL).

## 3. The 14 plates, mapped to trace truth

| pair | A (state, dist) | B (state, dist) | wall Δt | %px \|d\|>8 | %px \|d\|>24 |
|---|---|---|---|---|---|
| `ai-arch-swarm-*` | f0060 · reaver×2 **patrol** 16.07/15.49 | f0284 · **strike** 0.85 + recover 0.61 (both on player) | 15.1s | 40.6% | 37.4% |
| `ai-arch-bruiser-*` | f0082 · sentinel **patrol** 13.40 | f0424 · **strike** 2.30 (reavers on player 0.6–1.0) | 22.0s | 41.4% | 38.9% |
| `ai-arch-pouncer-*` | f0104 · stalker **stalk** 9.33 (all four mid-frame, none point-blank) | f0944 · **pounce** 0.90 | 65.7s | 41.1% | 37.3% |
| `ai-*-detect` | f0086 · reaver#2 **alert** 11.14 | f0116 · reaver#2 **chase** 4.45 (sentinel alerts 12.1→8.4 same window) | 3.6s | 7.4% | 6.4% |
| `ai-*-chase` | f0180 · reaver#1 **chase** 11.59 | f0274 · **chase** 2.26 (reaver#2 strikes 0.92) | 5.3s | 11.7% | 11.0% |
| `ai-*-strike` | f0114 · reaver#2 **chase** 4.45 | f0130 · **strike** 0.45 | 2.6s | 13.9% | 10.2% |
| `ai-*-retreat` | f0148 · reaver#2 **strike** 1.05 | f0160 · **recover** 2.82 | 0.7s | 16.9% | 15.7% |

Every row can be re-derived: `trace.csv` row `(frame, enemy)` must show the named state at the named dist. `_paircheck_<pair>.png` stacks each A/B with its caption for eyeball verification.

## 4. What the eye actually sees (checked, conservatively stated)

Blind description of the paircheck sheets (no leading prompts) confirmed, per pair: swarm/bruiser/chase — scene composition changes, the subject grows from a far silhouette to dominant-in-frame; pouncer — the central creature goes upright → crouched/pounce pose; retreat — the centre creature moves away and opens empty space mid-frame; strike — creature count and positions change markedly; detect — the leftmost far creature becomes markedly closer (the weakest pair, see §5).

## 5. Honesty notes / known limits

- **detect is the weakest pair (7.4%)** and is labelled as such: the patrol→alert beat spans ~0.5s and a few world units in this scenario, so a *quiet-scene-only* detect pair tops out at ~3%. v2 instead shows the detect *response* (alert → committed chase of the same enemy), which is what the pixels show. The alert beat itself is proven by `trace.csv` (per-enemy `alert` runs) and visible in the mp4.
- The stills prove **state contrast at trace-backed moments**. Motion continuity, cycle counts (swarm 14–16 chase→strike cycles, bruiser 5 advance→strike, pouncer 7 stalk→crouch→pounce) and telegraph locking remain proven by `trace.csv` + `runlog.txt` + `enemy-chase.mp4`, not by stills.
- Raw frames in `_frames/` are un-lifted and gitignored (local only); the lift applies to delivered plates/clip only, as disclosed in v1.
- Frames are 2 apart in numbering by capture cadence; all 14 plates sit on 14 **distinct** raw frames (script-gated).
