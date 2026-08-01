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

| | | evidence |
|---|---|---|
| Does the pack contain everything the game reads? | **Inferred, not proven** — the list comes from a static read of every `asset_server.load` / `std::fs` call site (§2) and is enforced by `pack_win64.ps1`. No run has yet opened a packed data file. | static audit only |
| Does the exe run outside the repo, with no cargo/source? | **Yes, proven** — `hero` case: exit 0, 730 KiB PNG rendered | `docs/assets/ship/cleanroom-hero.png` |
| Does it boot to the **game** (`--play`) outside the repo? | **Unproven.** The binary on disk panics at plugin registration in *every* mode, in the repo and in the clean room alike — a stale-artifact defect, not a packaging one. §6 | `cleanroom-play.err.txt` |
| Are there absolute/dev paths in the shipped exe? | Project paths: **none**. Dependency paths: **yes**, ~1 950 copies of `C:\Users\BagIdea\.cargo\registry\…`. Cosmetic + privacy, not functional. §5 | byte scan of the image |
| Extra runtime to install? | **VCRUNTIME140.dll** only. §3 | PE import table |

**Read the first row carefully.** The `hero` control case passes, but
`client/src/hero.rs` contains zero `asset_server` calls (`git grep -c asset_server
client/src/hero.rs` → no match), so it opened none of the 31 packed files. The
riskiest claim in this document — *"with no repo present, bevy resolves its asset
root to the exe directory and finds all 17 wavs plus the `.vox`"* — follows from
`bevy_asset`'s `get_base_path()` source, not from a measurement. `cleanroom_test.ps1`
now greps every run for `Path not found` / `AssetReaderError` and fails the case on
a hit, so the `play` case will settle it the moment a current binary exists (§6).

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

**Root B — the CURRENT WORKING DIRECTORY.** Seven call sites bypass the asset
server and use `std::fs` (or `Path::exists`) with a bare relative path, which
Windows resolves against the CWD, *not* the exe:

| reader | path | what happens if it's not there |
|---|---|---|
| `client/src/quest.rs:446` | `assets/story/act1.json` | `cache_story_data` inserts no `StoryDataRes`; `init_journal` / `spawn_npcs` take `Res<StoryDataRes>` → **panic on the first `--play` frame** |
| `client/src/scene.rs:37-44` (`Path::exists`) → `main.rs:1029` | `maps/edhari.json` | graceful — `play_map()` returns `None`, the game falls back to procedural terrain (i.e. **the village silently disappears**) |
| `client/src/import.rs:151` | `assets/models` (dir scan) | graceful — `let Ok(entries) … else` |
| `client/src/settings_menu.rs:154,175` (`SETTINGS_PATH`, l.24) | `settings.json` | read at startup, written on every change; failure is swallowed |
| `client/src/editor_config.rs:172,199` (`CONFIG_PATH`, l.159) | `editor_config.json` | same — read at startup, written on change, error only `eprintln!`d |
| `client/src/main.rs:1020,1023` (`save_world_to`) | `maps/quicksave.json` — `Editor.map_path`, `main.rs:452` | **writes into the install folder.** F5/quick-save and the editor Save button both land here |
| `client/src/main.rs:1029` (`load_map_file`) | same | F9 quick-load reads it back from the CWD |

