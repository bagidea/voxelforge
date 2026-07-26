# Voxelforge — MMO Netcode Architecture

> **Verdict in one sentence:** Build a standalone, authoritative Rust game server from day one, share only deterministic simulation logic with the Bevy client, and pick a transport that can serve both native (QUIC/UDP) and web (WebTransport) clients without rewriting the protocol later.

This doc locks the decisions that are expensive to reverse: the server-authoritative split, the shared-sim crate boundary, the transport/interest-management strategy, and the persistence layer. It extends the stack decision in [`decision-brief-voxel-stack.md`](./decision-brief-voxel-stack.md).

---

## 1. Authoritative Rust Game Server

### 1.1 Server role
The server is the only machine that can mutate world state. It validates every voxel edit, resolves movement/collision, owns inventory and economy state, and publishes deltas to clients. The client is a **predicted, interpolated viewer**: it simulates ahead for responsiveness, but the server’s word is final.

### 1.2 Tick rate

| Game / reference | Tick rate | Notes |
|---|---|---|
| **Minecraft** | 20 TPS (50 ms/tick) | Default; TPS drops if MSPT > 50 ms ([rb_bb2814e1]) |
| **Veloren** | 30 TPS (~33 ms/tick) | Stress test at 53 players averaged ~280 ms/tick before network refactor ([rb_5b13fb2a]) |
| **Valorant** | 128 TPS (~7.8 ms/tick) | Competitive FPS; uses prediction + interpolation + lag compensation ([rb_74c5c842]) |

**Decision for Voxelforge:**
- **Authoritative simulation: 30 TPS** for world/physics/voxel edits. This matches Veloren’s proven baseline, gives a 33 ms budget, and is forgiving for voxel physics that do not need FPS-grade latency.
- **Input collection / anti-cheat validation: 64–128 Hz** if needed for combat/aim validation, but decoupled from the world tick so it does not destabilize the 33 ms budget.
- **Render / presentation: unlocked** on the client, with interpolation between fixed simulation states.

**Why this must be locked now:** Changing tick rate after launch rewinds the entire movement feel, physics tuning, animation timing, and network buffering. Pick one and build every system around it.

### 1.3 World partitioning & interest management

Voxel worlds generate enormous fan-out if every player receives every block change. Use a **chunk-based area-of-interest (AOI)** model:

| Layer | Data structure | Purpose |
|---|---|---|
| **Chunk grid** | Dense or sparse `Chunk(x,y,z)` | Primary unit of loading, persistence, and streaming |
| **Spatial hash / chunk map** | `HashMap<ChunkCoord, ChunkState>` | Fast lookup of loaded chunks and active players |
| **Player AOI** | Radius around player in chunk coords | Determine which chunks/entities to replicate |
| **Entity relevance set** | Per-player `HashSet<EntityId>` | Track which entities are currently visible to each player |

**Rules to lock:**
1. **Interest radius:** Start with a modest radius (e.g., view distance of 8–12 chunks) and make it configurable per shard. Vast view distances are the #1 bandwidth killer.
2. **Relevance set delta:** Send spawn/despawn/change events only when an entity enters or leaves the player’s AOI. Do not resend full state every tick.
3. **Chunk LOD authority:** The server owns the authoritative chunk data. It may send simplified LOD snapshots for distant chunks; the client must not edit LOD-only data.
4. **Horizontal sharding by region:** A single shard should not simulate the whole planet. Partition the world into regions/cells, each handled by one server process. Seamless handoff between shards is a later optimization, but the **chunk-coordinate addressing scheme must be global from day one** so regions can be reassigned without renaming data.

**Why this must be locked now:** AOI radius, chunk size, and coordinate addressing are baked into save files, protocol messages, and client chunk streaming. Retrofitting sharding requires renaming/rebalancing the world.

### 1.4 Persisting chunk edits

| Backend | Best for | Caveat |
|---|---|---|
| **RocksDB** | Batch writes, compaction, huge worlds | FFI dependency, operational complexity |
| **redb** | Pure Rust, fast individual writes, simple API | Larger disk footprint than RocksDB |
| **Sled** | Pure Rust, ergonomic | Slower in benchmarks, less active maintenance |
| **SQLite** | Tooling, relational metadata | Slower random reads/writes for bulk chunk data |
| **LMDB** | Read-heavy, concurrent reads | Memory-mapped, tricky write concurrency |

Benchmark summary from the redb project (Ryzen 9950X3D + NVMe): RocksDB wins on batch-write throughput and compacted size; redb wins on individual writes; LMDB wins on read concurrency ([rb_4d689904]).

