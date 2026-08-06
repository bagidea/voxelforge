# Voxelforge — Title / Main Menu Design

> **Design spec + real mockup** for the title screen. Direction: read as one system with the
> combat HUD (`docs/hud-design.md`) and the settings panel (`client/src/settings_menu.rs`) —
> same warm walnut→amber tokens, same accent-tick language, same "fade/slide in, don't snap"
> motion rule — so a player sees three different UI surfaces and reads them as *one game's* UI,
> not three unrelated kits bolted on.
>
> Owner: Monanisa (Design) · Scope: **docs + mockup only**. There is currently no title-screen
> state in the client at all — `client/src/editor.rs`'s `AppState` only has `Editor`/`Play`, the
> game boots straight into one of those (`main.rs`, Kevin's lane per `docs/LANES.md`). Shipping
> this needs a new `AppState::MainMenu` variant plus a new `client/src/main_menu.rs`, both outside
> my lane and outside this task's scope (`ห้ามรัน cargo`, code scope = `settings_menu.rs` only).
> This doc is the handoff spec — same pattern as `hud-design.md`'s combat-HUD handoff to Kevin.
> Last updated: 2026-08-06.

---

## 0. The one rule this whole doc follows

Same rule as `hud-design.md` §0 and `look-bible.md` §4: **~85% warm walnut→amber, teal capped at
~10–15% and spent only where it means "Shaper."** The title screen is the very first frame a
player (or the CEO) sees, so it's the highest-stakes place to prove the golden-hour identity reads
immediately — no grey egui chrome, no cool-toned UI kit fighting the world behind it.

Second rule, carried from the HUD doc: **the world is the background, not a backdrop.** The menu
composites over the actual golden-hour render (`docs/assets/wide-hero-final-nohud2.png`, real
in-engine, no HUD) with a soft espresso vignette for legibility — never a flat opaque panel — so
the first thing the player sees is the game's own look, not a loading-screen-style overlay.

---

## 1. Layout — rule of thirds, not centered

Centering a title + vertical menu stack (the laziest default) would sit a solid block of UI right
on top of the frame's visual center — exactly where `hero.rs`/`look.rs`'s golden-hour framing
wants the eye to land. Instead:

- **Title wordmark**, top-left third: `VOXELFORGE`, Georgia Bold 62px, `text-cream`
  (`#E8D8B8`), with a 2px espresso drop-shadow for legibility against bright sky, and a short
  74px `accent-amber` (`#F4B860`) tick underneath — the same "amber tick = this is the brand's
  accent, spent deliberately" language as the HUD's quest banner and lock-on reticle.
- **Tagline**, directly under the tick: *"a golden-hour world"*, Georgia Italic 17px, muted
  cream, low-key — a mood line, not a subtitle competing with the wordmark.
- **Menu column**, lower-left third: `PLAY` / `SETTINGS` / `QUIT`, Segoe UI Light 26px,
  uppercase, hand-tracked letter-spacing (+4px per glyph — egui/PIL don't do CSS
  `letter-spacing` for free; the real implementation needs the same char-by-char advance loop
  the mockup script uses, not the font's default tight tracking). 56px vertical rhythm between
  items — enough to read as deliberate spacing, not a cramped list.
- **Version/build tag**, bottom-right corner, 12px muted text — present for QA/support, never
  competing for attention.

Nothing sits in the frame's center or right two-thirds — the vista stays the hero.

---

## 2. Hierarchy & motion — idle vs. hover

| State | Weight | Colour | Position | Tick |
|---|---|---|---|---|
| Unfocused item | Segoe UI **Light** | `text-cream` @ ~65% alpha | baseline `x` | none |
| Focused/hovered item | Segoe UI **Bold** | `accent-amber` full | `x + 10px` (slides right) | 4×28px amber bar, left of label |

The motion is three things changing together, not one: **weight** (Light→Bold), **color**
(dimmed cream→amber), and **position** (a 10px slide-right) — plus the tick bar appearing. Ease
over ~150ms. This mirrors the settings-menu tab underline (§ below) and the HUD's lock-on
brackets: the whole game uses "an amber tick + a small deliberate motion" as its one focus
language, rather than inventing a different hover idiom per screen.

Default focus on boot is `PLAY` (first item, no input yet) — a controller/keyboard-only player
should never open on a screen where nothing is focused.

---

## 3. Design tokens

All 4 pulled verbatim from the tokens already shipped in `client/src/settings_menu.rs::theme`
and `docs/hud-design.md` §2 — the title screen introduces **zero new colours**, which is the
point (one palette, three UI surfaces):

| Token | Hex | Used for |
|---|---|---|
| `panel-espresso` | `#1A120D` (vignette base, alpha-ramped) | Legibility gradient behind title + menu |
| `text-cream` | `#E8D8B8` | Title wordmark, unfocused menu items (dimmed) |
| `accent-amber` | `#F4B860` | Focus tick, focused item text, title underline tick |
| `text-muted` | `#B2A08A` | Tagline, version tag |

---

## 4. Mockup (real render, not concept art)

`docs/assets/main-menu/main-menu-v1-golden-hour-mockup.png` — idle (default focus: Play) and
hover (pointer over Settings) composited side by side on the real `wide-hero-final-nohud2.png`
background, so the hover delta (weight + colour + slide + tick) is visible directly, not just
described in the table above.

Rendered by `_main_menu_mockup.py` (repo root, scratch script — same convention as
`_hud_mockup.py`): PIL draws the vignette + wordmark + tracked-letter-spacing menu text on top of
the background PNG. This is a design mockup, not a live engine capture (cargo is off-limits this
pass) — but every hex value and every layout number in it matches this doc exactly, so it is what
the implementation should look like once built, not a guess at it.

---

## 5. Implementation handoff

Needs, in order:

1. **`AppState::MainMenu`** added to the enum in `client/src/editor.rs` (Kevin's lane) — boot
   flow becomes `MainMenu → (Play | Editor)` instead of booting straight into one.
2. **`client/src/main_menu.rs`** (new file, needs a lane assignment — nobody owns it yet per
   `docs/LANES.md`; ask the Director before the first edit, same as any unlisted file) — an egui
   pass drawing the layout in §1 using `client/src/settings_menu.rs::theme`'s existing tokens
   (import, don't refork the palette), wired so `PLAY` transitions to `AppState::Play`,
   `SETTINGS` opens the existing `SettingsMenuState` (already built, already themed — no new
   settings UI needed, just make it reachable from here too), `QUIT` calls
   `AppExit`/`EventWriter<AppExit>`.
3. The background is the **live 3D scene already rendering**, not a baked image — same camera/sun
   the golden-hour grade already applies to. The mockup PNG is a stand-in for what that live frame
   looks like; it is not itself an asset to ship.

I have not touched `main.rs`, `editor.rs`, or created `main_menu.rs` — those are Kevin's/an
unassigned lane, and this task's scope was docs + mockup for the title screen, settings-menu code
only. This spec plus the settings-menu upgrade (§6 of the handoff, see
`docs/hud-design.md`-style patch precedent) is what's ready for whoever picks up the lane.

---

## 6. Settings panel v2 — same system, now production-grade (code landed)

Unlike the title screen, the settings panel already has an owned lane and existing code
(`client/src/settings_menu.rs`, mine) — so this part shipped as real Rust this pass, not a
handoff spec. Changes:

- **Graphics tab rebuilt around 4 quality-preset cards** (Low/Medium/High/Ultra) instead of a
  bare `ComboBox` — each card shows the tier name and a one-line summary of what it actually
  turns on, sourced from `look.rs::insert_stack`'s tier `match` (read-only — I did not touch
  `look.rs`; the exact wording lives in `QUALITY_PRESETS` in `settings_menu.rs` and needs manual
  updating if rose retunes the tier map). Selected tier gets a solid `accent-amber` border +
  `widget-active` fill; hovering an unselected card eases the same border/fill in over ~150ms
  instead of snapping. An "Advanced" disclosure lists the same per-tier text for players who want
  it spelled out without cluttering the default view.
- **Animated tab underline** — the Graphics/Display/Audio/Keybinds tab row now grows an
  `accent-amber` underline in on hover and snaps to full width when selected (same accent-tick
  language as the title-menu focus indicator and the HUD's lock-on brackets/quest banner tick —
  one motion vocabulary across all three UI surfaces).
- **Fade + settle on open** — the panel eases in (alpha 0→248, plus a 10px settle-down) over
  ~180ms instead of appearing instantly. Steady-state alpha is a *near*-opaque 248/255, not the
  HUD's translucent-plaque look: this is a paused, text/slider-heavy modal the player stopped to
  read, so legibility wins over the "let the world bleed through" rule that's correct for
  in-combat HUD elements (`hud-design.md` §0) but wrong for a settings dialog.
- **Section subtext** added under every tab heading (Display/Audio/Keybinds too, for parity) so
  each tab reads as a labelled group, not just a heading + a pile of controls.
- Panel grew from 540×420 to 580×460 to fit the card row without cramming.

Mockup: `docs/assets/main-menu/settings-panel-v2-production-mockup.png` — same hex tokens and
layout maths as the real `graphics_tab`/`settings_ui` code, composited over the live-background
context (world dimmed by a scrim, no blur — nothing in the current stack blurs the background, so
the mockup doesn't invent one). This is a design mockup, not an engine screenshot — cargo is off
the table this pass — but every number in it is the number in the code, not a guess.

**Known-unverified new egui/API surface** (cargo forbidden this pass, so none of this has
compiled): `egui::CollapsingHeader`, `egui::Frame::group(&Style)`, `Response::interact(Sense)`,
`Context::data`/`data_mut` (temp-memory hover stash), `Context::animate_bool_with_time`,
`Painter::line_segment`. Everything else reuses patterns already proven compiling elsewhere in
this exact file/crate (`RichText::new(..).size(..).color(..)` is already load-bearing in
`dialogue_ui.rs`). Flagging these six specifically so whoever runs the next `cargo check
--target-dir target-<lane>` knows exactly where to look first if anything's off.

*Design: Monanisa. Palette source of truth: `docs/look-bible.md` §4, already-shipped
`settings_menu.rs::theme` tokens, `docs/hud-design.md` §2.*