The last two matter more than they look: the mode a bare double-click currently
opens *is* the editor (see the `--play` note below), so `maps/quicksave.json` is
the first file a confused player writes — into `C:\Program Files\…\Steam\steamapps\`
if the CWD happens to be right, and into whatever folder the shortcut points at if
it isn't. Under a per-machine install that write fails outright and the only
report is a line on stderr nobody sees.

**Game mode is an argument, not a default.** `read_cfg` (`client/src/main.rs:153`)
sets `play` only from the `--play` flag or `VOXELFORGE_PLAY`. Everything else —
double-click, a Steam launch option with no arguments, a shortcut — boots the
**editor sandbox on procedural terrain**. Both the shipped `run-voxelforge.cmd`
and the Steam launch option in §4 therefore pass `--play` explicitly.

Steam and Explorer both launch with CWD = the exe's folder, so the two roots
agree and the layout in §4 works. A desktop shortcut with a different **Start
in**, or a launcher script, breaks root B only — and it breaks it *quietly*:
you get procedural terrain instead of Edhari, then a panic.

**Measured, not assumed.** Clean-room `wrongcwd` case (exe in the pack, CWD =
`C:\`): no `SETTINGS saved` line in stdout and no `settings.json` anywhere — the
write failed and nothing reported it. Same run from CWD = pack folder writes
both files immediately.

`run-voxelforge.cmd` (shipped by the packer) does `cd /d "%~dp0"` and then calls
`voxelforge.exe --play %*`, removing both this class of bug and the wrong-mode
bug for non-Steam launches. It deliberately does **not** wrap the call in
`start`: the client is a console-subsystem binary, so `start` would give it a
second console and detach its stdout — which is how the clean-room `launcher`
case used to come back with an empty log every time.

**Measured (2026-08-01 01:47), with a stub `voxelforge.exe` that just prints its
argv and CWD** — the launcher was invoked from `CWD = C:\` with no arguments and
no environment:

```
argv=[--play]
cwd=C:\Users\BagIdea\AppData\Local\Temp\vf-launcher-proof

