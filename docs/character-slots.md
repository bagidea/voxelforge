# Character equipment slots

How Auren (and every character built from the same tables) wears things, and how a
piece of gear is swapped at runtime without rebuilding the character.

Source of truth: `client/src/equipment.rs` (slots, parts, presets, swap) and
`client/src/characters.rs` (bodies, stages, shot timeline). Both are linked by the
`voxelforge_charshot` bin (`client/src/char_shot_main.rs`) and by `voxelforge_shot`.

---

## 1. The shape of it

A character is **one root entity**. Under it hangs one container entity per slot:

```
  root  (Character, Wardrobe)
   ├── SlotNode(Head)    ── boxes…
   ├── SlotNode(Torso)   ── boxes…
   ├── SlotNode(Legs)    ── boxes…
   ├── SlotNode(Hands)   ── boxes…
   ├── SlotNode(Weapon)  ── boxes…
   └── SlotNode(Back)    ── boxes…
```

The body itself (skin + hair + head) is **not** in a slot — it is spawned once on
the root and never touched by an equip. That is the whole reason a costume change
does not have to respawn the character.

`Wardrobe` (a component on the root) stores two parallel arrays indexed by
`Slot::idx()`:

| field | what it holds |
|---|---|
| `loadout.parts[i]` | `Option<&'static Part>` — *what* is worn in slot `i` |
| `nodes[i]`         | `Option<Entity>` — the container entity currently rendering it |

Keeping the node entity in the component is deliberate: it makes `equip` a
despawn + spawn + two array writes, with **no** `Children` walk and no hierarchy
query. That is why the swap is cheap enough to do mid-frame.

`SLOT_COUNT = 6`. Adding a slot means adding a variant, an `idx()` arm, an `id()`
arm, and an entry in `Slot::ALL` — the arrays size themselves off `SLOT_COUNT`.

## 2. Swapping

Two entry points, both in `equipment.rs`:

```rust
// one slot — the weapon change
equip(&mut commands, &mut meshes, &pal, root, &mut wardrobe, Slot::Weapon, Some(&HAMMER_WAR));

// every slot — the outfit change (a loop over equip, still one root)
equip_loadout(&mut commands, &mut meshes, &pal, root, &mut wardrobe, warplate());
```

`equip` does exactly four things:

1. despawn `wardrobe.nodes[slot]` (recursive since Bevy 0.16 — the boxes go with it)
2. `spawn_slot(...)` a fresh container for the new part
3. write the new entity into `nodes[slot]`
4. write the new part into `loadout.parts[slot]`

Passing `part: None` un-equips the slot — the node is despawned and nothing takes
its place, which is how `Loadout::without` and the `bare` preset render.

Every call prints an audit line, so a swap is checkable from the log alone rather
than from the prose above:

```
EQUIP root=12v1 slot=weapon sword_short -> hammer_war boxes=6
```

**The root id in that line never changes across a swap.** That is the evidence
the character was re-dressed and not re-spawned; see §5.

## 3. Slots and the parts registry

20 parts, all `&'static Part` — geometry is a `&'static [Bx]` box list, so a part
costs nothing at rest and is shared by every character wearing it.

| Slot | id | Parts (`id` — name) |
|---|---|---|
| Head | `head` | `kerchief` — Field Kerchief · `hood_travel` — Traveller's Hood · `helm_great` — Greathelm |
| Torso | `torso` | `tunic_linen` — Linen Work Tunic · `jerkin_leather` — Traveller's Jerkin · `cuirass_steel` — Warplate Cuirass |
| Legs | `legs` | `trousers_work` — Work Trousers · `breeches_travel` — Travel Breeches & Boots · `greaves_plate` — Plate Greaves |
| Hands | `hands` | `wraps_cloth` — Cloth Hand Wraps · `gloves_leather` — Buckled Bracers · `gauntlets_steel` — Plate Gauntlets |
| Weapon | `weapon` | `sickle_field` — Field Sickle · `sword_short` — Traveller's Shortsword · `hammer_war` — Warhammer · `stave_shaper` — Shaper's Stave · `spear_guard` — Guard-Issue Spear |
| Back | `back` | `pack_harvest` — Harvest Basket · `cloak_half` — Half-Cloak & Travel Roll · `cape_battle` — Battle Cape |

Each slot's three (or five) parts are a deliberate **material ladder** — cloth →
leather → steel — so a swap reads at a glance even in silhouette, and so the
material lane has a character-sized test of every `Surf` class.

Lookup by string is `equipment::part("hammer_war")` / `Slot::from_id("weapon")`.

## 4. Presets

