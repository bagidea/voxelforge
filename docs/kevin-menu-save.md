# Kevin — Main menu + save/load full-circle proof

Owner: Kevin (menu + save/load seam). Status: evidence captured from one binary
(`target-kevin/perf/voxelforge.exe`), no mockups — every frame below is an
in-engine `Screenshot::primary_window()` write.

## What this lane adds

- **`client/src/main_menu.rs`** — the game entrance. A plain launch boots to
  `AppState::MainMenu` with the play scene pre-loaded behind it. Four rows:
  **New Game / Continue / Settings / Quit**, keyboard nav (↑/↓ + Enter) and
  mouse both route through one `MenuAction` message.
- **`client/src/save_game.rs`** — the real save/load seam. `savegame.json`
  (position / yaw / HP / stamina) + `quest_save.json` (Sun's journal) are written
  together on F6 / New-Game-then-walk, and Continue restores both.
- **`VOXELFORGE_MENUSHOT=before|after`** — headless menu capture (this lane's own
  flag, added to `main_menu.rs`, no cross-lane touch).

## Headless evidence recipe (reproducible)

`_kevin_run_demo.cmd` wraps the sequence; by hand it is:

1. `del savegame.json quest_save.json` — clean slate (Continue dimmed in the before shot)
2. `VOXELFORGE_MENUSHOT=before` → `menu-a-before.png` + `.runlog`
3. `VOXELFORGE_SAVE_DEMO=save` → New Game → walk → save → `save-before-load.png` + `.runlog`
4. `VOXELFORGE_MENUSHOT=after` → `menu-b-after.png` + `.runlog`
5. `VOXELFORGE_SAVE_DEMO=continue` → Continue → `save-after-load.png` + `.runlog`

Every step runs `target-kevin\perf\voxelforge.exe` from the repo root (so
`savegame.json` / `quest_save.json` / `docs/assets/menu/` resolve to one place)
and pipes stdout+stderr to the sibling `.runlog`.

## The four frames

| file | what it proves |
| --- | --- |
| `docs/assets/menu/menu-a-before.png` | real menu, **no save on disk → Continue dimmed** |
| `docs/assets/menu/menu-b-after.png` | real menu, **save on disk → Continue lit + slot line** (`Saved pos (…,…,…) hp …`) |
| `docs/assets/menu/save-before-load.png` | in Play, right after save — overlay prints pos/hp/quest flags |
| `docs/assets/menu/save-after-load.png` | in Play, right after Continue — overlay prints the **same** pos/hp/quest flags |

`save-before-load` and `save-after-load` carry an on-screen `SAVE / LOAD PROOF`
readout (pos, yaw, hp, stamina, quest flags) so the match is readable from the
pixels, not just from the log. The "before" frame prints the exact snapshot
`write_save` flushed (`LastSaveSnapshot`), so it can't drift from what Continue
restores.

## Game-printed proof lines (secondary, from the same runs)

- `GAME_SAVE ok path=savegame.json pos=(…) yaw=… hp=… stamina=…`
- `GAME_SAVE flags={…}` / `GAME_LOAD flags={…}` — quest state round-trips
- `GAME_LOAD ok pos=(…) yaw=… hp=… stamina=…` — must equal the `GAME_SAVE ok` line

## Keybind

- **F6** quick-save in Play (F5/F9 are the editor's map save/load).
