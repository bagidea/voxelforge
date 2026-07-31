# Voxelforge — Story Bible

> **Canon document.** Narrative spine for the full campaign. Engineer-readable where it touches gameplay. Design decisions marked **LOCKED** or **OPEN** for CEO review.
> Last updated: 2026-07-31 · Owner: Monanisa (Design). Act 1 story-data + Toma canon pass: Rose.

---

## 1. Logline, Theme & Tone

### Logline

> *The last survivor of a shattered village descends into the dungeon beneath it — and discovers that the creature which destroyed everything they knew was once the god who built it.*

### Theme

**Creation and destruction are the same act.** The power to build a world from nothing is identical to the power to unmake one. Every mechanic carries this — placing blocks, burning campfires, dying and returning. The player is not here to save the world. They are here to decide what the world becomes next.

### Tone

**Melancholy wonder** — not despair, not triumph. The emotion of finding a beautiful ruin and needing to understand it. Visually warm (golden hour, honey-amber walls, the *cozy kitchen at 5pm* from the beauty-shot brief). Narratively heavy. The contrast between the warmth you see and the grief underneath is the hook.

Reference register: the silence of Shadow of the Colossus, the weight of Elden Ring's environmental lore, the tactile joy of Minecraft's first morning — all three at once.

### Visual–Emotional Alignment

| Visual element | What it's supposed to make you feel |
|---|---|
| Golden key light (`#F4B860`) washing ruined stone | Something beautiful existed here |
| Falling ash particles (Act 0 spawn) | It's still happening — this is not aftermath, it's ongoing |
| Teal accent (`#4FC9D6`) on the dungeon gate sigil | Alien, ancient, pre-human — the Shapers' signature colour |
| No UI tutorial text | You are trusted. The world will teach you. |
| Campfire warmth at each rest point | You belong here. Come back. |

---

## 2. World & Lore

### Why This World Is Made of Blocks

The world of **Vaelthar** was not born — it was *built*.

In an age before memory, formless elemental chaos filled existence. Then came the **Shapers**: beings of pure creative will who discovered they could crystallise chaos into ordered matter — cubes of stone, wood, ore, and light. Block by block, they raised the land, carved the mountains, and filled the sky with stars. This act of crystallisation is called **the Forge** — not a place, but a process: the conversion of chaos into form.

The Forge is why everything in Vaelthar has hard edges. Organic curves are a luxury of older cosmologies. Here, a tree is a stack of wood-blocks because a Shaper decided where each one goes. The world remembers being made.

### The Shapers' Disappearance

Roughly three centuries before the game begins, the Shapers vanished. No recorded war, no visible cataclysm — simply absence. Villages like **Edhari** (and dozens of others) continued to exist, slowly forgetting the cosmological context of their own construction. Descendants of those who served the Shapers learned to build using inherited technique — placing blocks, shaping shelters — without understanding they were performing a diminished echo of a divine act.

What the villagers of Edhari do not know: the Shapers did not leave. They *unravelled*.

### The Unravelling

When a Shaper stops willing a structure to exist, it begins to dissolve — blocks losing cohesion and returning to raw chaos. The Unravelling is what happens when a Shaper turns their creative force inward, consuming form rather than generating it.

**In gameplay terms:** The Unravelling is visible. Crumbling block edges, ash particles in air, the faint shimmer where solid walls have lost their coherence. Players will learn to read it as environmental danger-signal — a zone where the Unravelling is active is structurally unstable. Walls collapse. Floors give way. The monster is always nearby.

### World Rules (Narrative Constraints)

| Rule | Gameplay expression |
|---|---|
| **Blocks remember their Shaper.** Structure placed with intent is more stable than structure placed carelessly. | Player-placed blocks during combat are permanent; enemy-touched blocks crumble faster |
| **Campfires are Forge-points.** Ancient Shaper anchors where chaos-suppression was strongest. | Respawn point, heal point — narratively: you are re-formed here |
| **The Teal is Shaper-light.** `#4FC9D6` appears only on objects of Shaper origin — gates, seals, artefacts. | Visual shorthand: if it glows teal, it's ancient and important |
| **Death is not permanent.** A partial Shaper can re-crystallise from their last anchor. | The campfire system is cosmologically justified |

---

## 3. Protagonist

### Identity

**Name:** ⟨OPEN — awaiting CEO approval. Working name: **Auren**⟩
**Role:** Traveller who arrived in Edhari on the night of the attack — not a villager, not a stranger. They came looking for something specific.
**Background:** Last of a bloodline that carries dormant Shaper ability. They do not know this yet. Elder Maren does.

### What They Were Looking For

Auren arrived in Edhari following rumours of a sealed dungeon beneath the village — said to contain the last functional fragment of the original Forge. Their goal on the night of the attack: ask for directions. The attack happened before they could.

