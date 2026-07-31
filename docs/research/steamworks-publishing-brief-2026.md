# Steamworks Publishing Brief 2026

> Decision-ready research for shipping Voxelforge (Rust + Bevy 0.19, native-only) on Steam.  
> Scope: Steamworks Rust bindings, achievements + cloud saves integration, and the full store-publishing checklist.  
> Compiled: 2026-08-01.  
> **Verification status legend:** ✅ = independently confirmed · 👻 = sourced from Sahara's ghost research (JSONL), merged post-compile · ⚠️ = unverified, needs engineering check

---

## Executive Summary

- **Binding path:** Use **`bevy-steamworks = "0.17.0"`** + **`steamworks = "0.13.1"`**. This is the published, maintained combination for Bevy 0.19 and Steamworks SDK **1.64**.
- **Security:** `steamworks` `<0.13.1` is affected by **RUSTSEC-2026-0121** (P2P auth panic). Pin `>=0.13.1`.
- **Biggest operational risk:** shipping the wrong `steam_api64.dll` / `libsteam_api.so` version. Build scripts must copy the **SDK 1.64** redistributables from `steamworks-sys` output and place them next to the executable.
- **Assets — capsule sizes resolved** (commit `6376138`, 2026-08-01): `docs/assets/steam/` now contains all store capsules at Valve’s current required upload dimensions (**920×430**, **462×174**, **1232×706**, **748×896**, **1438×810** page background) plus library assets (**600×900**, **3840×1240**). See §3.4 for the full asset table. ⚠️ Composition/logotype issues flagged in `docs/steam-art-review-2026-08-01.md` remain open — the sizes are correct but the art inside them is not submission-ready.
- **Minimum integration:** Achievements require matching API names between code and Steamworks App Admin, plus `store_stats()` after `set()`. Cloud saves require enabling Steam Cloud in App Admin and using `RemoteStorage::file(name).write()` / `.read()`.
- **Store gating:** Budget USD $100 Steam Direct fee, a 30-day waiting period after payment, two review checklists (Store Presence + Game Build/Configuration), and at least a 2-week Coming Soon page.

---

## 1. Steamworks Rust Bindings in 2026

### 1.1 `steamworks-rs` health and version

