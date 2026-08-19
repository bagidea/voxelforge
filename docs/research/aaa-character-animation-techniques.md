# AAA Character Design + Animation Techniques — Proportion, Silhouette, Blend Trees, IK, Root Motion

**Status:** research-only. No renderer/animation code (`anim.rs`, `characters.rs`, `enemies.rs`, `equipment.rs`, `combat.rs`) and no assets were touched.
**Owner of this lane:** the character + animation lane. This doc hands it the *techniques*, the *primary sources*, the *gap vs the boss's ruler* (AAA stylized action games), and the *exact Bevy 0.19 surface* to build on.
**Method:** same discipline as `aaa-look-techniques.md`. Every Bevy fact below is read from the **local cargo registry source** (`bevy_animation-0.19.0`, `bevy_math-0.19.0`, `bevy_gltf-0.19.0` — all three are already in `Cargo.lock`, no new dependency needed for the core). Every external crate was checked on crates.io / docs.rs for its *real* Bevy version requirement; crates that are stale or non-existent are marked **not found / stale**, never cited as usable. Every URL was opened and confirmed reachable.

---

## 0. What Voxelforge already ships — the base to build on

`client/src/anim.rs` (3,322 lines) is a **mature *procedural* animation layer** — the avatar and the Guard Husk are a 13-joint humanoid rig built out of cuboid slabs, parented as pseudo-bones, and every pose is *computed*, not sampled from a clip (`anim.rs:1-34`). `characters.rs` (1,262), `equipment.rs` (1,519), `enemies.rs` (908) build the actor rigs; `combat.rs` (4,768) publishes the state the rig reads.

| System | Where | What it is today |
|---|---|---|
| Rig | `anim.rs` `build_rig()` `:1046` | 13-joint box humanoid; child of the actor, no `SkinnedMesh` |
| Locomotion | `locomotion()` `:1997` | idle→walk→run blended on the actor's **real speed**; stride phase advanced by distance (no foot skating, `:10`) |
| Anticipation → contact → follow-through | `SwingPhase` enum `:71`, `swing_pose()` `:2318` | attack arcs keyed to `combat`'s own `LIGHT_ACTIVE`/`HEAVY_ACTIVE` timers |
| Additive recoil | `hit_react()` `:2165` | additive hit-reaction fired by a `Health.cur` drop |
| Weight / secondary | `locomotion()`, `landing()` `:2184`, `spring()` `:3091` | head/torso bob, lateral sway, cloak slab, squash-and-stretch on landing |
| Easing + springs | `ease_out` `:3051`, `ease_io` `:3057`, `ease_out_back` `:3079`, `smoothstep` `:3045`, `spring` `:3091` | hand-rolled easing + critically-damped springs |
| Turn-in-place | `turn_in_place()` `:2128` | rig leads the root with a torso/hips twist |
| Visual timing events | `AnimSwing`/`AnimDodge`/`AnimParry` messages `:84` | visual-phase events, deliberately split from `combat`'s gameplay events (`:46-67`) |
| glTF import | `import.rs` `ModelKind::Gltf` `:137` | `.glb`/`.gltf` already loads as a Bevy scene (`import.rs:214-222`) — but only as a *model*, no clip is played yet |

**The gap this doc is about.** Everything is *computed*; nothing is *sampled*. There is **no `SkinnedMesh`, no `AnimationPlayer`, no `AnimationClip`, no `AnimationGraph`** in the client today (a grep of `client/src` matches only prose comments and the raw `Gltf` asset load). No foot IK (feet are posed by the stride function, not grounded to voxel terrain), no root motion, no ragdoll. That is the right engineering for one hero + one husk, but it is a dead end for a full AAA cast: authoring every pose for N characters × M moves as code does not scale — authored clips + a blend tree does. The good news: the hard *animation theory* (anticipation/follow-through, additive layering, secondary motion, easing) is **already proven in `anim.rs`**; this lane's job is to carry it onto a sampled rig, not to re-learn it.

---

## 1. Character design — what makes a voxel/stylized character read "expensive"

### 1.1 Proportion + silhouette readability (the two free wins)