### Motivation Ladder

| Stage | What they want | Why |
|---|---|---|
| **Act 0–1** (10 min) | Survive. Find survivors. Kill whatever did this. | Immediate, visceral — driven by the scene around them |
| **Act 1–2** (Chapter 1–3) | Get into the dungeon. Find the creature. | Elder Maren's voice gives them a mission; their own stubbornness carries it |
| **Act 2** (Chapter 4–5) | Understand what the creature is. | The ruins inside the dungeon don't look like a lair — they look like a *home* |
| **Act 3** (Chapter 6) | Decide: destroy the last Shaper, or try to restore them | The central moral weight of the game |

### Character Arc

**Survivor → Warrior → Shaper**

Auren begins as someone who doesn't fully trust their own body (the stamina-based combat teaches them their limits). By the midpoint they fight with confidence. By the end they are deliberately shaping the world — placing blocks to solve problems, using Forge-energy as a weapon. The mechanical skill progression mirrors the narrative arc exactly.

**Arc beat in the first 10 minutes:** The moment they approach the dungeon gate and the teal sigil pulses in response to them — and them alone — is the first hint. The game does not comment on it. Players who replay will catch it.

### Supporting Cast — Toma (child of Edhari)

**Status:** Canon as of 2026-07-31. Proposed in `docs/character-design.md` §3.2 (Monanisa) and admitted by the story-bible owner on review. **No LOCKED fact was altered** — verified against every LOCKED item in §7: the carved toy carries *no* teal (respects "The Teal is Shaper-light"); Toma is a villager (Auren is not — §3); Toma is one of the three previously-unnamed Act 2 survivors (§5), so this only names something the bible already implies exists.

**Who:** A child of Edhari — the one who drew the picture on the west-house wall (§6, Act 1 environmental fragment). Small, frightened, still alive where almost everyone else is gone.

**Why Toma exists:** the ruined village is affecting because it is abstract; Toma makes it specific. Finding a living child hiding behind their own frightened drawing reframes the whole act — this is not a place that *was* a home, one is still trying to *stay* a home.

