# LOOK lane — WebGPU / wasm portability risk scan

**Scope:** every effect in the hero LOOK stack (`client/src/hero.rs` camera bundle + lights),
graded for "will this survive a `wasm32-unknown-unknown` + `--features webgpu` build running in a
browser". **This is a triage list only — nothing has been changed.** Requested by CEO 2026-07-27.

**Companion doc:** [`wasm-brief.md`](wasm-brief.md) (Sahara/Poppy) covers the layer *below* this one —
browser support matrix, wasm runtime limits, binary size. It contains **nothing** about the individual
post-process effects, and this doc contains nothing about browser share; read them together. If the
two ever disagree, `wasm-brief.md` wins on platform facts and this one wins on effect behaviour.

**Method + honesty note:** this is a **static** read of the effect list against known wgpu/WebGPU
feature limits — it is **not** a measured browser run. No browser was launched and **no wasm binary
was ever produced**, so every risk below is *predicted*, not observed.

A wasm build **was attempted** on 2026-07-27 (`cargo check --target wasm32-unknown-unknown` and a
`trunk build --release --features webgpu`) and it did **not** get far enough to say anything about
the effects — it fails at the dependency layer, before a single shader is compiled:

```
getrandom-0.3.4/src/backends.rs:194:17: error: The wasm32-unknown-unknown targets are not supported
by default; you may need to enable the "wasm_js" configuration flag.
error: could not compile `getrandom` (lib) due to 1 previous error
```

That attempt therefore **produced no evidence about any effect below**, and no risk level here was
adjusted from it. It only proved a dependency-layer blocker existed.

**That blocker has since been fixed by the wasm lane** (working tree, 2026-07-27):
`client/Cargo.toml` pins `getrandom` 0.3 *and* 0.4 with the `wasm_js` feature, and
`scripts/web-build.sh` exports the matching `RUSTFLAGS='--cfg getrandom_backend="wasm_js"'` —
`getrandom` errors unless **both** halves are present. So verification is **unblocked but still
unrun by me**: the risk levels below remain *predicted*, and the honest next step is `scripts/web-build.sh`
followed by a real browser (see the bottom section). The rest of the toolchain is present
(`wasm32-unknown-unknown` target + `wasm-bindgen` + `trunk`).

**Context that shrinks the blast radius:** `client/src/main.rs` (the actual shipping game) currently
carries **none** of this post stack — no Bloom, DoF, SSAO, TAA, volumetrics or grade. This whole
stack lives only in the offline hero-shot path (`shot_main.rs` → `hero.rs`), which renders on a real
desktop GPU. So none of these risks are *live* today; they become live the moment someone ports the
hero look onto the playable client for web.

---

## Risk table