The single biggest visual payoff per unit effort. Two levers:

- **Proportion.** Stylized characters compress the head-to-body ratio — roughly 2.5–4 heads tall vs a realistic 7–8 — and enlarge the head, hands and feet. In a voxel game the character is small on screen, so a big head + big hands + big feet are what stay readable. This is what *Minecraft Dungeons* (chunky ~3-head heroes) and *Hi-Fi Rush* (Chai's oversized hands + distinctive limbs) both exploit, and it is the one axis our box rig already leans into.
- **Silhouette readability.** Fill the character with solid black: if you can't tell who it is and what it's doing, it fails. *Overwatch* is the canonical silhouette-first design (each hero is recognisable by outline alone), and it maps directly to the 12 principles' **appeal** and **solid drawing** (§5, Wikipedia). For us: the reaver / sentinel / husk already differ in mass and outline — keep every new enemy silhouette distinct before adding detail.

### 1.2 Material/shading — character vs world separation

The expensive tell is not more polygons, it's that the character is **shaded differently from the world** so it pops:

- **Cel / toon shading on the character** (quantised diffuse ramp, hard or absent specular) while the world stays PBR. *Breath of the Wild*, *Hi-Fi Rush* and *Guilty Gear* all do this; it is the cheapest way to make a hero read as "authored" against a busy environment.
- **Fresnel rim light** on the character silhouette — a warm or cool rim lifts the hero off a cluttered voxel scene.
- **Palette / value grouping** — a saturated, warm character against a desaturated, cooler world (or vice versa). Fewer, larger value shapes = more expensive-looking than noisy detail.

Bevy note: this is the *look lane's* material work on top of `StandardMaterial` (already used by `look.rs`); the character lane's part is to author a mesh + rig that a toon/rim material can read. Don't start here — start with proportion/silhouette, which cost zero GPU.

### 1.3 For Voxelforge specifically

Our characters are cuboid-slab rigs — that is a *style choice*, not a downgrade (Cube World, Trove, Minecraft Dungeons are all box-based). The "expensive" upgrades, in order:

1. An authored **box-rig mesh** with real faces, eyes and separated limbs (not a single capsule) — the current rig proves the motion, but reads as a mannequin.
2. A **distinct silhouette per enemy** (keep the reaver/sentinel/husk mass differences, push them further).
3. A character **toon/rim material** to lift the hero off the voxel world (§1.2), once 1 and 2 exist.

---

## 2. AAA animation pipeline a small team can actually run

### 2.1 The pipeline

Author in Blender (or buy from Mixamo), export **glTF** (Bevy's native format), import clips, build a **blend tree**, drive its weights from gameplay parameters (speed, direction, aim). The seven techniques below are the whole AAA stack that matters for a character action game; a 1–2 person team runs the *same* stack, just with fewer authored clips.

### 2.2 The seven techniques

1. **Blend tree / state machine** — never hard-switch between clips; blend on a numeric parameter (e.g. speed 0→1 blends idle→walk→run). Unreal calls it a *Blend Space* (1D/2D graph of clips); Unity calls it a *Blend Tree* (§5). Both engines also carry a separate **transition** concept (crossfade with an exit-time condition) — that is the "attack" layer, distinct from the "locomotion" blend.
2. **Additive layer** — run a base locomotion layer and *add* aim, lean, breathing or a hit-react on top. Unity's animation layers support `Additive` explicitly (§5). Bevy: an `Add` node in the animation graph (§3.1). We already do this procedurally (`hit_react()`); the move to sampled clips must keep it.
3. **Foot IK** — after the animation poses the feet, run an IK pass to *pin* each planted foot to the ground so it doesn't clip through voxel terrain. Small-team version: a 2-bone (hip→knee→ankle) solver + a raycast ground probe, one bone chain per leg.
4. **Root motion** — let the character's *locomotion* come from the animation itself (the root bone's delta each frame), so the feet and the world-movement can never disagree (no foot skating). Unity documents this explicitly (§5). Bevy: **not built in** — see §3.2.
5. **Anticipation / follow-through** — 12 principles #2 and #5: every action winds up before it, and every part keeps moving after the main action stops. `anim.rs` already encodes this (`SwingPhase::Windup → Contact → Recover`); it must survive the move to authored clips or the animation will regress.
6. **Secondary motion / overlap** — 12 principles #8: hair, cloak, tail, chest continue moving a beat after the body. `anim.rs` already does head/torso bob and the cloak slab; in a sampled rig this becomes a physics/cloth pass or an additive layer.
7. **Animation curves, not linear** — ease in/out so motion accelerates and settles, instead of a constant lerp that reads robotic. Both engines author per-property curves (§5, Unity curves). Bevy: `bevy_math::curve` gives the whole easing library (§3.1).

### 2.3 Small-team reality

- **1 animator, 0 mocap budget:** buy/retarget Mixamo clips for the common verbs (idle/walk/run/jump), and hand-key only the hero's **signature** moves. Retargeting is free in Bevy (§3.1) as long as bone names match.
- **The hybrid that fits this project:** authored clips for the *personality* moments (signature attacks, deaths, emotes), and keep `anim.rs`'s procedural layer for the *continuous* stuff (locomotion weight, secondary bob, springs). We already have the procedural layer working — the doc's recommendation is to *add* a sampled base under it, not to rip it out.

---

## 3. Bevy 0.19 — what's real (verified against the local crate source)

### 3.1 The animation core (in-tree, already a dependency)

The graph-based system (Bevy RFC 51, §5). Exact names, from `bevy_animation-0.19.0`:

- **`AnimationClip`** (`lib.rs:105`) — curves keyed by `AnimationTargetId`; `add_curve_to_target()` `:284`, `duration()` `:254`.
- **`AnimationTargetId`** (`lib.rs:187`) — a UUID **name-path** id (`from_name`/`from_names` `:1316`). Because clips target *bone-name paths*, **retargeting is free**: any clip animating `"Hips/Spine/Chest/…"` plays on any rig whose bones carry the same names (`lib.rs:163-181`). This is the Mixamo-retarget story — standardise bone names and you're done.
- **`AnimatedBy(Entity)`** (`lib.rs:215`) — links a bone entity to its `AnimationPlayer`.
- **`AnimationPlayer`** (`lib.rs:732`) — `start()` `:859`, `play()` `:866`, `stop()` `:872`, `pause_all()`/`resume_all()`, `adjust_speeds()` `:949`, `seek_all_by()` `:963`.
- **`ActiveAnimation`** (`lib.rs:509`) — per-node `set_weight()` `:609`, `set_speed()` `:666`, `set_repeat()` `:635`, `seek_to()` `:708`, `rewind()` `:719`, `just_completed()` `:682`. **`RepeatAnimation::{Never, Count(u32), Forever}`** `:471`.
- **`AnimationGraph`** (asset, `.animgraph.ron`, `graph.rs:114`) + **`AnimationGraphHandle`** (component `:138`). It is a DAG of three node types — **`AnimationNodeType::{Clip, Blend, Add}`** (`graph.rs:217`):
  - `add_clip(clip, weight, parent)` `:473`, `add_blend(weight, parent)` `:537`, **`add_additive_blend(weight, parent)` `:577`** (the additive layer, #2.2.2), `add_edge` `:619`, `from_clip`/`from_clips` `:445`.
  - **Masks**: `add_clip_with_mask` `:492`, `add_blend_with_mask` `:555`, `add_target_to_mask_group` `:672`; `AnimationMask = u64` = 64 mask groups `:426`. Masks are how you stop a "hold weapon" animation from overriding the weapon hand — the doc comment on `graph.rs:95-101` describes exactly that use.
- **`AnimationTransitions`** (`transition.rs:33`) — `play(&mut player, node, Duration)` `:78` crossfades (greedy-layer weight normalisation `:111`). Use this for the "attack" transitions; use `AnimationPlayer::start` for instant cuts.
- **Animation events** — `AnimationClip::add_event(time, event)` `lib.rs:326` + `#[derive(AnimationEvent)]`; fire a footstep/impact at an exact clip time (Bevy example `animation_events.rs`). This replaces `anim.rs`'s hand-rolled `AnimSwing` phase-crossing for sampled clips.
- **Morph targets / blend shapes** — `MorphWeights` (bevy_mesh) driven by **`WeightsCurve`** (`morph.rs:21`); the glTF loader already builds morph targets (`bevy_gltf/src/loader/mod.rs:797`). This is facial animation / blinking / muscle bulge.
- **Curves + easing** — `bevy_math::curve`: the **`Curve`** trait (`sample_clamped` `:349`, `map` `:423`, `reparametrize` `:465`, `chain` `:564`, `repeat` `:613`, `forever` `:646`, `ping_pong` `:663`, `resample` `:801`); **`FunctionCurve::new(Interval::UNIT, |t| …)`**; **`EasingCurve::new(start, end, ease_fn)`** (`easing.rs:299`) with **`EaseFunction`** (`:435`) and named curves `LinearCurve`, `CubicInOutCurve`, `SmoothStepCurve`, `SmootherStepCurve`, `SineInOutCurve`, `BackInOutCurve`, `ElasticCurve`, `BounceOutCurve`, `StepsCurve` … (`easing.rs:675-966`); **`AnimatableKeyframeCurve::new([(t, value), …])`** (`animation_curves.rs:723`) + **`animated_field!(Transform::translation)`** (`:794`) + **`AnimatableCurve`** (`:288`) build a clip curve.

**What the glTF loader gives you for free** (`bevy_gltf-0.19.0`): with `GltfLoaderSettings::load_animations` (default **true**, `loader/mod.rs:201`), a `.glb` with animations loads **`AnimationClip`** assets (exposed on `Gltf.animations` / `Gltf.named_animations`, `assets.rs:43-46`), inserts **`AnimationTargetId`** on every bone (`loader/mod.rs:562`, name-path — hence retargeting) and **`AnimatedBy`**, and inserts an **`AnimationPlayer`** on the root (`loader/mod.rs:1093`). It supports glTF `Linear` / `Step` / `CubicSpline` interpolation (`loader/mod.rs:349-377`). **It does NOT build an `AnimationGraph`** — you build one yourself (`AnimationGraph::from_clips` / `add_clip`) and attach `AnimationGraphHandle`.

### 3.2 What's NOT there (verified absent — the honest gaps)

- **Root motion: no support.** A grep for `root motion` / `root_motion` / `motion` across `bevy_animation-0.19.0` and `bevy_gltf-0.19.0` returns nothing. The glTF loader keys the whole scene off a root `AnimationPlayer` but does **not** extract the root bone's translation into a motion delta. Hand-roll it: each frame read the root bone's `Transform.translation` delta, subtract it from the rendered root and add it to the actor's `Transform` (exactly what `anim.rs`'s `swing_root_motion()` `:2383` fakes for swings).
- **Foot IK: no in-tree component, no mature crate.** No IK lives in Bevy. Hand-roll a 2-bone solver + ground probe (§3.3 gives the probe).
- **Ragdoll: no in-tree.** Use a physics engine's joints (§3.3).

### 3.3 External crates (verified on crates.io / docs.rs)

| Crate | Version | Bevy req | Verdict for 0.19 |
|---|---|---|---|
| `avian3d` | 0.7.0 | `^0.19.0` | ✅ physics; `spatial_query` module has **`RayCaster`**, **`ShapeCaster`**, **`SpatialQuery`** (ground probe for foot IK) + joints for ragdoll |
| `bevy_tweening` | 0.16.0 | `^0.19` | ✅ tweens for procedural secondary / camera / UI |
| `bevy_hanabi` | 0.19.0 | `^0.19` | ✅ particle trails, dust, impact bursts (secondary motion) |
| `bevy_asset_loader` | 0.27.0 | `^0.19.0` | ✅ asset-loading states (rigs + clips, avoid pop-in) |
| `bevy_mod_raycast` | 0.18.0 | `^0.14.0` | ❌ **stale** — pinned to Bevy **0.14**; do NOT use. Use `avian3d::spatial_query` for the foot-IK probe instead |
| `bevy_ik` (crates.io) | 0.0.1 | — | ❌ reserved stub ("reserved for the Bevy Engine project"), not a real IK crate |
| `bevy_ik` (gschup, GitHub) | — | Bevy 0.9 | ❌ FABRIK IK, self-described WIP "not 100% functional", targets Bevy **0.9**; hand-roll instead |
| `bevy_retarget` | — | — | ❌ **not found** (404 on crates.io and on GitHub `Unidot/bevy_retarget`). Retarget via the built-in `AnimationTargetId` name-path (§3.1); the canonical tracking issue is bevyengine/bevy#15612 |
| `bevy_fbx` | — | — | ❌ **not found** on crates.io. FBX must be converted to glTF (Blender / Mixamo export) before Bevy loads it |

Related retargeting-adjacent work that *does* exist (for reference, not required): `emberlightstudios/humentity` (MakeHuman + glTF-clip retargeting) and `bevy_vrm1` (VRM humanoid bone markers). Neither is needed if bone names are standardised.

### 3.4 Bevy examples to copy (verified filenames in `examples/animation/`)

- `animation_graph.rs` — the blend-tree graph
- `animation_masks.rs` — mask groups (hold-object)
- `morph_targets.rs` — blend shapes
- `custom_skinned_mesh.rs` — skinned mesh from scratch
- `easing_functions.rs`, `eased_motion.rs` — easing
- `animation_events.rs` — timed clip events (footsteps)
- `animated_mesh.rs`, `animated_mesh_control.rs` — clip playback / control

---

## 4. Impact / effort — cheapest first, mapped to real files

| # | Item | What it buys | Effort | Touch points |
|---|---|---|---|---|
| 1 | **Skinned-mesh rig + `AnimationPlayer`** (glTF import, one clip plays) | Sampled poses instead of hand-computed; unlocks every item below | medium | `import.rs` (already loads glTF); new `rig.rs`; `AnimationGraph::from_clips` + `AnimationGraphHandle` |
| 2 | **Blend tree for locomotion** (idle→walk→run on speed) | Authored, smooth locomotion | small | `AnimationGraph::add_blend`; drive node weights from `combat`'s real speed (the same number `anim.rs` already reads) |
| 3 | **Character toon/rim material** | Hero pops off the voxel world | small–medium (look lane) | `characters.rs` mesh material + a toon/rim shader in the look lane |
| 4 | **Additive layer** (aim / lean / breathe on top of locomotion) | Life on top of the base | small | `AnimationGraph::add_additive_blend` (`Add` node) |
| 5 | **Retarget Mixamo clips** (standardise bone names) | Near-free animation content | small | bone-name convention; `AnimationTargetId` name-path does the rest |
| 6 | **Foot IK** (2-bone, `avian3d` raycast) | Feet ground on uneven voxel terrain | medium | new `foot_ik.rs`; `avian3d` `spatial_query::RayCaster` |
| 7 | **Root motion** (hand-rolled root-delta) | No foot skating during attacks | medium | `anim.rs` root-delta reader → actor `Transform` (mirror `swing_root_motion` `:2383`) |
| 8 | **Morph targets** (blink / facial / hit bulge) | Character expression | medium | `MorphWeights` + `WeightsCurve` |
| 9 | **Ragdoll** (avian joints on the corpse rig) | Physics death | large | `anim.rs` corpse rig `:1437` → avian joint chain |

**Recommended order: 1 → 2 → 5 → 3 → 4**, then **6 → 7** as the "feet never skate" pass, then **8**, and **9** last (it is the only genuinely large item and the procedural death in `anim.rs` already reads fine).

---

## 5. Primary sources (all verified reachable at time of writing)

### Animation principles / pipeline
- Twelve basic principles of animation (the canonical source for anticipation, follow-through, secondary, exaggeration, appeal) — `https://en.wikipedia.org/wiki/Twelve_basic_principles_of_animation`
- Unreal Engine, "Blend Spaces" — `https://dev.epicgames.com/documentation/en-us/unreal-engine/blend-spaces-in-unreal-engine`
- Unity Manual, "Blend Trees" — `https://docs.unity3d.com/Manual/class-BlendTree.html`
- Unity Manual, "How Root Motion works" — `https://docs.unity3d.com/Manual/RootMotion.html`
- Unity Manual, "Animation Layers" (additive layers) — `https://docs.unity3d.com/Manual/AnimationLayers.html`
- Unity Manual, "Use Animation curves" — `https://docs.unity3d.com/Manual/animeditor-AnimationCurves.html`

### Bevy 0.19 — primary (local registry source)
- `bevy_animation-0.19.0/src/graph.rs` (`AnimationGraph` :114, `AnimationNodeType::{Clip:217, Blend:224, Add:238}`, `add_blend` :537, `add_additive_blend` :577, `AnimationMask` :426)
- `bevy_animation-0.19.0/src/lib.rs` (`AnimationClip` :105, `AnimationTargetId` :187, `AnimatedBy` :215, `AnimationPlayer` :732, `ActiveAnimation` :509, `add_event` :326, retargeting doc :163-181)
- `bevy_animation-0.19.0/src/transition.rs` (`AnimationTransitions` :33, `play` :78)
- `bevy_animation-0.19.0/src/animation_curves.rs` (`AnimatableKeyframeCurve` :723, `AnimatableCurve` :288, `animated_field!` :794)
- `bevy_animation-0.19.0/src/morph.rs` (`WeightsCurve` :21)
- `bevy_math-0.19.0/src/curve/easing.rs` (`EasingCurve` :299, `EaseFunction` :435) and `curve/mod.rs` (`Curve` trait :327-905)
- `bevy_gltf-0.19.0/src/assets.rs` (`animations` :43, `named_animations` :46); `loader/mod.rs` (`load_animations` :201, `AnimationTargetId` insert :562, `AnimationPlayer` insert :1093)

### Bevy — tracking / examples
- Bevy RFC 51 (animation composition — the graph design) — `https://github.com/bevyengine/rfcs/blob/main/rfcs/51-animation-composition.md`
- Bevy animation examples (filenames in §3.4) — `https://github.com/bevyengine/bevy/tree/main/examples/animation`
- Bevy issue #15612 "Animation Retargeting Woes" — `https://github.com/bevyengine/bevy/issues/15612`

### External crates (crates.io / docs.rs / GitHub, all checked)
- `avian3d` 0.7.0 (bevy ^0.19; `spatial_query` ray/shape casting) — `https://docs.rs/avian3d/0.7.0/avian3d/`; `https://github.com/Jondolf/avian`
- `bevy_tweening` 0.16.0 (bevy ^0.19) — `https://docs.rs/bevy_tweening/0.16.0/bevy_tweening/`
- `bevy_hanabi` 0.19.0 (bevy ^0.19) — `https://docs.rs/bevy_hanabi/0.19.0/bevy_hanabi/`
- `bevy_asset_loader` 0.27.0 (bevy ^0.19) — `https://docs.rs/bevy_asset_loader/0.27.0/bevy_asset_loader/`
- `bevy_mod_raycast` 0.18.0 (bevy ^0.14 — **stale**) — `https://docs.rs/bevy_mod_raycast/0.18.0/bevy_mod_raycast/`
- `gschup/bevy_ik` (FABRIK, WIP, Bevy 0.9) — `https://github.com/gschup/bevy_ik`

**Not found / corrected while writing:** Bevy 0.19 has **no root motion** (grep of `bevy_animation` + `bevy_gltf` empty) — the doc's "root motion is built in" would have been false; it must be hand-rolled. `bevy_retarget` and `bevy_fbx` do **not exist** on crates.io (both 404) — retargeting is the built-in `AnimationTargetId` name-path, and FBX goes through Blender → glTF. The crates.io `bevy_ik` is a **reserved stub** (0.0.1), not the FABRIK implementation; the real one (gschup/bevy_ik) is a WIP pinned to Bevy 0.9, so foot IK is hand-rolled. `bevy_mod_raycast`, the crate everyone reaches for foot probes, is **stuck on Bevy 0.14** — `avian3d::spatial_query` is the 0.19-native replacement. `bevy_tweening`'s version (0.16.0) does not match its Bevy requirement (`^0.19`) — confirmed twice (crates.io API + docs.rs), not a typo.
