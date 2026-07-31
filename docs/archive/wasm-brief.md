# ⛔ SUPERSEDED 2026-07-31 — CEO cancelled web/wasm target permanently

> **Reason:** Voxelforge is now a **native desktop game targeting Steam** (Windows first, Vulkan/DX12).
> All web/wasm research, browser support matrices, wasm-opt profiles, and web build instructions
> in this document are **no longer applicable**. The document is archived for historical reference only.
>
> **Moved to:** `docs/archive/wasm-brief.md` — do not use for any active work.

---

# Voxelforge — Rust + Bevy 0.19 on WASM/WebGPU: Constraints & Best Practices (SUPERSEDED)

> **Dev-facing research brief.** Covers browser support, wasm runtime limits, and binary size/perf —
> for shipping Voxelforge (voxel game, compute-heavy renderer) to the web.
> Last updated: 2026-07-27 · Research: Sahara (via Monanisa, covering usage-limit gap) · Owner: Poppy (engine)

---

## A. WebGPU browser support — should we ship a WebGL2 fallback?

### Support matrix (July 2026)

| Browser | WebGPU | Version | Notes |
|---|---|---|---|
| Chrome / Edge (desktop) | ✅ default-on | 113+ | since Apr 2023 |
| Chrome Android | ✅ default-on | 121+ (Android 12+) | |
| Safari macOS | ✅ default-on | Safari 26.0 (macOS Tahoe 26) | |
| Safari iOS/iPadOS | ✅ default-on | iOS/iPadOS 26 | |
| Firefox Windows | ✅ default-on | 141+ | |
| Firefox macOS (ARM64) | ✅ default-on | 145+ | Intel Mac / older macOS lag |
| Firefox Linux | ❌ not shipped | — | blocks Bevy's WebGL2-drop plan (see below) |
| Firefox Android | ❌ in dev | — | Mozilla targeting late 2026 |
| Chrome Linux | ⚠️ partial | 144 beta+ | GPU/driver-gated, not universal |

**Aggregate:** ~83.6% global desktop coverage (caniuse), ~70% mobile (dragged down by Firefox Android + old iOS).

### Verdict: **do not build a dual WebGL2 fallback path**

1. **Bevy can't ship one binary for both backends.** [bevy#13168](https://github.com/bevyengine/bevy/issues/13168) (open) — you'd need two separate wasm builds + client-side `navigator.gpu` feature-detect to pick one. Real engineering cost.
2. **WebGL2 loses what a voxel renderer needs most:** no compute shaders at all, and storage buffers shrink to 16KB-uniform-buffer-only (vs 128MB min on WebGPU) — this isn't "same look, slower," it's a materially different, lower-fidelity render path (GPU-driven meshing/culling would need reimplementing or cutting).
3. **Bevy maintainers are signaling WebGL2's exit from core** within ~1–3 releases, contingent on Linux WebGPU landing (maintainer comment, 2026-04-24 on #13168) — investing in a fallback now targets a shrinking, soon-unsupported segment.

**Action:** Ship WebGPU-only. Gate the web build behind a `navigator.gpu` feature-detect landing page that tells unsupported users (mostly Linux Firefox/old-GPU Chrome, Firefox Android, pre-2025 Safari) to use a supported browser. Re-check in ~6 months (Firefox Android + Linux coverage are the swing factors).

**Sources:** [caniuse.com/webgpu](https://caniuse.com/webgpu) · [gpuweb Implementation Status](https://github.com/gpuweb/gpuweb/wiki/Implementation-Status) · [bevy#13168](https://github.com/bevyengine/bevy/issues/13168) · [bevy#17869](https://github.com/bevyengine/bevy/issues/17869) · [bevy#19469](https://github.com/bevyengine/bevy/issues/19469) · [Bevy + WebGPU blog](https://bevy.org/news/bevy-webgpu/)

---

## B. WASM runtime constraints

### B1 — Threading / Rayon
- **Constraint:** `wasm32-unknown-unknown` has no real threads by default; Bevy's multithreaded ECS scheduler does **not** run multithreaded on web. Real parallelism needs `SharedArrayBuffer` + Web Workers, which needs the server to send `Cross-Origin-Opener-Policy: same-origin` + `Cross-Origin-Embedder-Policy: require-corp` (and then every cross-origin subresource needs `Cross-Origin-Resource-Policy: cross-origin` or it's blocked).
- **Workaround:** Bevy CLI unstable flag `web-multi-threading = true` unlocks the infra for compatible crates (audio). For general parallel work (chunk meshing/worldgen) use **`wasm-bindgen-rayon`** directly — needs nightly + `-Ctarget-feature=+atomics,+bulk-memory,+mutable-globals`, same COOP/COEP headers, explicit `initThreadPool()` call. itch.io needs manual "Enable SharedArrayBuffer" toggle under Embed Options.
- **Voxelforge gotcha:** budget chunk meshing/worldgen for either (a) the COOP/COEP + wasm-bindgen-rayon path, or (b) a time-sliced single-thread fallback for hosts that strip those headers. Audit `Cargo.lock` (rapier/parry etc.) for unconditional `std::thread`/`Mutex` use that won't compile for wasm32.