| preset | aliases | Head / Torso / Legs / Hands / Back / Weapon |
|---|---|---|
| `bare` | `naked`, `body` | *(nothing — the body alone)* |
| `villager` | `village` | kerchief · tunic_linen · trousers_work · wraps_cloth · pack_harvest · sickle_field |
| `adventurer` | `traveller`, `hero` | hood_travel · jerkin_leather · breeches_travel · gloves_leather · cloak_half · sword_short |
| `warplate` | `war`, `plate`, `armour` | helm_great · cuirass_steel · greaves_plate · gauntlets_steel · cape_battle · hammer_war |

`adventurer` is the default Auren — the hero as Act I opens.

The two ladders the shot stages walk:

- `outfit_ladder()` → villager → adventurer → warplate
- `weapon_ladder()` → sword → hammer → stave *(weapon slot only; everything else held)*

## 5. Proving a swap is real

The obvious failure mode of a "costume change" screenshot is that it is three
separate spawns photographed once each. `charshot_timeline` in `characters.rs` is
built so that cannot be what happened:

- the root is spawned **once**, in `Startup`
- the root is never despawned and `spawn_character` is never called twice
- all three captures come out of **one process**, at one camera, one shader cache
- every capture prints `SWAP_CAPTURE n/3 root=… wearing[…]`, and every equip
  prints `SWAP_APPLY … (root NOT respawned)` plus the per-slot `EQUIP root=…`

So the three frames are one body, and the log carries the root entity id on each
of them. Matching root ids across the three lines is the proof; the picture on
its own is only the illustration.

Capture timing is **frame-counted, not wall-clock** (`WARM_FRAMES = 96`,
`STEP_FRAMES = 54`, `TAIL_FRAMES = 60`), and the equip lands two frames *after*
each capture — so a grab can never race the change it is supposed to precede, and
a busy machine cannot catch two plates at different points of the settle.

## 6. Rendering the stages

```
VOXELFORGE_CHARSHOT=<stage>   what to render
VOXELFORGE_SHOT=<path.png>    where to save
VOXELFORGE_CAM / _SUN / _AMBIENT / _EXPOSURE    the usual shot-lane overrides
```

| stage | aliases | renders | captures |
|---|---|---|---|
| `auren` | | the current Auren body, default loadout | 1 |
| `auren-v1` | | the 2026-08-13 Auren body, for the before/after plate | 1 |
| `auren-v2` | `v2`, `presculpt` | the 2026-08-14 proportion pass, pre-sculpt | 1 |
| `quad` | `ladder`, `gear`, `sets` | **the four gear sets on one entity** | 4 |
| `line` | `cast` | the cast line-up | 1 |
| `silhouette` | `sil` | the line-up as flat silhouettes | 1 |
| `swap` | `outfits`, `loadouts` | the outfit ladder on one body | 3 |
| `weapons` | `weapon`, `arms` | the weapon ladder on one body | 3 |
| `catalog` | `catalogue`, `dump` | the registry as JSON — no render | 0 |

Two extra levers, both read by the bin rather than baked:

| env | default | what it does |
|---|---|---|
| `VOXELFORGE_RES` | `1280,720` | frame size. The camera is untouched — Bevy's `fov` is *vertical*, so aspect changes what is visible left/right, never how tall the subject is in frame. |
| `VOXELFORGE_SCULPT` | on | `0` renders the **pre-sculpt** tables (every part truncated to its `presculpt_len`, Auren's body swapped for `AUREN_V2`) so one binary shoots both halves of a before/after. See [`character-sculpt-pass.md`](character-sculpt-pass.md). |

The multi-capture stages own their own screenshots and exit
(`CharShot::owns_capture()`); the single-still stages go through the bin's
frame-counted one-grab path. Multi-capture output is named
`<stem>-<n>-<label>.png`, e.g. `swap.png` → `swap-1-villager.png`,
`swap-2-adventurer.png`, `swap-3-warplate.png`.

`auren-v1` exists **only** so the before/after pair is two stages of one binary
at one camera: the plates cannot differ by build, shader cache, driver or
framing — only by the geometry under test.

`catalog` dumps the whole registry (every part, its slot, box count, material
classes, and the three presets) to JSON. That is the hook a future
item/inventory system reads instead of re-deriving this table.

## 7. Why a separate bin

`voxelforge_shot` `#[path]`-includes `hero.rs` **and** `vfx.rs`. A teammate
mid-edit in either file blocks the character contact sheet from rendering at all,
even though the character lane touches neither. `voxelforge_charshot` links
`characters.rs` + `equipment.rs` only, so it cannot be stopped by a lane it does
not use. The stage wiring in both bins is the same block driving the same tables,
so they render the same frames — this one is just the copy that always builds.