**Decision for Voxelforge:**
- **Primary hot store: RocksDB** for modified chunk data. It handles high write throughput, compression, and very large worlds.
- **Metadata & player data: SQLite** (or redb) for accounts, inventories, and region bookkeeping where relational queries matter.
- **Cold archive: object storage** (S3-compatible) for snapshots, rollback history, and region backups.
- **Chunk snapshot strategy:** Save incremental deltas on a timer (e.g., every 30 s for active chunks) and full snapshots on chunk unload. Keep a write-ahead journal for crash recovery.

**Why this must be locked now:** The chunk storage schema (key format, delta encoding, compression) is the hardest thing to migrate after players have built structures. Choose the format and write path before the first public build.

### 1.5 Realistic shard capacity

Comparable baselines:

| Reference | Concurrent players / shard | Notes |
|---|---|---|
| Minecraft vanilla | 20–100 | Highly dependent on redstone/entities/view distance |
| Veloren (pre-refactor) | ~50 before severe lag | 30 TPS, single-threaded message handling bottleneck ([rb_5b13fb2a]) |
| Veloren (post refactor) | ~100 | Split channels, removed duplicate serialization ([rb_ae4f1bc2]) |
| MultiPaper (clustered) | 100–250 | Academic benchmark with multiple nodes ([source in decision-brief]) |

**Target for Voxelforge (v0.1 server):**
- **Stable: 100 concurrent players per shard** on a single high-core server, assuming 30 TPS, 8–10 chunk view radius, and moderate voxel edit rate.
- **Stretch: 250 players** after horizontal region sharding and network optimization.
- **Do not promise one giant world with thousands of players on a single process.** That requires SpatialOS-style distributed simulation, which is not in scope.

**Why this must be locked now:** Capacity targets drive every downstream decision — tick budget, AOI radius, DB write rate, and shard topology. Over-promising early forces a rewrite later.

### 1.6 Anti-cheat & validation

The server must validate every client action before applying it:

| Action | Validation |
|---|---|
| **Voxel edit** | Range check, tool ownership/durability, block registry, permissions, rate limit |
| **Movement** | Speed cap, physics replay, collision against authoritative chunk state |
| **Inventory use** | Server-authoritative; client may preview but server resolves |
| **Combat / hit** | Lag compensation by rewinding relevant entities to the client’s firing tick |

Keep validation deterministic and in the shared crate so the client prediction can mirror it exactly.

---

## 2. Shared-Simulation Crate (`shared`)

### 2.1 What belongs where

| Domain | Crate | Reason |
|---|---|---|
| Block registry & properties | `shared` | Must be identical on client and server |
| Voxel edit rules | `shared` | Client predicts; server authorizes using the same code |
| Chunk data structure & delta encoding | `shared` | Shared serialization and checksums |
| Movement / collision integration | `shared` | Predicted on client, replayed on server |
| Deterministic RNG | `shared` | Same seed discipline and algorithm |
| Fixed timestep & tick scheduling | `shared` | Keeps client and server in sync |
| Inventory data model | `shared` | Shared predicates; authority lives on server |
| Rendering, audio, windowing | `client` | No Bevy render/audio in `shared` |
| Mesh baking, GPU upload | `client` | Presentation only |
| Camera, UI, particles | `client` | Presentation only |
| Network transport, auth, DB writes | `server` | Authority and I/O |
| Persistence / rollback logs | `server` | Authority and I/O |

**Boundary rule:** If two binaries must produce the *same result* from the same tick + inputs, it lives in `shared`. If it only displays or only authorizes, it lives in the respective binary.

### 2.2 Determinism requirements

| Source of non-determinism | Fix |
|---|---|
| `f32` / `f64` math | Use fixed-point (`fixed` crate) or deterministic float paths; never rely on transcendental functions |
| Parallel system ordering | Use explicit `.chain()` in `FixedUpdate`; do not rely on default ambiguous ordering |
| `rand::thread_rng()` | Use `rand_chacha::ChaCha8Rng` with explicit seeds and streams ([agent shared-sim report]) |
| `Instant::now()` | Drive time from a `SimTick` resource |
| Physics engine | If using Rapier, enable `enhanced-determinism` and disable `parallel`/`simd-stable` ([rb_136e677d]); Avian is ECS-native and uses Bevy fixed timestep |
| Change detection | Do not use `Changed<T>` for one-shot gameplay logic during rollback/replay |

### 2.3 Client-side prediction, reconciliation, interpolation

