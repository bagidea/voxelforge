# Dialogue UI — before/after proof (Sun, dialogue lane)

The Act-I dialogue box was redesigned from an egui overlay into a native Bevy
Node UI that wears the same walnut/amber HUD language as `hud.rs`. This proof
ships three captures under `docs/assets/story/` so the change is visible
without a playthrough:

| File | Proof mode | What it shows |
|---|---|---|
| `dialogue-a-before.png` | `before` | The **pre-redesign egui box** (pulled verbatim from `git show HEAD:client/src/dialogue_ui.rs`) |
| `dialogue-b-typing.png` | `typing` | The **new native box mid-typewriter reveal** (t=2.9 s seed, shot t=3.2 s) |
| `dialogue-c-choices.png` | `choices` | The **new native box in choosing mode** (5 real choices laid out) |

All three seed the real `dlg_maren_gate` node from `assets/story/act1.json`
(Elder Maren, 4 lines + 5 choices), so what's on screen is shipped story
content rendered by the real UI — not placeholder text. Seeding lives in
`dialogue_ui.rs::dialogue_debug_proof_seed`; rendering in `before` mode lives
in `dialogue_ui.rs::dialogue_box_egui`.

## before → after

| Area | Before (egui) | After (native Node) |
|---|---|---|
| Render path | `egui::Area` overlay in `EguiPrimaryContextPass` | Bevy `Node` tree rebuilt per frame in `Update` |
| Palette | hard-coded near-black fill + gold `Stroke(1.5)` | `PANEL_BG` / `ACCENT_AMBER` / `TEXT_CREAM` / `PANEL_BORDER` tokens, same as `hud.rs` |
| Speaker header | gold `colored_label` + `✕` close | portrait swatch (canon colours, `character-design.md` §3) + nameplate + `X` close |
| Line reveal | instant full text | per-character typewriter (30 chars/s), `E` skips to full |
| Continue cue | static dim hint text | blinking amber indicator + dim hint |
| Choices | bare `egui::Button` rows | bordered nodes, `1..N` numbered labels, click or digit key |
| Advance key | `Space`/`Enter` | `E`/`Space` (advance-only) |

## What broke → what was fixed

1. **Enter/continue overloading auto-picked a choice.** The old `Space` handler
   wrote `Choose(0)` the moment `choosing` was true, so holding the advance key
   across a line-ending advance dropped the player into choice 0 on the same
   frame (the exact bug that sent the quest demo down the wrong dialogue branch
   — the old file's own comment calls it out). The new input system is
   **advance-only on E/Space**; a choice is never auto-picked on a continue
   press.
2. **No reveal pacing.** Before, a line appeared whole; there was no way to feel
   Maren speaking. After, the typewriter reveals at 30 chars/s and `E` skips.
3. **Visual drift from the HUD.** The egui box was generic — a near-black
   egui window that ignored the walnut/amber HUD tokens. The native box reuses
   `PANEL_BG`/`ACCENT_AMBER`/`TEXT_CREAM`/`TEXT_DIM` verbatim and adds the gold
   focus-frame + portrait swatch so the dialogue reads as part of the HUD, not a
   floating debug panel.

## Reproduce

```cmd
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
set VOXELFORGE_DIALOGUE_PROOF=before
set VOXELFORGE_SHOT=docs\assets\story\dialogue-a-before.png
target-sun\debug\voxelforge.exe --play > docs\assets\story\dialogue-a-before.runlog 2>&1
```

(`typing` / `choices` swap the two env vars; each `.runlog` carries the real
`DIALOGUE_PROOF_SEED mode=… t=… lines=4 choices=5` line from the seeder.)

## Build

`cargo build --bin voxelforge` with `CARGO_TARGET_DIR=target-sun` (isolated
target dir; the working tree is shared across lanes, so no git stash/checkout).