### B2 — Audio
- **Constraint:** `bevy_audio`/rodio target Web Audio API and compile fine, but browsers block audio start until a user gesture (`AudioContext` starts `"suspended"`). Real-world reports show playback can stay flaky even post-gesture in iframe embeds ([bevy#15273](https://github.com/bevyengine/bevy/issues/15273)). Not multithreaded on wasm by default.
- **Workaround:** Gate first audio-touching action behind an explicit "Click to start" — never autoplay ambient music on load. For heavier audio needs, `bevy_seedling` (Firewheel) + `web-audio` feature + the multi-threading unstable flag gets worklet-based mixing off the main thread.

### B3 — Asset loading
- **Constraint:** No filesystem on web — assets load via async `fetch()`. `AssetServer::load_folder()` **does not work on wasm** (no directory-listing equivalent — [bevy#2916](https://github.com/bevyengine/bevy/issues/2916), [#9591](https://github.com/bevyengine/bevy/issues/9591)). Cross-origin asset URLs need CORS headers.
- **Workaround:** Generate a build-time manifest (`.json`/`.ron`) of asset paths instead of folder-scanning. HTTP asset loading is now upstreamed into Bevy core (formerly the third-party `bevy_web_asset` crate, archived Oct 2025) — check the 0.19 migration guide for the exact integrated API. **For voxel chunk streaming specifically:** don't route dynamically-generated world data through the generic `AssetServer` — stream raw chunk bytes over your own `fetch()`/WebSocket protocol instead.
- **Gotcha:** `dynamic_linking` Cargo feature is unsupported on wasm — make sure it's off in the web profile.

### B4 — bevy_egui on wasm
- **Constraint:** Compiles and runs, but: mobile virtual keyboard only works if `WindowPlugin.prevent_default_event_handling` stays `true` (conflicts with capturing all keyboard input for gameplay); touch-release events are unreliable ([bevy#3752](https://github.com/bevyengine/bevy/issues/3752), [#9116](https://github.com/bevyengine/bevy/issues/9116)); touch Y-axis coordinate bugs reported on wasm exports ([bevy#10694](https://github.com/bevyengine/bevy/issues/10694)).
- **Workaround:** Keep `prevent_default_event_handling: true` if mobile text input matters; scope any keyboard-capture-for-gameplay narrowly instead of disabling default handling globally. Test touch UI on real mobile Safari/Chrome, not just desktop emulation. Given Voxelforge's current control scheme is desktop mouse/keyboard (third-person orbit controller), treat mobile/touch as stretch-goal QA, not launch-blocking.

**Sources:** [Bevy CLI — web multi-threading](https://thebevyflock.github.io/bevy_cli/cli/web/multi-threading.html) · [bevy#4078](https://github.com/bevyengine/bevy/issues/4078) · [wasm-bindgen-rayon](https://github.com/RReverser/wasm-bindgen-rayon) · [Bevy Cheat Book — WASM](https://bevy-cheatbook.github.io/platforms/wasm.html) · [bevy#16307](https://github.com/bevyengine/bevy/issues/16307) · [bevy_egui README](https://github.com/vladbat00/bevy_egui/blob/main/README.md)

---

## C. Binary size & performance

### Baseline (unoptimized)
Bevy's own `fox` example wasm: **~22MB**. Community reports of default-feature Bevy wasm builds running **20–30MB+**, coming down to ~15MB after `wasm-opt` alone. Driven by: monolithic default features (audio codecs, GLTF/scene loading, gizmos, dev-tools, hot-reload watcher), unstripped symbols, no dead-code elim in default release profile. Tracked as [bevy#3978](https://github.com/bevyengine/bevy/issues/3978) and [#15596](https://github.com/bevyengine/bevy/issues/15596) (still open).

### Reduction checklist

**1. Dedicated `web-release` Cargo profile:**
```toml
[profile.web-release]
inherits = "release"
opt-level = "z"       # benchmark "z" vs "s" — cheat book: "z" isn't always smaller
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

**2. `wasm-opt` (binaryen) AFTER `wasm-bindgen`, never before** (can panic wasm-bindgen if run first):
```bash
wasm-opt -Oz -o output_optimized.wasm output.wasm
```
Reported gain: another 15–20% beyond LLVM alone.

**3. Consider `bevy_cli`** — ships built-in web-release profiles and runs `wasm-opt` automatically instead of hand-rolling scripts.

**4. Trim Bevy features — `default-features = false` + explicit list:**
```toml
bevy = { version = "0.19", default-features = false, features = [
  "bevy_winit", "bevy_render", "bevy_pbr", "bevy_asset", "webgpu",
] }
```
Drop for web release builds: `bevy_audio`(if unused)/codec bloat, `bevy_gizmos`, `bevy_dev_tools`, `bevy_scene` (unless loading GLTF), `file_watcher`/`embedded_watcher` (dev-only), `multi_threaded` (no real benefit on wasm — see B1), `dynamic_linking` (must be off).

**5. `wee_alloc`** — smaller code size vs default allocator; test it doesn't regress voxel chunk-mesh allocation hot path.

### Real-world numbers
- `2d_rect` trivial example: 13MB → **~4MB** after size flags (~70% cut) — use as the reduction-ratio reference.
- `vx_bevy` (Bevy voxel prototype, greedy meshing, 16-chunk render distance): **~100 FPS on a GTX 1060-class GPU**, native release build.
- **wasm perf is not native perf, independent of binary size:** [bevy#6363](https://github.com/bevyengine/bevy/issues/6363) (inconsistent rendering on weaker hardware) and [#7242](https://github.com/bevyengine/bevy/issues/7242) (FPS drop from ~30 to ~5 in Brave on a trivial scene) are known, documented issue classes — load-test on real target browsers early, don't assume parity.
- Large GLTF/texture payloads can take **minutes** to load over wasm (Cheat Book) — argues for procedural/streamed chunk generation over baked assets for Voxelforge specifically.

### Targets for the dev team

| Metric | Unoptimized baseline | Target after checklist |
|---|---|---|
| `.wasm` size | 20–30MB+ | **5–10MB** |
| FPS, mid-tier GPU | untested/unstable | validate empirically per browser — don't assume native parity |
| Threading | N/A | design chunk meshing to tolerate single-thread scheduling (`AsyncComputeTaskPool` spread across frames) unless COOP/COEP + wasm-bindgen-rayon path is set up |

**Sources:** [Bevy Cheat Book — Optimize for Size](https://bevy-cheatbook.github.io/platforms/wasm/size-opt.html) · [Rust and WebAssembly book — Shrinking .wasm Size](https://rustwasm.github.io/docs/book/reference/code-size.html) · [Bevy CLI — Web Apps](https://thebevyflock.github.io/bevy_cli/cli/web.html) · [bevy#3978](https://github.com/bevyengine/bevy/issues/3978) · [vx_bevy](https://github.com/Game4all/vx_bevy) · [bevy#6363](https://github.com/bevyengine/bevy/issues/6363) · [bevy#7242](https://github.com/bevyengine/bevy/issues/7242)

---

## Action items (priority order)

1. Add `web-release` Cargo profile + `wasm-opt -Oz` in the build pipeline (after wasm-bindgen).
2. Audit `Cargo.toml` → `default-features = false`, trim to what the renderer/input/asset pipeline actually uses.
3. Ship WebGPU-only; add a `navigator.gpu` feature-detect landing gate — skip the WebGL2 dual-build.
4. Design chunk meshing/worldgen to tolerate single-thread scheduling as the default path; treat `wasm-bindgen-rayon` + COOP/COEP as an opt-in perf upgrade, not a dependency.
5. Stream chunk data via a custom fetch/WebSocket protocol, not `AssetServer::load_folder()` (doesn't work on wasm) or generic asset loading for world data.
6. Gate audio behind an explicit user-gesture start; don't autoplay.
7. Keep `prevent_default_event_handling: true` unless mobile text input is explicitly out of scope.
8. Load-test on real mid-tier hardware across Chrome/Firefox/Brave/Safari before locking a performance target — wasm perf variance across browsers is a documented, real issue class, not hypothetical.

---

*Research: Sahara (compiled by Monanisa while Sahara was at usage limit). Engine implementation: Poppy.*
