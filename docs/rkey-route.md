# The R key has two owners — `RKeyRoute` decides which one eats the press

R is bound twice in Voxelforge:

| consumer | file | what R does |
|---|---|---|
| lock-on toggle (§2.3) | `client/src/combat.rs` → `gather_input` | locks the camera onto the nearest enemy / unlocks |
| `place_block` objective | `client/src/quest.rs` → `check_block_place_triggers` | places the quest block (q4 `o1_build`) |

Bevy's `ButtonInput::just_pressed` is a **query, not a consume**: the flag stays
`true` for every later reader until the next `PreUpdate`. So before the arbiter,
one press fired *both* — the build objective completed *and* the camera snapped
onto a husk in the same frame.

`combat::RKeyRoute` is the arbiter. This page is its contract.

## The contract

1. **One claim per frame.** `RKeyRoute::claim(want, who)` returns `true` to the
   first caller of a frame and `false` to everyone after it. A `false` means the
   caller must leave the key alone — not "try harder", not "do it anyway".
2. **Priority is system order, not an if-chain.** The contextual consumer runs
   first and only claims when it can really act; lock-on is the fallback and
   takes whatever is left. There is no priority number anywhere in the code:
   read the schedule, not a constant.
3. **Claim before you mutate.** `check_block_place_triggers` asks
   `place_block_in_reach()` first, then claims, and only then touches the
   journal (`quest.rs:1336-1344`). Claiming and *then* discovering you can't act
   would starve lock-on for that frame.
4. **The route is cleared every frame, unconditionally.** `reset_r_route` runs in
   `First` — ahead of Bevy's own input clearing in `PreUpdate` and of every
   `Update` reader — and is **not** state-gated: a stale claim left behind on the
   frame `Play` is entered or left would silently eat the next real press
   (`combat.rs:948-958`).

## Who claims what

```
RKeyUse::Unclaimed   nobody has taken the press yet this frame  (the default)
RKeyUse::QuestPlace  a place_block objective was in reach → the press builds
RKeyUse::LockOn      nothing contextual wanted it → the press toggles lock-on
```

Claim sites, in the order they run:

```
quest::check_block_place_triggers   claims QuestPlace   quest.rs:1340
combat::gather_input                claims LockOn       combat.rs:1440
```

`gather_input` reads the claim with `&&`, so the toggle only ever happens on a
frame the arbiter handed it the key:

```rust
let lock_toggle =
    keys.just_pressed(KeyCode::KeyR) && route.claim(RKeyUse::LockOn, "combat::gather_input");
```

## The ordering that makes it work

```
First:   reset_r_route                       combat.rs:2555
Update:  quest_demo                          (scripted input, demo only)
         combat::r_route_probe               (scripted input, probe only)
         quest::check_block_place_triggers   .after(quest_demo)
                                             .after(combat::r_route_probe)
                                             .before(combat::gather_input)   quest.rs:469-472
         combat::gather_input
```

Three edges are load-bearing and must not be dropped:

- `.before(combat::gather_input)` — gives the build prompt first refusal. Without
  it the two consumers race and both fire.
- `.after(quest_demo)` / `.after(combat::r_route_probe)` — the scripted presses
  are injected *inside* `Update`; a reader ordered before the injector sees
  nothing, because `just_pressed` is cleared next `PreUpdate`.
- `reset_r_route` in `First` — see contract rule 4.

**Do not** order `quest_demo .after(check_kill_triggers)`: that closes a schedule
cycle (`quest_demo → gather_input → player_combat → check_kill_triggers →
quest_demo`) and Bevy panics while building the graph, before frame 1. The
reasoning is in `docs/q4-one-run-triage.md` §7a.

## Reading a run

`VOXELFORGE_RKEY_LOG=1` (implied by the probe) prints one line per decision:

```
R_ROUTE frame=92 CLAIM LockOn who=combat::gather_input
R_ROUTE frame=… DENY  want=LockOn who=combat::gather_input already=QuestPlace claimed_by=quest::check_block_place_triggers
```

`DENY` is not an error — it is the arbiter working. The quest side prints its own
half:

```
QUEST_DEBUG_BLOCK no place_block objective in reach - R left to lock-on
QUEST_DEBUG_BLOCK R already claimed by … - build objective skipped this frame
```

## Proving it

- **Headless, no GPU:** six `#[cfg(test)]` tests in `combat.rs:3145-3243`
  (`stub_quest_claim` stands in for the quest system) — run with
  `bash scripts/build_safe.sh test --bin voxelforge`. They cover both directions,
  the per-frame reset, and first-claimer-wins. They do **not** cover the real
  schedule ordering: the stub is registered by the test, so a broken
  `.before(gather_input)` edge in `quest.rs` still passes them. That gap is what
  the runtime proof is for.
- **Runtime, one log:** `scripts/_poppy_rkey_proof.sh [target-dir]` builds and runs
  `VOXELFORGE_RKEY_PROBE=1 VOXELFORGE_QUEST_DEMO=1`. The probe taps R at t=2.0s
  and t=2.6s, far from any build objective (`CLAIM LockOn`); the quest demo
  presses R standing on q4's `o1_build` (`CLAIM QuestPlace`). Both directions in
  one file is the proof.
- **Run it from the repo root.** `scene::play_map()` tests `maps/edhari.json`
  with a **relative** path (`scene.rs:42-46`), so an exe launched with its own
  directory as CWD silently boots procedural terrain instead of Edhari
  (`SCENE_READY source=procedural+clearing`), the demo's hand-surveyed
  `GATE_ROUTE` walks into nothing, and phase 0 times out at 90 s without ever
  reaching q4. Assets are pinned to the exe dir (commit 888b31c) and are *not*
  affected — only the map is.
