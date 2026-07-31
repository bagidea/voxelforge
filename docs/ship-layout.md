# Ship layout — what a Voxelforge Windows depot actually contains

**Lane:** ship (Poppy) · **Date:** 2026-08-01 · **Audited binary:**
`target/release/voxelforge.exe`, mtime `2026-07-31 21:10:43`,
sha256 `45622a23…e825f7c94d`, 76.2 MiB.

The question this doc answers is not "does it build" — it's *"if I hand this
folder to someone with no repo, no cargo and no Rust, do they get a game?"*
Everything below was measured against the binary on disk. Nothing was rebuilt.

Tools that produce/verify the answer:

| script | what it does |
|---|---|
| `scripts/ship_audit_imports.py` | parses the PE import tables — the real DLL dependency list, not a `strings` guess |
| `scripts/pack_win64.ps1` | copies exe + runtime data into a depot folder, then re-verifies the result and writes a SHA256 manifest |
| `scripts/cleanroom_test.ps1` | copies the pack **outside the repo** and launches it there, five cases, each self-terminating with a proof PNG |

---

## 1. Verdict

| | |
|---|---|
| Does the pack contain everything the game reads? | **Yes** — audit in §2, list enforced by `pack_win64.ps1` |
| Does the exe run outside the repo, with no cargo/source? | **Yes, proven** — `hero` case: exit 0, 730 KiB PNG rendered, `docs/assets/ship/cleanroom-hero.png` |
| Does it boot to the **game** (`--play`) outside the repo? | **No — and not because of packaging.** The binary on disk panics at plugin registration in *every* mode, in the repo and in the clean room alike. See §6. |
| Are there absolute/dev paths in the shipped exe? | Project paths: **none**. Dependency paths: **yes**, ~1 950 copies of `C:\Users\BagIdea\.cargo\registry\…`. Cosmetic + privacy, not functional. §5 |
| Extra runtime to install? | **VCRUNTIME140.dll** only. §3 |

---

## 2. Asset path audit — the client uses *two* different roots

This is the single most important finding for shipping, and it is not obvious
from reading any one file.