1. **Local player:** client runs `shared` movement immediately, stores predicted states in a ring buffer by tick, sends inputs to server.
2. **Server:** simulates inputs authoritatively, returns the processed tick + authoritative state.
3. **Reconciliation:** on mismatch, the client snaps/lerps to the server state and replays unacknowledged inputs forward ([rb_23a4b9af]).
4. **Remote players:** interpolate between the two latest server snapshots; do not predict.
5. **Voxel edits:** client predicts the edit; server validates and sends a corrected delta on mismatch. Large edits (explosions) should be server-first with streamed results.

### 2.4 Crate structure (Cargo workspace)

```
voxelforge/
├── Cargo.toml
├── shared/              # deterministic sim core
│   ├── Cargo.toml       # default-features = false, no bevy render/audio
│   └── src/
│       ├── blocks.rs
│       ├── world.rs     # chunk storage
│       ├── physics.rs   # movement/collision rules
│       ├── inventory.rs
│       ├── rng.rs
│       ├── net/         # messages, input schemas
│       └── schedule.rs
├── client/              # Bevy rendering + prediction
└── server/              # headless authoritative server
```

`shared` should depend only on `bevy_ecs`, `bevy_app`, `bevy_time`, `bevy_math`/`bevy_reflect`, and a fixed-point/math crate — **no rendering or audio**.

### 2.5 Serialization

- Use `serde` derive schemas.
- For wire format, prefer `bitcode` (small, fast, serde-compatible) or `bincode 2.x`.
- Send **actions and deltas** over the wire, not full voxel state every tick.
- For late-join / chunk snapshots: delta encoding + bitwise block arrays + zstd compression.

---

## 3. Transport: WebTransport/QUIC vs WebSocket

### 3.1 Requirements

- **Native clients:** low-latency UDP-like semantics, reliability where needed, no head-of-line blocking.
- **Web clients:** must work inside a browser with COOP/COEP isolation headers; WebTransport is the modern path, WebSocket is the fallback.

### 3.2 Protocol comparison

| Transport | Pros | Cons |
|---|---|---|
| **WebTransport over QUIC** | Runs over HTTP/3/QUIC; unreliable datagrams + reliable streams; no HOL blocking; connection migration; lower latency than TCP ([aeronet notes]) | Requires browser support; needs TLS/certificate config; COOP/COEP still required for WASM threads |
| **WebSocket** | Universally supported; easy to proxy; works through many firewalls | Built on TCP; head-of-line blocking; higher baseline latency |
| **Raw UDP/netcode** | Fastest for native; used by renet | Does not work in browser; needs separate web path |

**Browser reality:** WebTransport reached baseline support in 2026 ([rb_e5a9be1a]); WASM threads still require `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` ([rb_0919c0d5]).

### 3.3 Crate comparison

| Crate | Latest | Bevy 0.19 | Transport | Scope | Best for |
|---|---|---|---|---|---|
| **Lightyear** | 0.28.0 (2026-06-26) | ✅ ([rb_6fd0df71], [rb_cb6eaf69]) | WebTransport, WebSocket, UDP/netcode | Full server-authoritative networking: prediction, rollback, snapshot interpolation, interest management | Voxelforge’s recommended high-level library |
| **bevy_replicon** | 0.41.1 (2026-06-24) | ✅ ([rb_207e9907]) | None built-in; plug in aeronet/renet/lightyear | ECS replication, visibility, remote events | Good if you want to compose your own transport |
| **aeronet** | 0.21.0 (2026-06-24) | ✅ ([rb_9b8f3f2d], [rb_dcb5eb08]) | WebTransport/QUIC, WebSocket, Steam (WIP) | Low-level IO + reliability/fragmentation; **not** replication | Rock-solid primitives; use as IO layer under replicon/lightyear |
| **renet** | 2.0.0 (2026-01-20) | via `bevy_renet` 5.0.0 ([rb_79e7cce4]) | UDP/netcode, Steam | Reliable UDP messaging, channels, auth | Native-only; no web/WASM path |
| **renet2** | 0.16.0 (2026-07-11) | Unknown | UDP/netcode (fork) | Fork of renet | Same native limitation |

### 3.4 Recommendation

**Decision for Voxelforge:**
- **Primary transport stack: Lightyear 0.28 + aeronet WebTransport IO.**
  - Lightyear gives us client-side prediction, rollback, snapshot interpolation, and server authority out of the box.
  - Aeronet gives us a proven WebTransport/QUIC + WebSocket IO layer that works on native and WASM.