; and with an extra argument appended:
run-voxelforge.cmd --combat-demo  ->  argv=[--play --combat-demo]
```

**Recommended follow-up (code, not packaging).** Resolve the read-only `std::fs`
paths against `std::env::current_exe()`'s parent instead of the CWD — one helper,
three call sites — and move the four *writable* ones (`settings.json`,
`editor_config.json`, `maps/quicksave.json`, and whatever the editor's Save
dialog produces) to `%APPDATA%\Voxelforge\`. Writing into the install directory
fails outright under a per-machine install or a read-only depot. A better default
for `play` would remove the argument dependency entirely, but that is a product
call, not a packaging one. Not done in this lane: each needs a rebuild to verify.

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
run-voxelforge.cmd                   launcher: passes --play, pins CWD (see §2)
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

* **Launch option** — executable `voxelforge.exe`, **arguments `--play`**,
  *working directory* the install root (Steam's default). Do not point it at a
  subfolder.
  * The arguments field is not optional. Without it the Play button opens the
    editor sandbox on procedural terrain (`read_cfg`, §2) — the store page
    promises a village, the player gets a grey box.
  * `run-voxelforge.cmd` is the equivalent for a desktop shortcut, a zip you
    hand a playtester, or an itch build: it passes `--play` itself and pins the
    CWD, so it needs neither field set correctly by the person launching it.
    Any extra arguments you give the `.cmd` are forwarded.
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

**The harness never sets `VOXELFORGE_PLAY`.** It scrubs it, and game mode has to
arrive the way it does for a player: `--play` on the command line for the cases
that stand in for the Steam launch option, and *nothing at all* for the
`launcher` case, which exists precisely to prove `run-voxelforge.cmd` supplies
the flag by itself. An earlier revision exported the env var before every case;
that would have passed a depot whose launcher boots the editor.

A case passes only with exit 0 **and** a ≥2 KiB PNG **and** no panic **and** no
`Path not found` / `AssetReaderError` / `MissingAssetLoader` in the log. Those
three needles come from `bevy_asset`'s error text and have not yet been seen fire
on this project — the first real `play` run should be read by eye once, to
confirm bevy words a missing wav the way the grep expects. `STORY_LOAD ok` is reported
separately as the game-mode witness — `quest.rs:449` prints it only in `--play`,
so a green PNG with `StoryOk=False` means the editor booted and the harness says
so out loud.

### Harness self-test (2026-08-01 01:47)

The harness was run against a fake pack whose `voxelforge.exe` is a 4 KiB stub
that prints its argv, CWD and the two mode env vars, so what each case *actually
delivers to the process* is on the record rather than assumed:

| case | argv the exe saw | CWD | `VOXELFORGE_PLAY` |
|---|---|---|---|
| `hero` | *(empty)* | room | `<unset>` (`VOXELFORGE_HERO=1`) |
| `play` | `--play` | room | `<unset>` |
| `nostory` | `--play` | room | `<unset>` |
| `wrongcwd` | `--play` | `C:\` | `<unset>` |
| `launcher` | `--play` | room | `<unset>` |

The `launcher` row is the point: it was started from `C:\` with no arguments and
no environment, and the exe still received `--play` with the CWD moved to the
install folder — so the `.cmd` did both jobs on its own. All five cases produced
a captured log (the pre-fix `start`-based launcher produced none), and the
`STORY_LOAD` warning fired for `play` and `launcher` exactly as designed, because
a stub never prints it. Pass is `False` everywhere: a stub renders no PNG. This
run tests the harness, not the game.

Run of 2026-08-01 01:24, against the 07-31 21:10 binary (harness revision that
still exported `VOXELFORGE_PLAY`; the arguments column shows what the current
revision sends instead):

| case | setup | now sends | exit | PNG | verdict |
|---|---|---|---|---|---|
| `hero` | pack intact, CWD = pack | `VOXELFORGE_HERO` (unchanged) | 0 | 730 KiB | **PASS** |
| `play` | pack intact, CWD = pack | `--play` argument | 101 | — | FAIL (panic) |
| `nostory` | `assets\story` removed | `--play` argument | 101 | — | inconclusive |
| `wrongcwd` | pack intact, CWD = `C:\` | `--play` argument | 101 | — | inconclusive |
| `launcher` | `run-voxelforge.cmd` from CWD = `C:\` | nothing — the `.cmd` does it | n/a | — | inconclusive |

**What the `hero` pass proves, and what it does not.** It launched the packed exe
from a temp folder with no repo present, resolved every DLL import, initialised
Vulkan on the discrete GPU, created the window, rendered and saved a frame, and
exited 0. So the *binary* is self-contained. It does **not** validate the packed
data: `hero.rs` has no `asset_server` call, so that run read none of the 30 data
files the packer copied. Every claim in §2 about which files the game needs is
still static analysis, and stays that way until `play` runs green.

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
| `hero` | PASS — exit 0, PNG ≥ 2 KiB. `StoryOk` stays **false**; that is correct, hero mode has no story |
| `play` | PASS — exit 0, PNG, `STORY_LOAD ok`, **`AssetErr` false**. This is the one that upgrades §1 row 1 from *inferred* to *proven*: it is the first run that opens the packed wavs and `.vox` with no repo on the box |
| `launcher` | PASS with `STORY_LOAD ok` from CWD = `C:\` and no arguments — i.e. `run-voxelforge.cmd` did both jobs. `StoryOk=False` here means the launcher forgot `--play` |
| `nostory` | FAIL with a `StoryDataRes` panic — that is the point; it proves `act1.json` is a hard dependency and must never drop out of the pack |
| `wrongcwd` | boots, but **no** `STORY_LOAD ok` and no `maps/edhari.json` — the §2 fragility, visible |

---

## 7. Open items for the ship lane

1. **Rebuild the release binary** at ≥ `8176d01` and re-run §6. Until then there
   is no shippable artifact, packaging aside, and §1 row 1 stays *inferred*.
2. Make the read-only `std::fs` paths exe-relative and move the writable ones —
   `settings.json`, `editor_config.json`, `maps/quicksave.json` — to `%APPDATA%`
   (§2). Needs a build to verify.
3. Decide whether `play` should be the default mode instead of an argument
   (`read_cfg`, `main.rs:153`). Today every launch path that forgets `--play`
   silently ships the editor; the launcher and the Steam launch option both
   carry the flag, but that is a workaround for a default, not a fix.
4. `trim-paths = "all"` on the release profile (§5).
5. `assets/audio/CREDITS.md` is packed — someone should confirm the licences
   listed there are satisfied by shipping that file alone.