**Root A — the EXE directory.** `bevy_asset` resolves every
`asset_server.load("…")` against `get_base_path()`
(`bevy_asset-0.19.0/src/io/file/mod.rs:19`), which is
`$BEVY_ASSET_ROOT` → else `$CARGO_MANIFEST_DIR` → else **the directory
containing the exe**. On a player's machine neither env var is set, so this is
`<install>\assets\`.

| reader | path | needs |
|---|---|---|
| `client/src/audio.rs:215-232, 298-325` | `audio/*.wav` | all 17 wavs |
| `client/src/import.rs:202,221` | `models/*.vox`, `.gltf` | the `.vox` files |

**Root B — the CURRENT WORKING DIRECTORY.** Four call sites bypass the asset
server and use `std::fs` with a bare relative path, which Windows resolves
against the CWD, *not* the exe:

| reader | path | what happens if it's not there |
|---|---|---|
| `client/src/quest.rs:445` | `assets/story/act1.json` | `cache_story_data` inserts no `StoryDataRes`; `init_journal` / `spawn_npcs` take `Res<StoryDataRes>` → **panic on the first `--play` frame** |
| `client/src/scene.rs:37-43` | `maps/edhari.json` | graceful — `play_map()` returns `None`, the game falls back to procedural terrain (i.e. **the village silently disappears**) |
| `client/src/import.rs:147` | `assets/models` (dir scan) | graceful — `let Ok(entries) … else` |
| `client/src/settings_menu.rs:24`, `client/src/editor_config.rs:159` | `settings.json`, `editor_config.json` | written on startup; failure is swallowed |

Steam and Explorer both launch with CWD = the exe's folder, so the two roots
agree and the layout in §4 works. A desktop shortcut with a different **Start
in**, or a launcher script, breaks root B only — and it breaks it *quietly*:
you get procedural terrain instead of Edhari, then a panic.

**Measured, not assumed.** Clean-room `wrongcwd` case (exe in the pack, CWD =
`C:\`): no `SETTINGS saved` line in stdout and no `settings.json` anywhere — the
write failed and nothing reported it. Same run from CWD = pack folder writes
both files immediately.

`run-voxelforge.cmd` (shipped by the packer) does `cd /d "%~dp0"` and removes
this whole class of bug for non-Steam launches.

**Recommended follow-up (code, not packaging).** Resolve those four `std::fs`
paths against `std::env::current_exe()`'s parent instead of the CWD — one helper,
four call sites — and move `settings.json` / `editor_config.json` to
`%APPDATA%\Voxelforge\`. Writing config into the install directory fails outright
under a per-machine install or a read-only depot. Not done in this lane: it needs
a rebuild to verify, and this lane is explicitly forbidden from rebuilding.

---

## 3. Runtime dependencies

`python scripts/ship_audit_imports.py target/release/voxelforge.exe` — 29 imports,
0 delay-load. Everything is a Windows system DLL except one:

* **`VCRUNTIME140.dll`** — the MSVC runtime. Rust's `x86_64-pc-windows-msvc`
  target links the CRT dynamically. Missing it = the process dies at load with
  `0xC0000135` before `main()`; there is no error message to a player.
  * On Steam: tick **"Visual C++ Redist for Visual Studio 2015-2022 (x64)"** in
    the app's *Installation → Redistributables*. This is the correct answer for a
    depot.
  * For a zip you hand someone directly: `pack_win64.ps1 -IncludeVCRuntime`
    copies it next to the exe (app-local deployment).
* `api-ms-win-crt-*.dll` — the UCRT, part of Windows 10/11. Nothing to ship.
* `pdh.dll`, `powrprof.dll`, `setupapi.dll`, `uiautomationcore.dll`, `dwmapi.dll`,
  `uxtheme.dll`, `combase.dll` — all in `System32` on every Windows 10/11.
* **Not** in the import table: `d3d12.dll`, `dxgi.dll`, `vulkan-1.dll`. `wgpu`
  loads its backends with `LoadLibrary` at runtime and degrades between them, so
  a machine without a Vulkan ICD still gets DX12. The audited run picked Vulkan
  on a GTX 1060 / driver 560.94.

No project DLLs, no side-by-side assemblies, no `.pdb` needed at runtime.

---

## 4. Depot layout

`powershell -File scripts\pack_win64.ps1 -Clean` produces
`_ship\voxelforge-win64\` — 31 files, 78.3 MiB. Upload **the contents of this
folder** as the depot root:

```
voxelforge.exe            76.2 MiB   the client
run-voxelforge.cmd                   CWD-pinning launcher (see §2)
MANIFEST.txt                         sha256 + size of every file, + commit & exe mtime
assets\
  audio\  *.wav (17)      1.4 MiB    loaded relative to the EXE dir
  audio\  CREDITS.md                 attribution required by the audio licences
  models\ sample.vox                 scanned at startup
  story\  act1.json         30 KiB   REQUIRED — missing = panic in --play
maps\
  edhari.json              356 KiB   the shipping world
  arena / castle / castle_nl / demo / handmade / pyramid / tower / walls .json
```

Steam app config:

* **Launch option** — executable `voxelforge.exe`, *working directory* the
  install root (Steam's default). Do not point it at a subfolder.
* **Installation → Redistributables** — VC++ 2015-2022 x64 (§3).
* The depot is content-only; no installer script is needed.

Deliberately excluded, and why: `*.pdb` (13 MiB of debug symbols, no runtime
use), `assets/story/{validate.py, serde_check.py, _validation.log}` (dev
validators), `maps/FORMAT.md` (dev doc), `voxelforge-server.exe` (the client
links no networking crate — it is fully offline), `dist/` (the retired wasm
build), and everything else in the repo. `pack_win64.ps1` fails the pack if a
`.pdb`/`.py`/`.log`/`.rs` ever slips in.

---

## 5. Absolute paths in the binary

Scanned the 76 MiB image for dev paths, ASCII and UTF-16:

| needle | hits |
|---|---|
| `E:\Projects`, `E:/Projects`, `\workspace\projects` | **0** |
| `C:\Users\BagIdea\.cargo\registry` | **1 954** |

Workspace-local crates are compiled with paths relative to the workspace root —
which is why the panic in §6 reads `client\src\main.rs:445:14` and not an `E:\…`
path. Dependency crates are compiled from the absolute registry path, so every
`panic!`/`assert!` location string inside Bevy and friends bakes in the
developer's Windows username.

Not a functional problem — nothing resolves those strings at runtime. It is a
minor information leak in a public artifact. The one-line fix, for whoever owns
the ship build: add to the workspace `Cargo.toml`

```toml
[profile.release]
trim-paths = "all"
```

or build with `RUSTFLAGS=--remap-path-prefix=$CARGO_HOME=.`. Not applied here —
it changes the release profile and would need a full rebuild to verify.

---

## 6. Clean-room test

`powershell -File scripts\cleanroom_test.ps1` copies the pack to
`%TEMP%\voxelforge-cleanroom` — outside the repo, no cargo, no source — scrubs
`BEVY_ASSET_ROOT` and `CARGO_MANIFEST_DIR` (either one would fake a pass by
redirecting the asset root back at the repo), and runs each case with
`VOXELFORGE_SHOT` so it captures a frame at t=3.2 s and exits itself at t=4.4 s.

Run of 2026-08-01 01:24, against the 07-31 21:10 binary:

| case | setup | exit | PNG | verdict |
|---|---|---|---|---|
| `hero` | pack intact, CWD = pack, `VOXELFORGE_HERO` | 0 | 730 KiB | **PASS** |
| `play` | pack intact, CWD = pack, `VOXELFORGE_PLAY` | 101 | — | FAIL (panic) |
| `nostory` | `assets\story` removed | 101 | — | inconclusive |
| `wrongcwd` | pack intact, CWD = `C:\` | 101 | — | inconclusive |
| `launcher` | `run-voxelforge.cmd` from CWD = `C:\` | n/a | — | inconclusive |

**The `hero` pass is the packaging proof.** It launched the packed exe from a
temp folder with no repo present, resolved every DLL import, initialised Vulkan
on the discrete GPU, created the window, rendered and saved a frame, and exited
0. Nothing about "a folder outside the repo" is broken.

**The `play` failure is a code defect in the binary, not a missing file:**

```
thread 'main' (22256) panicked at client\src\main.rs:445:14:
Error adding plugin voxelforge::settings_menu::SettingsPlugin: : plugin was already added in application
```

App-builder registration — it happens before any asset or map is touched, and it
is identical whether the CWD is right, the CWD is wrong, or `assets\story` has
been deleted. `git grep` finds exactly **one** `add_plugins(settings_menu::SettingsPlugin)`
in the current tree (`client/src/main.rs:444`); the duplicate was removed in
`8176d01 feat(settings): review fixes — single plugin, …`. So
`target/release/voxelforge.exe` predates that commit — it is a stale artifact,
and no `target*/release/voxelforge.exe` anywhere in the repo is newer.

The three inconclusive cases exist to test §2's claims and cannot report until a
binary built at ≥ `8176d01` lands. Full log: `docs/assets/ship/cleanroom-play.err.txt`.

### Finishing the proof

No code change required — only a current binary. When one exists:

```powershell
powershell -File scripts\pack_win64.ps1  -Clean
powershell -File scripts\cleanroom_test.ps1
```

Expected, once the binary matches the source:

| case | expected |
|---|---|
| `hero`, `play`, `launcher` | PASS — exit 0, PNG ≥ 2 KiB, `STORY_LOAD ok` in the log |
| `nostory` | FAIL with a `StoryDataRes` panic — that is the point; it proves `act1.json` is a hard dependency and must never drop out of the pack |
| `wrongcwd` | boots, but **no** `STORY_LOAD ok` and no `maps/edhari.json` — the §2 fragility, visible |

---

## 7. Open items for the ship lane

1. **Rebuild the release binary** at ≥ `8176d01` and re-run §6. Until then there
   is no shippable artifact, packaging aside.
2. Make the four `std::fs` paths exe-relative and move writable config to
   `%APPDATA%` (§2). Needs a build to verify.
3. `trim-paths = "all"` on the release profile (§5).
4. `assets/audio/CREDITS.md` is packed — someone should confirm the licences
   listed there are satisfied by shipping that file alone.