| # | effect (`hero.rs`) | risk | what breaks on WebGPU/wasm |
|---|---|---|---|
| 1 | **`DepthOfFieldMode::Bokeh`** (L843) | 🔴 **HIGH** | Bokeh needs the `DUAL_SOURCE_BLENDING` wgpu feature, which browser WebGPU does not expose. Expect a silent downgrade to `Gaussian` (or the DoF node refusing to queue). The hexagonal-bokeh character is web-unreachable; also `sensor_height: 0.35` is a non-physical hack tuned against the Bokeh path, so the Gaussian fallback will not look the same at the same numbers. |
| 2 | **`ScreenSpaceAmbientOcclusion` Ultra** (L858) | 🔴 **HIGH** | SSAO is compute-based (already documented as WebGL2-incompatible). Beyond that, Bevy's SSAO builds its depth mip chain as an **`R16Float` storage texture** — `r16float` is **not** in WebGPU core's permitted storage-texture format list, so the pipeline can fail to create at all. This is the single most likely hard failure in the stack, and G4 (contact AO) depends on it. |
| 3 | **PCSS soft shadows** (`experimental_pbr_pcss`, `soft_shadow_size: 3.0`, L600) | 🟠 **MED-HIGH** | Gated behind a Bevy **experimental** feature flag — not covered by Bevy's web support promises, so it can break on any engine bump independent of WebGPU. Mechanically it should work (WGSL permits non-comparison sampling of `texture_depth_2d`), but the many-tap penumbra loop is expensive on browser-tier GPUs. G4's penumbra half rides on this. |
| 4 | **`VolumetricFog` + `VolumetricLight`** (L609, L870) | 🟠 **MED** | Functionally portable (fragment raymarch), but `step_count: 96` **per pixel with a shadow-map tap per step** is a brutal fill-rate cost — this is the frame-time cliff on integrated/mobile GPUs, not a compile error. Volumetrics are also WebGL2-unsupported, so there is no fallback path. The god-ray + `DUST` motes corridor is the shot's signature, so a step-count cut is a *look* regression, not a free win. |
| 5 | **`TemporalAntiAliasing`** (L828) | 🟠 **MED** | Needs the motion-vector prepass; WebGPU-capable but WebGL2-impossible. Bigger issue is **coupling**: the comment at L823-826 is explicit that TAA is what resolves the stochastic PCSS + Ultra-SSAO samples into clean penumbra/AO. If TAA drops on web, items 2 and 3 don't just lose polish — they turn to visible per-frame noise. |
| 6 | **`ShadowFilteringMethod::Temporal`** (L827) | 🟠 **MED** | Portable in itself; inherits TAA's fate — temporal filtering without temporal accumulation is noise. Treat as one unit with #5. |
| 7 | **Bind-group / texture-limit pressure** (whole stack) | 🟠 **MED** | Depth prepass + motion-vector prepass + SSAO + volumetric + bloom + DoF + PCSS stacked on the PBR shader pushes toward WebGPU's conservative defaults (`maxBindGroups` 4, `maxSampledTexturesPerShaderStage` 16, `maxStorageTexturesPerShaderStage` 4). Failure mode is an ugly one: pipeline-creation errors that only appear with *all* features on, so it won't reproduce if you bisect one effect at a time. |
| 8 | **`Bloom` (NATURAL, intensity 0.26)** (L829) | 🟢 **LOW** | Standard down/upsample fragment chain on a filterable float target. Well-trodden on web. |
| 9 | **`Tonemapping::AcesFitted`** (L804) | 🟢 **LOW** | Needs Bevy's 3D tonemapping LUT textures to load as filterable float — supported on web; just confirm the LUT assets ship in the wasm bundle instead of 404-ing. |
| 10 | **`ColorGrading`** temp/sat/contrast + `SHOULDER` (L811-822) | 🟢 **NONE** | Pure ALU inside the tonemapping shader. The whole atmosphere-pin grade (contrast 1.30 + `hi_gain` shoulder 0.64) is fully portable. |
| 11 | **`DistanceFog`** exponential (L880) | 🟢 **NONE** | Pure math in the PBR shader. |
| 12 | **`DUST` motes** (L534-565) | 🟢 **NONE** | 210 emissive `Cuboid` mesh entities — plain geometry, zero exotic GPU features. Only cost is draw calls, and they're trivially instanceable/batchable if that ever matters. |
| 13 | **`Msaa::Off`, `Exposure`, `ev100`, FOV/near** | 🟢 **NONE** | Plain state. |

## Read in one line

The **grade** half of the look (items 10-13 — tonemap, grade, shoulder, fog, dust) is essentially
free on web. The **god-ray, PCSS and grade** trio the CEO asked about splits: **grade is safe**,
**god-ray is a performance risk not a correctness risk**, and **PCSS is a correctness+perf risk
because it is experimental**. The two effects most likely to hard-fail are the ones nobody asked
about: **Bokeh DoF** (missing dual-source blending) and **SSAO** (`r16float` storage texture).

Also worth flagging: the shot is graded at **1280×720 with ~4.4s of TAA accumulation before the
grab**. A browser at 60fps gets ~16ms and no accumulation budget, so even where every effect
*compiles*, the frame the player sees will not be the frame the gate measured.

## Cheapest way to turn these predictions into facts

```bash
# 1) does it even build for web?  Use the script — a bare cargo/trunk invocation dies on
#    getrandom because it misses the RUSTFLAGS half of the fix.
scripts/web-build.sh
# 2) does the pipeline actually create? (only a real browser answers this)
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' trunk serve --features webgpu
#    ...then read the console for wgpu validation errors
```
Order matters: #1 catches nothing about items 1/2/7 — those are **runtime** pipeline-creation
failures. Only #2 settles them. As of 2026-07-27 **neither has been run to completion here** — #1's
blocker is fixed but the build was never seen through, and #2 has never been reached.

> ⏱ These are **long** builds (the aborted 2026-07-27 attempt was still compiling `bevy_*` after
> ~4 min). Run them in the foreground where you can see them finish — do not fire and walk away.