**Narrative function (does triple duty, like Maren's lines):**
- **Stakes** — a named, vulnerable survivor the player has actually met, not just heard about.
- **Lore vehicle** — Toma saw the Hollow come up through the ground but lacks the words for what it was: a child's-eye account, unfiltered, terrified, specific in exactly the wrong details. Adult NPCs (Maren) cannot give this register.
- **The toy** — Toma keeps a **carved wooden toy that does not glow**. It is the one block-crafted object in the cast carrying no Forge-light: a child "performing a diminished echo of a divine act" (§2) with no idea what they are really doing. The absence of teal is the point — innocence under the weight the player already understands. (Visual spec lives in `docs/character-design.md` §3.2, Monanisa's lane; do not redesign here.)

**Through-line to Act 2:** Toma is **one of the three survivors** the player later recovers from stasis in the deep (§5, Act 2). Naming them in Act 1 turns a static environmental prop into a thread the player carries down with them: the child they met upstairs is one of the three they will find below. Maren knows Toma is still hidden up in the village and does not say so until asked — rationing hope the way she rations truth.

**What Toma is NOT (canon guard):** not a Shaper, not Shaper-touched, carries no latent Forge ability. Toma is an ordinary child. That ordinariness is load-bearing — it is what the Hollow's tragedy is measured against, and it is why the toy stays plain wood even if the thread is developed further.

⟨**OPEN:** Toma's ultimate fate across the two endings (does the child survive Ending A and Ending B, and reunited with whom?) — defer with the canonical-ending OPEN item in §7.⟩

---

## 4. Antagonist & Central Conflict

### The Antagonist: The Hollow

**Working name:** The Hollow ⟨OPEN — CEO may rename⟩

**What it appears to be:** A massive creature nesting in the dungeon below Edhari. It took the villagers. It crumbled the buildings. It is slow, enormous, and terrifyingly patient. Guard Husks (Act 3, first-playable-loop.md) are remnants of villagers partially Unravelled — still moving but no longer themselves.

**What it actually is:** The last of the Shapers. One who experienced a grief so absolute that their creative will inverted — the Forge inside them began to consume rather than create. The Hollow does not destroy out of malice. It Unravels the world because it no longer remembers why it built it.

**What it wants:** To find the thing it lost. It does not know what that thing is anymore. It only knows the search.

**Why it "took" the villagers:** Villagers of Edhari carry a trace of Forge-energy inherited from generations of block-crafting. The Hollow is drawn to this. It has been absorbing their energy trying to remember. It cannot.

### Central Conflict

The conflict is not "hero vs monster." It is:

> **Can something that has forgotten how to create be reminded — or must it be destroyed?**

This is the question Auren must answer by Act 3. There is no objectively correct answer. The game supports both. The world changes differently depending on the choice.

**For the first vertical slice (Milestone 1):** The player encounters the first dungeon boss — a partially-Hollow mid-boss (a former village elder, now Unravelled). Defeating it reveals the first fragment of the Hollow's true nature: a carved wall in the boss chamber that shows a Shaper building Edhari. The Shaper's face is familiar.

---

## 5. Three-Act Structure (High Level)

### Act 1 — The Descent (Chapters 1–2)

**World state:** Edhari is crumbling. The Unravelling radiates outward from the dungeon gate.
**Player goal:** Enter the dungeon. Find out what took the people. Find Elder Maren.
**Story beats:**
- Clear the dungeon entrance (first-playable-loop Acts 0–4)
- Maren provides the first direct exposition: *"The Forge beneath this village was never truly dormant. Something woke it — or something never let it sleep."*
- First dungeon region: the **Hollow Reach** — an ancient Shaper worksite, half-built, half-consumed. Evidence everywhere that something once built here with love.
- Chapter boss: **The Warden** — a former village elder, almost fully Unravelled; still wears the apron of a village craftsperson (in Edhari an elder often *was* a craftsperson; wording unified with §4 Milestone-1 and `docs/character-design.md` §2.2). Defeating it drops the first **Shaper Fragment** — a voxel artefact that pulses teal in Auren's hands. A cutscene: Auren's hand involuntarily shapes a small block from nothing. The game immediately moves on. No dialogue about it.

**Tone:** Grief. The dungeon looks like a place people used to love.

### Act 2 — The Truth in the Deep (Chapters 3–5)

**World state:** The deeper the player descends, the more the architecture shifts from village ruin to something older — clearly pre-dating Edhari by centuries. Beautiful. Enormous. The Hollow built this.
**Player goal:** Reach the Hollow's nest. Recover the remaining survivors (three imprisoned in stasis by Unravelling energy — one of them is **Toma**, the child Auren met hiding in Edhari; see §3 Supporting Cast).
**Story beats:**
- The three **Shaper Chambers** — each a historical record in environmental form. Block-art murals showing the Hollow in its original form: a radiant being building the world with joy.
- A sealed room: the Hollow's personal chamber. Inside, a structure that resembles a cradle. Something small was being made here when the grief hit.
- Maren reveals what she knows: the Hollow lost something (or someone) it was building a future for. She does not know what. She says: *"I think it's still trying to finish it. It doesn't know it's been centuries."*
- Chapter boss: **The Architect** — the Hollow's semi-conscious fragment, manifesting as a massive Shaper-form. The fight is beautiful and terrible. The arena is the half-finished structure from the cradle room.
- After the fight: the Hollow's full form is visible for the first time — an enormous presence filling the deepest chamber, semi-dormant, barely aware.

**Tone:** Grief shifting to understanding. The enemy is not monstrous. It is *lost*.

### Act 3 — The Reshaping (Chapter 6)

**World state:** Edhari above is almost gone — the Unravelling has reached the surface. Players can see the sky through collapsing ceilings. The final campfire burns at the edge of the Hollow's chamber.
**Player goal:** Enter the Hollow's chamber and make the choice.
**The Two Endings:**

**Ending A — The Final Unravelling:** Auren uses the accumulated Shaper Fragments as weapons — drives them into the Hollow, forcing it to unmake itself completely. The Hollow dissolves. Edhari stabilises. The Forge goes dark. Maren survives. The world is safe but diminished — no more Forge energy, no more Shapers, just a quiet world of blocks and people. *"We are free of gods. Now the rest is up to us."*

**Ending B — The Reshaping:** Auren uses the Shaper Fragments to reconstruct what the Hollow lost — the small thing it was building. This completes the Hollow's unfinished act. The Hollow remembers. It does not become what it was — too much is gone — but it becomes something new: a passive, dormant Forge-anchor deep beneath a rebuilt Edhari. Maren survives. The world is rebuilt. The sky turns golden. The Unravelling stops. But something immense still breathes below. *"We built it back together. I don't know what that means yet."*

⟨**OPEN:** Canonical ending — CEO decides which is default / whether both are equally canonical.⟩

---

## 6. Story Through Gameplay — First 10 Minutes

*How the lore above lands in docs/first-playable-loop.md without a word of exposition.*

### Act 0 — Cold Open (first-playable-loop §Act 0)

| Gameplay element | Story it's telling |
|---|---|
| Falling ash particles | The Unravelling is active right now, not historical |
| The campfire already lit | A Forge-point is anchored here — something protected this spot |
| Collapsed stone shelter with hard voxel edges | This was built deliberately; someone shaped this carefully |

**Nothing is said.** The player's body reads the story through survival instinct.

### Act 1 — Explore (first-playable-loop §Act 1)

| Environmental fragment | Narrative layer |
|---|---|
| *Child's drawing — figures fleeing something large* | The Hollow came from below, moved upward. The artist is **Toma** (§3) — a specific child hiding in the west house, not an abstraction |
| *Well inscription: "He came from below. We fed him everything. He left anyway."* | The villagers knew. They tried to appease it. They offered their forge-energy (building, shaping) and it consumed that too — but the grief driving it is not satisfied by energy |
| *Dungeon gate sigil pulses when approached* | **The teal responds to Auren specifically** — other players reaching the gate in a future chapter log will note NPCs cannot trigger it. Auren is a Shaper. The gate recognises them. |

### Act 2 — Story Beat (first-playable-loop §Act 2)

Elder Maren's three lines do triple duty:

| Line | Immediate meaning | Bible layer |
|---|---|---|
| *"The seal will hold another hour — maybe less."* | Urgency | The seal is Shaper-made. It's degrading because the Hollow's Unravelling erodes all Forge-anchors |
| *"The creature below — it's nesting."* | Threat | It's not moving. It's *building something*. The Hollow is still trying to finish what it started. |
| *"There is a way in. But something is already patrolling outside it."* | Obstacle | Guard Husks are former villagers. Maren knows this. She cannot say it yet. |

**What Maren knows and withholds:** Maren is old enough to know the old stories. She recognises the Hollow from oral tradition. She is choosing what to tell Auren and in what order — not out of deception, but out of care. She will tell the full truth in Chapter 2.

### Acts 3–4 — Combat & Resolution (first-playable-loop §§Act 3–4)

| Mechanic | Thematic resonance |
|---|---|
| Guard Husk as the first enemy | The player is fighting someone's neighbour. The weight is there even without exposition. |
| Death → campfire → *"Rest. Try again."* | The Forge re-forms you. Death is not loss — it's a return to origin. |
| Door opening as the reward (no loot) | Progress is architectural. You changed the world by being here. |
| Second campfire activating inside the guard post | You are extending the Forge's reach. Every campfire you activate is you, rebuilding. |

---

## 7. Status — What's Locked vs Open for CEO

### LOCKED

- World name: **Vaelthar**
- Village name: **Edhari** (from first-playable-loop.md, already in engine)
- NPC: **Elder Maren** — role, voice register, position behind gate (from first-playable-loop.md)
- First enemy type: **Guard Husk** — visual (armoured voxel figure), patrol behaviour
- **The Hollow's true nature:** a Shaper, not a monster — this is the irreducible core of the story. Changing this changes everything.
- Teal (`#4FC9D6`) as Shaper-light — already locked in GAME-VISION.md palette
- **Two-ending structure** — both endings exist; canonical default is OPEN
- Campfire = Forge-point cosmology (already implied in GAME-VISION.md)
- Theme: Creation and destruction as the same act
- First dungeon boss: **The Warden** — a former village elder, almost fully Unravelled, in a craftsperson's clothes; drops the first Shaper Fragment (wording unified across §4/§5; visual spec in `docs/character-design.md` §2.2)
- Supporting NPC: **Toma** — a child of Edhari; artist of the west-house drawing; one of the three Act 2 survivors. Ordinary, *not* Shaper-touched (the carved toy deliberately carries no teal). Role locked; visual spec in `docs/character-design.md` §3.2

### OPEN — Awaiting CEO Decision

| Item | Options / Considerations |
|---|---|
| **Protagonist name** | Working name *Auren* — CEO approves or renames |
| **Protagonist gender/presentation** | Intentionally unspecified; third-person camera reduces attachment pressure. Recommend: customisable silhouette at campfire, no voiced protagonist lines |
| **Antagonist name** | Working name *The Hollow* — evocative but could be stronger |
| **What the Hollow lost** | The cradle suggests a child/creation that was destroyed. Deliberately ambiguous to allow CEO to weigh in on emotional register — parental grief vs creative grief vs romantic grief |
| **Canonical ending** | Ending A (extinction/hope) vs Ending B (reconciliation/unease) — or both equal. This affects Chapter 6 design significantly |
| **Maren's fate** | Does she survive both endings? Does she enter the dungeon herself? |
| **Toma's fate** | Survives both endings? Reunited with whom? (Defer with the canonical-ending decision.) |
| **Chapter count** | 6 chapters assumed for full campaign (~15–20 hr). CEO to confirm scope before Chapters 3–6 are designed in detail |
| **Multiplayer implication** | GAME-VISION.md marks co-op as non-goal for v1. Story Bible does not contradict this. Flagged in case v2 scope changes. |

---

*Design: Monanisa. Combat narrative integration: Sahara (research pending). Engine implementation: Poppy. Story sign-off: CEO.*