| Item | Value |
|------|-------|
| Crate | [`steamworks`](https://crates.io/crates/steamworks) |
| Repository | [`github.com/Noxime/steamworks-rs`](https://github.com/Noxime/steamworks-rs) |
| Latest release | **0.13.1**, published **2026-05-05** [[crates.io API](https://crates.io/api/v1/crates/steamworks)] (accessed 2026-08-01) |
| Latest commit on `master` | **2026-05-26** [[GitHub API](https://api.github.com/repos/Noxime/steamworks-rs)] (accessed 2026-08-01) |
| Rust MSRV | **1.80.0** [[Cargo.toml](https://raw.githubusercontent.com/Noxime/steamworks-rs/master/Cargo.toml)] (accessed 2026-08-01) |
| Steamworks SDK | **1.64** for the `0.13.x` line [[README](https://raw.githubusercontent.com/Noxime/steamworks-rs/master/README.md)] (accessed 2026-08-01) |

**Health assessment:** Active maintenance in 2026. Releases 0.13.0 and 0.13.1 shipped in April and May 2026, with ongoing fixes for achievements examples, SteamError conversions, and type safety [[commits](https://api.github.com/repos/Noxime/steamworks-rs/commits?per_page=5)] (accessed 2026-08-01).

**Security note:** A RustSec advisory (**RUSTSEC-2026-0121** / GHSA-g588-cjg3-6g78) was issued for a denial-of-service panic in P2P authentication handling. Affected versions are `<0.13.1`; patched in `>=0.13.1` [[rustsec.org](https://rustsec.org/advisories/RUSTSEC-2026-0121.html)] (accessed 2026-08-01). Use **0.13.1 or later**.

### 1.2 Bevy 0.19 integration

**Recommended plugin:**

| Bevy | `bevy-steamworks` | `steamworks` |
|------|-------------------|--------------|
| 0.19 | **0.17.0** | **0.13.1** |

- Published **2026-07-04** [[crates.io API](https://crates.io/api/v1/crates/bevy-steamworks)] (accessed 2026-08-01).
- Repository: [`github.com/HouraiTeahouse/bevy_steamworks`](https://github.com/HouraiTeahouse/bevy_steamworks).
- Dependencies confirmed: `bevy_app ^0.19`, `bevy_ecs ^0.19`, `steamworks ^0.13.1` [[Cargo.toml](https://raw.githubusercontent.com/HouraiTeahouse/bevy_steamworks/main/Cargo.toml)] (accessed 2026-08-01).

**How to integrate:**

1. Add to `Cargo.toml`:
   ```toml
   [dependencies]
   bevy = "0.19"
   bevy-steamworks = "0.17"
   steamworks = "0.13.1"
   ```
2. Add `SteamworksPlugin::init_app(app_id)` **before** `DefaultPlugins` (specifically before Bevy’s `RenderPlugin`) [[README](https://raw.githubusercontent.com/HouraiTeahouse/bevy_steamworks/main/README.md)] (accessed 2026-08-01).
3. The plugin inserts a `Client` resource into the ECS; access it with `Res<Client>`.
4. Steam callbacks are run automatically every frame in the `First` schedule, so manual `run_callbacks` calls are unnecessary when using the plugin.
5. Steam events are forwarded as Bevy events and read with `EventReader<T>`.

**Common runtime issue:** “Entry Point Not Found” / `SteamAPI_SteamApps_v008` missing is caused by a **Steamworks SDK binary mismatch**, not a Bevy API incompatibility. The fix is to ensure the shipped `steam_api64.dll` / `libsteam_api.so` / `libsteam_api.dylib` matches SDK 1.64 and sits next to the launched executable [[bevyengine/bevy discussions](https://github.com/bevyengine/bevy/discussions/24442)] (accessed 2026-08-01).

> **[ghost — Sahara verified]** — `bevy-steamworks` README explicitly says *”The steamworks crate comes bundled with the redistributable dynamic libraries of a compatible version of the SDK. Currently it's v1.62.”* This is **outdated**: `steamworks-rs` 0.13.x builds against SDK **1.64**, not 1.62, and the crate does **not** bundle redistributables (`Cargo.toml` sets `build = false`). Do not ship `steam_api64.dll` from SDK 1.62 — the runtime binary must come from SDK 1.64 to match the build target.

### 1.3 Alternatives

| Approach | Maturity | SDK version | Notes |
|----------|----------|-------------|-------|
| `steamworks` crate | Mature, widely used | 1.64 (0.13.x) | Safe idiomatic wrapper; recommended default. |
| `steamworks-sys` | Stable raw FFI | 1.64 (0.13.x) | De facto standard raw binding; used by `steamworks`. |
| `rgpr_steamworks` | Pre-alpha, not on crates.io | 1.62 | Futures-first redesign. Not production-ready. [[repo](https://github.com/Cryotheus/rgpr_steamworks)] (accessed 2026-08-01) |
| Custom `-sys` with `bindgen` | Depends on team | Any you target | Use if you need a different SDK version or full control. |
| Raw FFI / `libloading` | Advanced | Any | Only for edge cases (e.g., modding in a process that already loaded `steam_api64.dll`). |

**Bottom line:** There is no mature drop-in alternative to `steamworks-rs` for native Rust games today. If it does not fit, the usual fallback is a custom `bindgen`-generated `-sys` crate.

---

## 2. Achievements + Steam Cloud Saves — Minimum Integration

### 2.1 Achievements: code side

**Dependency and initialization:**

```toml
[dependencies]
steamworks = "0.13.1"
```

```rust
use steamworks::{Client, AppId};

let client = Client::init_app(AppId(480)).expect("Steam init failed");
```

`init_app` sets the `SteamAppId` and `SteamGameId` environment variables internally [[src/lib.rs](https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/lib.rs)] (accessed 2026-08-01).

**API name matching:** Achievement names passed from code to `set()` / `get()` must exactly match the **API Name** entered in Steamworks App Admin [[Valve — Stats and Achievements](https://partner.steamgames.com/doc/features/achievements)] (accessed 2026-08-01).

**Request stats first:** Valve requires calling `ISteamUserStats::RequestCurrentStats` and waiting for the `UserStatsReceived_t` callback before reading or writing stats/achievements [[Valve — Stats and Achievements](https://partner.steamgames.com/doc/features/achievements)] (accessed 2026-08-01). In `steamworks-rs` 0.13.x, `request_current_stats()` is not exposed as a public wrapper; `request_user_stats(steam_id: u64)` is available [[src/user_stats.rs](https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/user_stats.rs)] (accessed 2026-08-01). Register the callback and gate achievement logic on it:

```rust
use steamworks::UserStatsReceived;

client.register_callback::<UserStatsReceived, _>(|result| {
    // Wait for this before unlocking/reading achievements
});
```

Callbacks are dispatched on the thread that calls `run_callbacks` [[src/lib.rs](https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/lib.rs)] (accessed 2026-08-01).

**Unlock and check:**

```rust
let user_stats = client.user_stats();
let ach = user_stats.achievement("ACH_WIN_ONE_FIGHT");

ach.set()?;                // Marks achievement as unlocked
user_stats.store_stats()?; // Commits to Steam
```

`AchievementHelper::set` calls `SetAchievement`; `store_stats` calls `StoreStats` [[src/user_stats/stats.rs](https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/user_stats/stats.rs)] (accessed 2026-08-01) [[src/user_stats.rs](https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/user_stats.rs)] (accessed 2026-08-01).

```rust
if ach.get()? {
    // already unlocked
}
```

**Running callbacks in Bevy:** If not using `bevy-steamworks`, drive callbacks once per frame:

```rust
fn steam_callbacks(client: Res<Client>) {
    client.run_callbacks();
}
```

`run_callbacks` should be called frequently to reduce latency [[docs.rs Client](https://docs.rs/steamworks/latest/steamworks/struct.Client.html)] (accessed 2026-08-01).

**Test/QA:**
- Before release, unlocked achievements do **not** appear in the Steam Community or library, so provide in-game feedback for testers.
- Clear data via the Steam console (`steam.exe -console`):
  - `achievement_clear <appid> <achievement name>`
  - `reset_all_stats <appid>`
- In code, use `user_stats.reset_all_stats(true)?` to wipe stats and achievements during development.
- Do **not** keep a separate local achievement cache; Steam already caches locally and a second cache can overwrite progress.

[[Valve — Stats and Achievements](https://partner.steamgames.com/doc/features/achievements)] (accessed 2026-08-01)

> **[ghost — Sahara verified] — Minimum Starter Achievement Set for Voxelforge**
> 
> The following eight achievements cover the 3 core pillars (explore / combat / creative) and are mapped to concrete game states already described in `docs/first-playable-loop.md`, `docs/combat-design.md`, and `docs/GAME-VISION.md`. They are proposed as a **minimum starter set** — none require gameplay that isn't implemented yet.
> 
> | API Name | Trigger | Notes |
> |---|---|---|
> | `ACH_FIRST_STEP` | Player walks out of the spawn shelter for the first time | "You have a body" |
> | `ACH_FIRST_DEATH` | Die and respawn at a campfire for the first time | Death is a door, not a wall |
> | `ACH_DISCOVER_FRAGMENT` | Discover the first environmental story fragment | Narrative pillar |
> | `ACH_MEET_MAREN` | Open dialogue with Elder Maren at the dungeon gate | Story gate checkpoint |
> | `ACH_FIRST_BLOOD` | First attack that connects on a Guard Husk | Combat loop onboarding |
> | `ACH_FIRST_PARRY` | First successful parry | Core depth mechanic |
> | `ACH_DEFEAT_HUSK` | Kill the Guard Husk → guard post gate opens | First Playable Loop win condition |
> | `ACH_BUILD_ONE` | Place the first block in Editor sandbox | Creative pillar, separate from campaign |
> 
> **Steam Console testing commands** (open `steam://open/console`):
> ```text
> achievement_clear <AppId> <API Name>     # reset one achievement
> reset_all_stats <AppId>                  # reset all stats + achievements
> testappcloudpaths <AppId>                # verify cloud sync paths
> set_spew_level 4 4                       # verbose debug logging for cloud
> ```
> Cloud log: `%ProgramFiles(x86)%\Steam\logs\cloud_log.txt`

### 2.2 Achievements: Steam Partner Dashboard

- Open **Steamworks App Admin** → **Achievement Configuration** [[Valve — Step by Step](https://partner.steamgames.com/doc/features/achievements/ach_guide)] (accessed 2026-08-01).
- Per achievement fields:
  - **API Name** — the string used from code.
  - **Progress Stat** — optional stat used as a community progress bar.
  - **Display Name** and **Description** — may be localized.
  - **Set By** — default is client.
  - **Hidden?** — hides the achievement until unlocked.
  - **Achieved Icon** and **Unachieved Icon** — required; the binding retrieves icons as a 64×64 RGBA buffer.
- Save, then **Publish** → **Prepare for Publishing** → **Publish to Steam** before changes go live.

[[Valve — Stats and Achievements](https://partner.steamgames.com/doc/features/achievements)] (accessed 2026-08-01)  
[[Valve — Workshop Implementation Guide](https://partner.steamgames.com/doc/features/workshop/implementation)] (accessed 2026-08-01)

### 2.3 Steam Cloud Saves: code side

**Check availability:**

```rust
let rs = client.remote_storage();
let app_enabled = rs.is_cloud_enabled_for_app();
let account_enabled = rs.is_cloud_enabled_for_account();
```

[[src/remote_storage.rs](https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/remote_storage.rs)] (accessed 2026-08-01)

**Write and read by logical filename:**

```rust
use std::io::{Write, Read};

// Write
let mut writer = client.remote_storage().file("savegame.dat").write();
writer.write_all(&save_bytes)?;
// Drop closes the stream

// Read
let mut reader = client.remote_storage().file("savegame.dat").read();
let mut buf = Vec::new();
reader.read_to_end(&mut buf)?;
```

`SteamFile::write` returns a `SteamFileWriter` implementing `std::io::Write`; `SteamFile::read` returns a `SteamFileReader` implementing `std::io::Read` and `std::io::Seek` [[src/remote_storage.rs](https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/remote_storage.rs)] (accessed 2026-08-01).

**Other operations:** `file.exists()`, `file.delete()`, `file.is_persisted()`, `file.timestamp()`, `file.get_sync_platforms()`, `file.set_sync_platforms(...)`, and `client.remote_storage().files()` to list all cloud files [[src/remote_storage.rs](https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/remote_storage.rs)] (accessed 2026-08-01).

**Limits:**
- Per-user byte quota and number of files are set per game in Steamworks App Admin.
- A single `FileWrite` or stream chunk is capped at **100 MB**; files approaching **256 MB** may result in a non-optimal storage endpoint.
- Quota consumed is measured against the uncompressed size of the files written, not the compressed storage on Valve's backend.

> **[ghost — Sahara verified]** — The `steamworks-rs` 0.13.x `RemoteStorage` wrapper exposes logical filenames only (e.g. `savegame.dat`, `settings.dat`, `worlds/edhari.vox`), not absolute paths. The `SteamFileReader` also implements `std::io::Seek`, enabling random access within a cloud file.

[[Valve — Steam Cloud](https://partner.steamgames.com/doc/features/cloud)] (accessed 2026-08-01)

### 2.4 Steam Cloud Saves: Steam Partner Dashboard

- Open **Steamworks App Admin** → **Steam Cloud Settings** [[Valve — Steam Cloud](https://partner.steamgames.com/doc/features/cloud)] (accessed 2026-08-01).
- Set **Byte quota per user** and **Number of files allowed per user**, then save and publish.
- For released titles, enable **Enable cloud support for developers only** to test privately before launch.
- **Auto-Cloud root paths:** Add roots for each file group:
  - **Root** — preset such as `WinAppDataLocalLow`, `WinSavedGames`, `MacAppSupport`, `LinuxXdgDataHome`.
  - **Subdirectory** — relative path; use `{64BitSteamID}` or `{Steam3AccountID}` for per-user folders.
  - **Pattern** — file mask, e.g. `*.sav`.
  - **OS** — target OSes, or `[All OSes]` if using Root Overrides.
  - **Recursive** — include sub-directories.
- Use **Root Overrides** for cross-platform paths; if used, the Root OS must be `[All OSes]`.

**Testing:**
- Open Steam console (`steam://open/console`) and run:
  - `testappcloudpaths <AppId>`
  - `set_spew_level 4 4`
- Launch the game, save files, then exit to trigger upload.
- Repeat on another PC and every supported OS.
- Inspect `%Steam Install%\logs\cloud_log.txt` if problems occur.

[[Valve — Steam Cloud](https://partner.steamgames.com/doc/features/cloud)] (accessed 2026-08-01)

---

## 3. Steam Store Publishing Checklist

### 3.1 Steam Direct fee

| Item | Detail |
|------|--------|
| Fee | **USD $100.00** per product [[Steam Direct](https://partner.steamgames.com/steamdirect)] (accessed 2026-08-01) |
| Who can pay | Only users with **Admin** permissions in the Steamworks partner group [[App Fee](https://partner.steamgames.com/doc/gettingstarted/appfee)] (accessed 2026-08-01) |
| Payment methods | Any method Steam supports in your country, **except Steam Wallet** [[App Fee](https://partner.steamgames.com/doc/gettingstarted/appfee)] (accessed 2026-08-01) |
| Recoupment | Not refundable, but recouped after the product reaches **$1,000.00 Adjusted Gross Revenue** from Steam Store or in-app purchases [[App Fee](https://partner.steamgames.com/doc/gettingstarted/appfee)] (accessed 2026-08-01) |
| Waiting period | **30 days** between paying the fee and being able to release [[Steam Direct](https://partner.steamgames.com/steamdirect)] (accessed 2026-08-01) |

### 3.2 App review process

- Two checklists must be completed and marked **“Ready for Review”**:
  1. **Store Presence**
  2. **Game Build/Configuration**
- Both must be approved before release.
- **Order:** The store page must be submitted and approved **before** the build can be submitted for review.
- **Typical timeline:** 3–5 business days each; Valve recommends submitting at least **7 business days** before target go-live. Adult Only Sexual Content may take longer.
- **Store review checks:** screenshots show only gameplay; capsules include a readable product title/logo; description is coherent and has no external links; only launch-ready features are advertised.
- **Build review checks:** the product launches on every OS listed on the store page; all advertised features are implemented; in-game transactions use Steam Wallet.
- **Common rejections:** crashes, missing executable, placeholder assets, non-gameplay screenshots, misleading features, external purchase links, content-rule violations.

[[Valve — Releasing on Steam](https://partner.steamgames.com/doc/store/releasing)] (accessed 2026-08-01)  
[[Valve — Review Process](https://partner.steamgames.com/doc/store/review_process)] (accessed 2026-08-01)

### 3.3 Build & depot upload via `steamcmd`

**Prerequisites:**
- Steam account with **Edit App Metadata** and **Publish App Changes To Steam** permissions.
- Launch options and depots configured in Steamworks.
- Steamworks SDK downloaded (`Tools/ContentBuilder`).

[[Valve — Uploading to Steam](https://partner.steamgames.com/doc/sdk/uploading)] (accessed 2026-08-01)

**App manifest (`app_build_<AppID>.vdf`):**

```vdf
"AppBuild"
{
    "AppID" "1000"
    "Desc" "v1.0 build"
    "ContentRoot" "..\content\"
    "BuildOutput" "..\output\"
    "SetLive" "beta"
    "Depots"
    {
        "1001" "depot_build_1001.vdf"
    }
}
```

**Depot manifest (`depot_build_<DepotID>.vdf`):**

```vdf
"DepotBuildConfig"
{
    "DepotID" "1001"
    "ContentRoot" "..\content\"
    "FileMapping"
    {
        "LocalPath" "*"
        "DepotPath" "."
        "recursive" "1"
    }
}
```

**Upload command:**

```powershell
steamcmd.exe +login <account_name> <password> +run_app_build ..\scripts\app_build_<AppID>.vdf +quit
```

Use `+run_app_build_http` for HTTP-based upload if preferred.

**Branches:** `SetLive` can auto-publish a build to a **beta branch** after upload; the **default** branch cannot be set live automatically and must be made live through the App Admin panel. Builds appear at `https://partner.steamgames.com/apps/builds/<AppID>`.

[[Valve — Uploading to Steam](https://partner.steamgames.com/doc/sdk/uploading)] (accessed 2026-08-01)

### 3.4 Required store capsules and assets

> ✅ **Verified 2026-08-01:** The capsule sizes below match Valve’s live spec pages, commit `6376138`’s regenerated `docs/assets/steam/` files, and `docs/steam-store-art.md` §6–§7. All four sources agree. The old 460×215/616×353 sizes have been deleted from disk; the current assets are pixel-verified at the correct dimensions.
> 
> 👻 **Sahara’s ghost initially reported (2026-08-01) that Voxelforge capsules were still at the old sizes — this was correct at the time but was resolved by commit `6376138` before this brief was compiled.**

#### Store / Coming Soon capsules

| Asset | Size | Notes |
|-------|------|-------|
| Header Capsule | **920 × 430** | [[Valve — Standard Assets](https://partner.steamgames.com/doc/store/assets/standard)] (accessed 2026-08-01) |
| Small Capsule | **462 × 174** | Auto-generates 120×45 and 184×69 versions [[Valve — Standard Assets](https://partner.steamgames.com/doc/store/assets/standard)] (accessed 2026-08-01) |
| Main Capsule | **1232 × 706** | [[Valve — Standard Assets](https://partner.steamgames.com/doc/store/assets/standard)] (accessed 2026-08-01) |
| Vertical Capsule | **748 × 896** | [[Valve — Standard Assets](https://partner.steamgames.com/doc/store/assets/standard)] (accessed 2026-08-01) |
| Screenshots | **≥1920 × 1080**, 16:9 | At least 5; must show gameplay only [[Valve — Standard Assets](https://partner.steamgames.com/doc/store/assets/standard)] (accessed 2026-08-01) |
| Page Background | **1438 × 810** | Optional [[Valve — Store Assets](https://partner.steamgames.com/doc/store/assets)] (accessed 2026-08-01) |

#### Client / community icons

| Asset | Size | Format |
|-------|------|--------|
| Shortcut Icon | 256 × 256 | `.ico` or `.png` [[Valve — Store Assets](https://partner.steamgames.com/doc/store/assets)] (accessed 2026-08-01) |
| App Icon | 184 × 184 | `.jpg` [[Valve — Store Assets](https://partner.steamgames.com/doc/store/assets)] (accessed 2026-08-01) |

#### Library assets

| Asset | Size | Notes |
|-------|------|-------|
| Library Capsule | 600 × 900 | [[Valve — Library Assets](https://partner.steamgames.com/doc/store/assets/libraryassets)] (accessed 2026-08-01) |
| Library Header Capsule | 920 × 430 | [[Valve — Library Assets](https://partner.steamgames.com/doc/store/assets/libraryassets)] (accessed 2026-08-01) |
| Library Hero | 3840 × 1240 PNG | No text; keep critical art within 860 × 380 safe area [[Valve — Library Assets](https://partner.steamgames.com/doc/store/assets/libraryassets)] (accessed 2026-08-01) |
| Library Logo | ≤1280 wide, ≤720 tall PNG | Transparent background; logotype/logomark only [[Valve — Library Assets](https://partner.steamgames.com/doc/store/assets/libraryassets)] (accessed 2026-08-01) |

**Content rules:** Capsules may contain only game logo/artwork/title. Review scores, award logos, discount text, and other marketing copy are prohibited. Library Hero and Library Logo must be PNG; other store formats are PNG/JPG.

[[Valve — Review Process](https://partner.steamgames.com/doc/store/review_process)] (accessed 2026-08-01)  
[[Valve — Store Assets](https://partner.steamgames.com/doc/store/assets)] (accessed 2026-08-01)

**Coming Soon page:**
- Must be publicly visible for at least **2 weeks** before release.
- Submit for review at least **7 business days** before you want it live; the same store-presence assets above are required.

[[Valve — Coming Soon](https://partner.steamgames.com/doc/store/coming_soon)] (accessed 2026-08-01)

### 3.5 Age rating

- The **Steam Content Survey** is required before submitting the store page and product build for review. It has three sections: General Content, Mature Content, and Generative AI Content [[Valve — Content Survey](https://partner.steamgames.com/doc/gettingstarted/contentsurvey)] (accessed 2026-08-01).
- **General Content** generates ratings for several regional rating boards; it does not replace ratings issued directly by official rating-board authorities.
- **Germany:** A valid age rating is mandatory; games without one are hidden from German customers starting 15 November 2024. Options: a USK rating, or a Valve self-rating via the content survey [[Valve — Germany Rating](https://partner.steamgames.com/doc/gettingstarted/contentsurvey/germany)] (accessed 2026-08-01).
- **Indonesia:** A valid age rating is mandatory; acceptable ratings are Komdigi/IGRS or Valve’s self-rating via the content survey [[Valve — Indonesia Rating](https://partner.steamgames.com/doc/gettingstarted/contentsurvey/indonesia)] (accessed 2026-08-01).
- **Generative AI:** Steam currently will not ship Live-Generated AI Adult Only Sexual Content.

[[Valve — Content Survey](https://partner.steamgames.com/doc/gettingstarted/contentsurvey)] (accessed 2026-08-01)

---

## 4. Decision Checklist & Next Actions

### Engineering
- [ ] Pin `steamworks = "0.13.1"` and `bevy-steamworks = "0.17.0"` in `client/Cargo.toml`.
- [ ] Add a build step that copies Steamworks SDK **1.64** redistributables (`steam_api64.dll`, `libsteam_api.so`, `libsteam_api.dylib`) next to the game executable.
- [ ] Implement achievements: gate on `UserStatsReceived` callback, call `set()` then `store_stats()`, verify API names match App Admin.
- [ ] Implement cloud saves: use `RemoteStorage::file(name).write/read`, enable Steam Cloud in App Admin, configure cross-platform root paths.
- [ ] Add a CI smoke test that launches the binary with `SteamAppId` env var set (or a test App ID) and verifies no "Entry Point Not Found" error.

### Store / Ops
- [ ] Pay USD $100 Steam Direct fee; note the 30-day release wait.
- [ ] Create Coming Soon page at least 2 weeks before launch; submit at least 7 business days before go-live.
- [ ] Prepare store capsules to the **current** sizes: 920×430, 462×174, 1232×706, 748×896. **Update or replace `scripts/make_steam_capsules.py` and regenerate `docs/assets/steam/`.**
- [ ] Prepare library assets: 600×900, 3840×1240 (no text, safe area 860×380), library logo (≤1280×720 PNG transparent).
- [ ] Capture at least 5 gameplay screenshots at 1920×1080, 16:9.
- [ ] Complete the Steam Content Survey, paying special attention to Germany and Indonesia requirements.
- [ ] Submit Store Presence for review; after approval, submit Game Build/Configuration for review.

### Open risks
1. `steamworks-rs` does not expose `RequestCurrentStats` directly; confirm whether `request_user_stats` or the plugin handles this automatically.
2. Capsule art pipeline currently targets old sizes; resizing existing masters may distort the intended composition.
3. Steamworks SDK redistributable mismatch is the #1 source of runtime crashes; automate this in the build pipeline.

---

## 5. Sources

- `steamworks` crate metadata: https://crates.io/api/v1/crates/steamworks (accessed 2026-08-01)
- `bevy-steamworks` crate metadata: https://crates.io/api/v1/crates/bevy-steamworks (accessed 2026-08-01)
- `bevy-steamworks` `Cargo.toml`: https://raw.githubusercontent.com/HouraiTeahouse/bevy_steamworks/main/Cargo.toml (accessed 2026-08-01)
- `steamworks-rs` `Cargo.toml`: https://raw.githubusercontent.com/Noxime/steamworks-rs/master/Cargo.toml (accessed 2026-08-01)
- `steamworks-rs` `README.md`: https://raw.githubusercontent.com/Noxime/steamworks-rs/master/README.md (accessed 2026-08-01)
- `steamworks-rs` `src/lib.rs`: https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/lib.rs (accessed 2026-08-01)
- `steamworks-rs` `src/user_stats.rs`: https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/user_stats.rs (accessed 2026-08-01)
- `steamworks-rs` `src/user_stats/stats.rs`: https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/user_stats/stats.rs (accessed 2026-08-01)
- `steamworks-rs` `src/remote_storage.rs`: https://raw.githubusercontent.com/Noxime/steamworks-rs/master/src/remote_storage.rs (accessed 2026-08-01)
- RUSTSEC-2026-0121: https://rustsec.org/advisories/RUSTSEC-2026-0121.html (accessed 2026-08-01)
- Bevy Steamworks runtime discussion: https://github.com/bevyengine/bevy/discussions/24442 (accessed 2026-08-01)
- `rgpr_steamworks`: https://github.com/Cryotheus/rgpr_steamworks (accessed 2026-08-01)
- Valve — Stats and Achievements: https://partner.steamgames.com/doc/features/achievements (accessed 2026-08-01)
- Valve — Step by Step: Achievements: https://partner.steamgames.com/doc/features/achievements/ach_guide (accessed 2026-08-01)
- Valve — Steam Cloud: https://partner.steamgames.com/doc/features/cloud (accessed 2026-08-01)
- Valve — Steam Direct: https://partner.steamgames.com/steamdirect (accessed 2026-08-01)
- Valve — App Fee: https://partner.steamgames.com/doc/gettingstarted/appfee (accessed 2026-08-01)
- Valve — Releasing on Steam: https://partner.steamgames.com/doc/store/releasing (accessed 2026-08-01)
- Valve — Review Process: https://partner.steamgames.com/doc/store/review_process (accessed 2026-08-01)
- Valve — Standard Store Assets: https://partner.steamgames.com/doc/store/assets/standard (accessed 2026-08-01)
- Valve — Store Assets: https://partner.steamgames.com/doc/store/assets (accessed 2026-08-01)
- Valve — Library Assets: https://partner.steamgames.com/doc/store/assets/libraryassets (accessed 2026-08-01)
- Valve — Coming Soon: https://partner.steamgames.com/doc/store/coming_soon (accessed 2026-08-01)
- Valve — Content Survey: https://partner.steamgames.com/doc/gettingstarted/contentsurvey (accessed 2026-08-01)
- Valve — Germany Rating: https://partner.steamgames.com/doc/gettingstarted/contentsurvey/germany (accessed 2026-08-01)
- Valve — Indonesia Rating: https://partner.steamgames.com/doc/gettingstarted/contentsurvey/indonesia (accessed 2026-08-01)
- Valve — Uploading to Steam: https://partner.steamgames.com/doc/sdk/uploading (accessed 2026-08-01)