- **Fallback:** Lightyear’s WebSocket transport for browsers/networks that block QUIC.
- **Do not start with renet if web is a target.** Renet is UDP-only and has no browser path; adding web later would require a second protocol.

**Why this must be locked now:** The transport choice determines the message protocol, serialization assumptions, connection model, and whether the web client can share the same networking code. Picking a native-only crate today and adding web later is a rewrite.

---

## 4. Locked Decisions

| Decision | Choice | Cost of changing later |
|---|---|---|
| **Authority model** | Standalone Rust server, server-authoritative | Rewriting client trust model and anti-cheat |
| **Tick rate** | 30 TPS world sim | Re-tuning physics, animation, networking buffers |
| **Shared-sim boundary** | `shared` crate: block rules, movement, collision, RNG, tick | Massive desync bugs if logic drifts between binaries |
| **Interest management** | Chunk-based AOI, configurable radius | Save-file/protocol breakage, bandwidth crisis |
| **Chunk storage** | RocksDB hot + SQLite metadata + S3 cold | Migration of player-built worlds |
| **Shard capacity target** | 100 stable / 250 stretch | Over-promising forces distributed-sim rewrite |
| **Transport** | Lightyear + aeronet WebTransport, WebSocket fallback | Two protocols, two netcodes, two sets of bugs |
| **Physics determinism** | Fixed-point math + deterministic RNG + ordered systems | Cross-platform desync, irreproducible replays |

---

## 5. Open Questions (not locked yet)

1. **Region sharding handoff:** How do players move between shards seamlessly? Defer until 100-player shard is proven.
2. **Economy / NFT bridge:** If Voxelforge ties to waxwing-style assets, the on-chain bridge belongs in the server layer but is out of scope for this doc.
3. **Web build details:** Single-threaded fallback, reduced lighting tier — see `decision-brief-voxel-stack.md`.

---

## Sources

- [Lightyear GitHub README](https://github.com/cBournhonesque/lightyear) — [rb_6fd0df71]
- [crates.io API: lightyear](https://crates.io/api/v1/crates/lightyear) — [rb_cb6eaf69]
- [Aeronet GitHub README](https://github.com/aecsocket/aeronet) — [rb_9b8f3f2d]
- [crates.io API: aeronet](https://crates.io/api/v1/crates/aeronet) — [rb_dcb5eb08]
- [Renet GitHub README](https://github.com/lucaspoffo/renet) — [rb_2427f53b]
- [crates.io API: renet](https://crates.io/api/v1/crates/renet) — [rb_cf2bcfdc]
- [crates.io API: bevy_renet](https://crates.io/api/v1/crates/bevy_renet) — [rb_79e7cce4]
- [crates.io API: bevy_replicon](https://crates.io/api/v1/crates/bevy_replicon) — [rb_207e9907]
- [crates.io API: renet2](https://crates.io/api/v1/crates/renet2) — [rb_a2ba3156]
- [Veloren Devblog 93 — Performance Analysis](https://veloren.net/blog/devblog-93) — [rb_5b13fb2a]
- [Veloren Devblog 100 — Network Analysis](https://veloren.net/blog/devblog-100) — [rb_ae4f1bc2]
- [Veloren Server Configuration](https://book.veloren.net/players/server-hosting/configuration.html) — [rb_5f2124b6]
- [Minecraft Wiki — Tick Rate](https://minecraft.wiki/w/Tick_rate) — [rb_bb2814e1]
- [WebGPU Hits Critical Mass](https://www.webgpu.com/news/webgpu-hits-critical-mass-all-major-browsers/) — [rb_e5a9be1a]
- [Bevy Cheat Book — WASM / COOP/COEP](https://bevy-cheatbook.github.io/platforms/wasm.html) — [rb_0919c0d5]
- [redb GitHub — benchmarks](https://github.com/cberner/redb) — [rb_4d689904]
- [Riot Games — Peeking Valorant's Netcode](https://www.riotgames.com/en/news/peeking-valorants-netcode) — [rb_74c5c842]
- [Gabriel Gambetta — Fast-Paced Multiplayer](https://www.gabrielgambetta.com/client-side-prediction-server-reconciliation.html) — [rb_23a4b9af]
- [Rapier Determinism Guide](https://rapier.rs/docs/user_guides/rust/determinism/) — [rb_136e677d]
- [bevy_replicon README](https://github.com/projectharmonia/bevy_replicon) — [rb_6e848afb]

---

*Researched by Sahara. Written 2026-07-25.*
