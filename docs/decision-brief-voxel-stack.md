# Decision Brief: Voxelforge Stack — Rust + Bevy 0.19 + wgpu

> **Verdict in one sentence:** Build the client on **Bevy 0.19**, use **binary greedy meshing + chunk LOD + Bevy 0.16+ GPU-driven rendering** for native AAA, but treat **web deployment** and **MMO server authority** as separate tracks from day one.

| Goal | Feasibility | Key caveat |
|---|---|---|
| Native desktop AAA voxel | ✅ Viable | Bevy 0.19 + binary greedy meshing + chunk LOD + GPU-driven batching works; real-time RT GI only on NVIDIA RTX via Solari |
| Web / WASM / WebGPU | ⚠️ Secondary tier | WebGPU is mainstream (~77–84% support), but Bevy web defaults to WebGL2; WASM threads need COOP/COEP; RAM/effect limits are below native |
| Embedded webview plugin | ⚠️ Same constraints as web | Use the web build path; verify the host webview actually supports WebGPU |
| MMO server-authoritative | ⚠️ No turnkey crate | Authoritative server and chunk streaming must be built separately; Bevy is client + shared-sim only |

---

## (A) Bevy Voxel Rendering — SOTA 2025–2026

### Meshing options (fastest → simplest)

| Technique | Crate / project | Greedy? | Speed / notes |
|---|---|---|---|
| **Binary greedy meshing** | [`binary-greedy-meshing`](https://github.com/Inspirateur/binary-greedy-meshing) / [`binary_greedy_mesher_demo`](https://github.com/TanTanDev/binary_greedy_mesher_demo) | Yes | **~65 µs/chunk** opaque on Ryzen 5 5500; ~30× faster than plain greedy |
| Greedy meshing | [`block-mesh-rs`](https://github.com/bonsairobo/block-mesh-rs) | Yes | Engine-agnostic `greedy_quads`; older, check Bevy compatibility |
| Face-culling + runtime update | [`bevy_meshem`](https://github.com/Adamkob12/Meshem) | **No** | O(1) runtime updates, smooth AO, good for dynamic props |
| Per-chunk LOD | [`bevy_voxel_world`](https://github.com/splashdust/bevy_voxel_world) | Yes | Simple API, supports Bevy 0.15–0.19 |
| Transvoxel 6-LOD | [`bevy_transvoxels_demo`](https://github.com/ahmadaliadeel/bevy_transvoxels_demo) | Yes | Crack-free transitions, async meshing, Bevy 0.18 |
| SVO-DAG engine | [`voxelis`](https://github.com/WildPixelGames/voxelis) | Its own | Sparse-voxel compression, batch editing, multi-engine bindings |

### GPU-driven rendering in Bevy

**Bevy 0.16 (Apr 2025) shipped the first GPU-driven rendering stack** and it is still the foundation in 0.19:
- **Multi-Draw Indirect / Count (MDI/MDIC)** — draw many meshes in one call.
- **Mesh Allocator** — packed vertex/index buffers on GPU.
- **Bindless Material/Texture Allocator** — fewer bind-group switches.
- **GPU Transform & Cull** — frustum/occlusion culling on compute.
- **Retained Render World** — upload only what changed.
- **Cached Pipeline Specialization** — lower CPU overhead for large scenes.
- **Bevy 0.19 adds** partial bindless / reduced bind-group overhead, render-graph-as-systems, and improved skinned-mesh culling.

For a dense voxel world this means you can push **many chunk draws cheaply**, provided you feed the renderer with merged/batched geometry and use the 0.16+ APIs.

### Reference architecture: Veloren / Voxygen

Veloren is the closest open-source analogue. It is not Bevy-based (its `Voxygen` renderer is wgpu-native), but its design is directly applicable:
- **Greedy meshing** for terrain, figures, sprites.
- **Chunk size 32×32×16**, subdivided into 4×4×4 sub-groups for cache efficiency.
- **LZ4 compression** on the wire, **copy-on-write** during meshing, **8-byte compressed vertex** format.
- **SIMD** terrain/figure meshing, shadow math, animation.
- **LOD terrain + horizon maps + LOD shadows**.
- **Chunk-relative / player-relative coordinates** to fix precision at large distances.

**Recommendation for Voxelforge native:**
1. Use `binary-greedy-meshing` for static/opaque terrain chunks.
2. Use `bevy_meshem` or `block-mesh-rs` for dynamic objects/props that need O(1) updates.
3. Adopt `bevy_transvoxels_demo` (Transvoxel) or `bevy_voxel_world` for terrain LOD.
4. Build the 9-pass pipeline as Bevy **custom render graph nodes** and lean on 0.16+ GPU-driven batching.
5. Copy Veloren’s memory layout, CoW meshing, and coordinate-precision tricks.

---

## (B) Web Deployment Reality

### 1. WebGPU browser support

WebGPU is now broadly available but not universal.

| Browser / runtime | First stable version | Status (mid-2026) |
|---|---|---|
| Chrome / Edge | 113+ | Shipped, enabled by default |
| Firefox | 141+ Windows; 145+ Apple Silicon macOS 26+ | Linux/Android/Intel Mac still partial/flagged |
| Safari | 26+ (macOS/iOS/visionOS 26+) | Enabled by default |
| Samsung Internet | 24+ | Enabled by default |

Effective WebGPU reach (mid-2026):
- **caniuse** reports **~83.6%** global support (full + partial).
- **web3dsurvey** telemetry reports **77.4%** global support, with strong coverage on Windows (~86%), macOS (~87%), iOS (~79%), and Android (~73%), but very weak Linux coverage (~11%).

This means roughly **15–25% of users still cannot run WebGPU** and need a fallback.

### 2. Bevy web backend

- Bevy's default feature set on `wasm32` includes the `webgl2` feature, so **WebGL2 is the default web backend**.
- WebGPU support was introduced in Bevy 0.11 and is enabled with the `webgpu` Cargo feature. When enabled, it overrides WebGL2; the build requires WebGPU at runtime.
- In Bevy 0.19, `WgpuSettingsPriority::Compatibility` was renamed to `WgpuSettingsPriority::WebGPU`; set `WGPU_SETTINGS_PRIO=webgpu` to prefer WebGPU.
- A single WASM binary that supports both WebGL2 and WebGPU with runtime detection is **not yet implemented** (open issue [#13168](https://github.com/bevyengine/bevy/issues/13168), labeled `S-Ready-For-Implementation`). Today you must build separate binaries or choose one backend.
- Bevy 0.19 also ships **cancellable web tasks**, making `Task` behaviour consistent across native and WASM.

### 3. WASM threads and SharedArrayBuffer

- Bevy's WASM executor defaults to `SingleThreaded` on `wasm32`; `PipelinedRenderingPlugin` is excluded.
- WebAssembly threads require `SharedArrayBuffer`, which in turn requires the page to be cross-origin isolated:
  - `Cross-Origin-Opener-Policy: same-origin`
  - `Cross-Origin-Embedder-Policy: require-corp` (or `credentialless`)
- The Bevy CLI has an unstable `web-multi-threading` feature, but it only helps third-party crates (e.g. audio); it **does not enable Bevy's own multi-threaded scheduler** and requires nightly Rust.
- Long-term tracking: [bevyengine/bevy#4078](https://github.com/bevyengine/bevy/issues/4078).

If COOP/COEP headers are missing, `SharedArrayBuffer` is unavailable and threaded WASM fails; the single-threaded fallback still runs.

### 4. RAM / heap / storage-buffer / texture limits

Web budgets are far smaller than native. Default WebGPU minimums (from the spec/MDN):

| Limit | Default / minimum | Notes |
|---|---|---|
| `maxStorageBufferBindingSize` | 128 MiB | Enough for modest voxel chunk metadata; large worlds must stream |
| `maxBufferSize` | 256 MiB | Hard cap on a single buffer |
| `maxTextureDimension2D` | 8192 | Atlas size ceiling unless higher limits requested and granted |
| `maxTextureDimension3D` | 2048 | 3D texture / volume limits |
| `maxStorageBuffersPerShaderStage` | 8 | Very low compared to native; affects bindless-style voxel lookups |
| `maxSampledTexturesPerShaderStage` | 16 | Texture atlas / binding budget |
| `maxComputeWorkgroupStorageSize` | 16 KiB | Compute shared-memory budget |
| WASM / tab memory | ~2–4 GB practical | Browser tab budget, not just WASM heap |

Browsers may report tiered values to reduce fingerprinting; higher limits can be requested via `requiredLimits`, but rejection means fallback.

Voxel-specific implication: a dense 1024³ voxel world with 1 byte/voxel is 1 GB — impossible in browser. Streaming, chunk pooling, and greedy meshing are mandatory.

### 5. Shipped Bevy web titles

- **Bevy official examples** run in-browser via WASM/WebGL2, with a separate live WebGPU examples set.
- **Tiny Glade** is a confirmed native commercial Bevy title; it is not a browser-first WebGPU title.
- No authoritative public list of shipped, browser-only Bevy WebGPU AAA games was found. The web output in 2025–2026 is primarily engine examples, technical demos, jam games, and smaller indie releases.

### 6. Webview support for WebGPU (Tauri, Electron, wry, Edge WebView2)

`wry` (the Rust webview library used by Tauri) does not render WebGPU itself; it delegates to the OS webview:

| Platform | Engine | WebGPU status |
|---|---|---|
| Windows | WebView2 (Edge/Chromium) | Supported in runtime 113+; some cases still need `--enable-features=WebGPU` via `AdditionalBrowserArguments`. |
| macOS | WKWebView | Available on macOS 26+ (Tahoe), which ships Safari/WebKit WebGPU. |
| Linux | WebKitGTK (`webkit2gtk`) | Not reliably available; WPE WebKit is discussed as a future alternative but is not the current backend. |

- **Electron** bundles Chromium, so modern Electron versions generally support WebGPU out of the box (subject to the bundled Chromium version).

For a `wry`/Tauri plugin embedding Voxelforge, Windows WebView2 is viable, macOS 26+ is required, and Linux needs a WebGL/native fallback.

### 7. Actionable recommendations for the Voxelforge web tier

1. **Build two web artifacts** until runtime dual-backend lands: a WebGL2 build (max reach) and a WebGPU build (better performance/features). Serve WebGPU when `navigator.gpu` exists, else WebGL2.
2. **Default Bevy web build stays WebGL2** for reach; enable the `webgpu` feature only for the high-tier build.
3. **Serve COOP/COEP headers from day one** on any host that will ever use threading, even if the current build is single-threaded.
4. **Design for browser limits**: chunk streaming, greedy meshing, texture atlases ≤ 8192, storage buffers ≤ 128 MiB, and a 2–4 GB total budget.
5. **Webview embedding**: verify `navigator.gpu` in the host webview. On Windows pass `--enable-features=WebGPU` if needed. Target macOS 26+. Do not rely on Linux webview WebGPU before 2027.
6. **Lighting tier for web**: drop VBAO, Solari, and hardware ray tracing; use baked GI, reflection probes, SSR (WebGPU), and bloom/DOF.
7. **Single-threaded fallback**: ship a non-threaded WASM build and use it when cross-origin isolation cannot be guaranteed (e.g., simple static hosting, embedded iframes).

---

## (C) AAA Lighting in Bevy/wgpu

Data below reflects **Bevy 0.19** (released 2026-06-19).

| Feature | Native | Web | Notes |
|---|---|---|---|
| Deferred / Forward PBR | ✅ | ✅ (WebGPU) | `StandardMaterial` supports metallic/roughness, normal, occlusion, emissive, clearcoat, sheen, specular tint/map, parallax mapping |
| SSR (screen-space reflections) | ✅ | ⚠️ | **Built-in since Bevy 0.19** as “Physically Based Screen Space Reflections”; WebGPU only, not WebGL2 |
| Contact shadows | ✅ | ⚠️ | Built-in since 0.19; compute-based, WebGPU only |
| Rectangular area lights | ✅ | ⚠️ | Built-in since 0.19 |
| Lightmaps / reflection probes / parallax-corrected cubemaps | ✅ | ✅ | Baked GI is web-compatible |
| Bloom (incl. anamorphic) | ✅ | ✅ | Post-processing stack |
| DOF / motion blur / chromatic aberration / vignette / lens distortion | ✅ | ✅ | Post-processing stack; vignette & lens distortion added in 0.19 |
| Volumetric fog / volumetric lighting | ✅ | ✅ | Directional since 0.14, point/spot since 0.15 |
| Atmosphere | ✅ | ✅ | Physically-based sky in 0.16 |
| TAA | ✅ | ✅ | `TemporalAntiAliasing` |
| **VBAO** (Visibility Bitmask AO) | ✅ | ❌ | Replaced GTAO in 0.15; compute-based, not on WebGL2/WebGPU |
| **Solari** real-time ray-traced GI | ✅ (NVIDIA RTX only) | ❌ | Introduced 0.17; uses ReSTIR DI/GI + DLSS Ray Reconstruction → effectively NVIDIA-only via Vulkan on Windows/Linux |
| Volumetric clouds | ❌ | ❌ | Not built-in; needs custom raymarch or plugin |
| SSGI / dynamic GI | ❌ | ❌ | Needs custom render graph or third-party crate |

### Water reflections

Bevy has no built-in planar water-reflection system. Practical options:
- **Reflection probes** + PBR for static/specular water.
- **SSR** (now built-in) for dynamic screen-space reflections.
- A **custom render graph node** that renders the scene from below the water plane to a texture if you need true planar reflections.

### Path-traced GI on the web

- **No hardware ray-tracing API in WebGPU.** Solari’s DLSS-RR dependency makes it impossible on web.
- A **software BVH + compute-shader path tracer** is theoretically possible (e.g., [James Randall](https://www.jamesdrandall.com/posts/building-a-real-time-path-tracer-in-webgpu/), [gnikoloff/webgpu-raytracer](https://github.com/gnikoloff/webgpu-raytracer)), but it is heavy, noisy, and must fit within WebGPU storage limits.
- For a shipped web voxel game, use **baked GI (lightmaps, irradiance volumes, reflection probes) + SSAO + SSR + bloom/DOF**.

**Recommendation:**
- **Native AAA:** use built-in SSR, contact shadows, rectangular area lights, volumetrics, bloom/DOF/TAA. Add Solari **only** if you can require NVIDIA RTX; otherwise rely on baked GI + VBAO.
- **Web tier:** drop VBAO, Solari, and dynamic path-traced GI; use baked GI + SSR (WebGPU) + bloom/DOF/volumetric fog.

---

## (D) MMO Server-Authoritative Voxel Architecture

**There is no turnkey Bevy MMO voxel backend.** Design the architecture in layers from the start.

| Layer | Technology | Responsibility |
|---|---|---|
| **Client** | Bevy | Rendering, input, audio, client-side prediction, interpolation |
| **Shared simulation crate** | Plain Rust (no Bevy rendering) | Components, block types, math, deterministic rules used by both client and server |
| **Authoritative server** | Rust **outside** Bevy’s render ECS loop | Own world state, validate all voxel changes, enforce anti-cheat, broadcast deltas |
| **Networking** | UDP/QUIC/WebTransport + reliable deltas | Send only what changed, only to clients who need it |
| **Chunk storage** | RocksDB / Sled / Redb / SQLite | Persistent voxel data, chunk snapshots, hot/cold separation |
| **Interest management** | Spatial hash / octree | Limit network fan-out to nearby chunks/entities |

### Networking crates relevant to Bevy

| Crate | Role | Scale note |
|---|---|---|
| [`bevy_replicon`](https://github.com/projectharmonia/bevy_replicon) | Server-authoritative ECS replication, visibility/interest, remote events | Mature; transport-agnostic; good for medium-scale |
| [`lightyear`](https://github.com/cBournhonesque/lightyear) | Prediction, rollback, interpolation, snapshot interpolation, WASM/WebTransport | Higher-level; strong for fast-paced multiplayer |
| [`aeronet`](https://github.com/aecsocket/aeronet) | Low-level IO/transport abstraction (sessions, fragmentation, reliability) | Used by lightyear and via `aeronet_replicon` |
| [`renet`](https://github.com/lucaspoffo/renet) / `renet2` | UDP/netcode transport | Simple, no native WASM/WebTransport |
| [`bevy_rewind`](https://github.com/NiseVoid/bevy_rewind) | Server-authoritative rollback on top of `bevy_replicon` | Useful for physics rollback, expensive for whole-world rollback |
| [Bevygap](https://www.metabrew.com/article/bevygap-bevy-multiplayer-with-edgegap-and-lightyear) | Autoscaling headless Bevy servers with Edgegap + Lightyear | Deployment reference, not a game architecture |

### Reference: Veloren networking

- Server runs at **30 TPS**.
- Traffic split into **PING / CHUNK / GAMESTATE** channels; LZ4-compressed chunks.
- Stress test with **53 players averaged ~280 ms tick time**; message handling and server events were the largest costs.
- Default `max_players` is **100**, configurable.

This tells us that even a well-optimised Rust voxel server starts to strain below 100 players without careful event/chunk optimisation.

### “Engine layer on Bevy vs from scratch”

| Approach | When it wins | Risk |
|---|---|---|
| **Bevy for client + shared Rust crate** | Fast iteration, strong renderer, physics, ECS | Temptation to run the authoritative server inside Bevy’s ECS/render loop, coupling simulation to rendering |
| **From scratch** | Full control over memory, networking, determinism | Years of engine work before you can ship |

**Recommendation:** Use **Bevy on the client and a separate Rust authoritative server**. Share only simulation types and deterministic logic. Do not let Bevy’s rendering ECS drive server authority.

---

## Action Plan

1. **Native foundation:** Bevy 0.19 + `binary-greedy-meshing` + chunk LOD + baked GI + built-in SSR/contact shadows/rectangular area lights. Add Solari only if the target hardware is NVIDIA RTX.
2. **Web port:** Build **two artifacts** (WebGL2 reach + WebGPU quality), serve COOP/COEP, keep a single-threaded fallback, and use a reduced lighting tier (no VBAO, no Solari).
3. **Authoritative server:** Start a standalone Rust process from day one, sharing a `voxelforge-sim` crate with the client.
4. **Networking prototype:** Begin with `bevy_replicon` or `lightyear` for small-scale tests, but design chunk delta formats and interest management as if moving to a custom server.
5. **Plugin/webview embedding:** Delay until the web build is stable; verify the host webview’s WebGPU support before shipping.

---

## Sources

- [Bevy 0.19 Release Notes](https://bevy.org/news/bevy-0-19/) — primary, 2026-06-19
- [Bevy 0.16 Release Notes — GPU-Driven Rendering](https://bevy.org/news/bevy-0-16/) — primary, 2025-04
- [Bevy + WebGPU](https://bevy.org/news/bevy-webgpu/) — primary
- [Bevy Migration Guide: 0.18 → 0.19](https://bevy.org/learn/migration-guides/0-18-to-0-19/)
- [Unofficial Bevy Cheat Book: WASM](https://bevy-cheatbook.github.io/platforms/wasm.html)
- [Bevy WebGPU issue #13168 — runtime WebGL2/WebGPU selection](https://github.com/bevyengine/bevy/issues/13168)
- [Bevy WebGPU debug issue #18463](https://github.com/bevyengine/bevy/issues/18463)
- [web3dsurvey.com — WebGPU support stats](https://web3dsurvey.com/webgpu)
- [caniuse — WebGPU](https://caniuse.com/webgpu)
- [Stray Spark Studio — WebGPU and Browser-Based Indie Games in 2026](https://www.strayspark.studio/blog/webgpu-browser-indie-games-2026)
- [Stray Spark Studio — Bevy 0.18 in 2026](https://www.strayspark.studio/blog/bevy-rust-game-engine-2026-indie-guide)
- [binary-greedy-meshing](https://github.com/Inspirateur/binary-greedy-meshing)
- [binary_greedy_mesher_demo](https://github.com/TanTanDev/binary_greedy_mesher_demo)
- [block-mesh-rs](https://github.com/bonsairobo/block-mesh-rs)
- [bevy_meshem](https://github.com/Adamkob12/Meshem)
- [bevy_voxel_world](https://github.com/splashdust/bevy_voxel_world)
- [bevy_transvoxels_demo](https://github.com/ahmadaliadeel/bevy_transvoxels_demo)
- [drusniel-voxels-bevy](https://github.com/danielsobrado/drusniel-voxels)
- [voxelis](https://github.com/WildPixelGames/voxelis)
- [Veloren Performance Manual](https://book.veloren.net/players/performance.html)
- [Veloren Project Architecture](https://book.veloren.net/contributors/developers/codebase-structure.html)
- [Veloren Devblog 93 — Performance Analysis](https://veloren.net/blog/devblog-93)
- [Veloren Devblog 100 — Network Analysis](https://veloren.net/blog/devblog-100)
- [Virtual Geometry in Bevy 0.15 — JMS55](https://jms55.github.io/posts/2024-11-14-virtual-geometry-bevy-0-15/)
- [Realtime Raytracing in Bevy 0.17 (Solari) — JMS55](https://jms55.github.io/posts/2025-09-20-solari-bevy-0-17/)
- [Bevy 0.15 Release Notes — VBAO](https://bevyengine.org/news/bevy-0-15/)
- [James Randall — Real-Time Path Tracer in WebGPU](https://www.jamesdrandall.com/posts/building-a-real-time-path-tracer-in-webgpu/)
- [gnikoloff/webgpu-raytracer](https://github.com/gnikoloff/webgpu-raytracer)
- [bevy_replicon](https://github.com/projectharmonia/bevy_replicon)
- [lightyear](https://github.com/cBournhonesque/lightyear)
- [aeronet](https://github.com/aecsocket/aeronet)
- [renet](https://github.com/lucaspoffo/renet)
- [bevy_rewind](https://github.com/NiseVoid/bevy_rewind)
- [Bevygap — autoscaling Bevy multiplayer](https://www.metabrew.com/article/bevygap-bevy-multiplayer-with-edgegap-and-lightyear)
- [Rapier Determinism Guide](https://rapier.rs/docs/user_guides/rust/determinism/)
- [Gabriel Gambetta — Fast-Paced Multiplayer](https://www.gabrielgambetta.com/client-side-prediction-server-reconciliation.html)
- [wry Linux WebGPU discussion](https://github.com/tauri-apps/wry/discussions/996)
- [Edge WebView2 release notes](https://learn.microsoft.com/en-us/microsoft-edge/web-platform/release-notes/139)

---

*Researched by Sahara. Updated 2026-07-25.*
