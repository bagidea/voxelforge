# Combat & World-Feel Research Brief

> **For:** Voxelforge — AAA voxel action-adventure  
> **Vibe:** Minecraft-style voxel world + third-person camera + soulslike combat + story-driven campaign first, co-op later.  
> **Date:** 2026-07-26  
> **Researched by:** Sahara

---

## 1. Soulslike Combat Loop — What the Prototype Must Have

| System | Must-have behavior | Why it matters |
|--------|-------------------|----------------|
| **Stamina** | Attacks, dodges, blocks, parries, and sprint all drain a shared stamina bar. Depletion leaves the player unable to act and vulnerable. | Creates the commitment/recovery rhythm that defines methodical combat. [[1]](https://eldenring.wiki.fextralife.com/Stamina) [[2]](https://www.theseus.fi/bitstream/handle/10024/877212/Nguyen_Thuan.pdf) |
| **Dodge / Roll + i-frames** | Rolling grants brief invincibility. Equip load changes roll speed and i-frames: light = longer i-frames and fast recovery; heavy = better defense but sluggish roll. | Rewards timing and makes build choice tactile. [[3]](https://eldenring.wiki.fextralife.com/Dodging) |
| **Lock-on / Soft-lock** | Camera and movement orbit the current target; release for ranged/AOE or environment awareness. | Essential for melee readability in third-person voxel spaces. |
| **Light / Heavy / Charged Attacks** | Light = fast, low commitment, interrupts light foes. Heavy = slow, high damage, hyper armor. Charged = risk/reward burst windows. | Gives every weapon a simple but expressive vocabulary. |
| **Poise / Stagger / Hit-reaction** | Fast weapons interrupt easily; heavy swings grant hyper armor. Enemies and player both have poise thresholds that trigger stagger. | Adds matchup depth beyond raw DPS. [[4]](https://darksouls.wiki/books/mechanics/page/poise) [[5]](https://darksouls3.wiki.fextralife.com/Poise) |
| **Parry / Guard Counter** | Timed defense deflects an attack and opens a punish window. | High-risk high-reward option for aggressive defense. [[6]](https://www.thedrastikmeasure.com/2026/05/11/windrose-pc-review/) |
| **Boss Telegraphs** | Every attack broadcasts intent through animation, sound, posture, glows, or movement. Fast jabs = short tells; heavy/special strikes = layered, long tells. | Fair difficulty comes from practice, not guessing. [[7]](https://brokenbuildstudios.com/why-enemy-telegraphs-matter-in-game-design/) |
| **Pattern + Phase Design** | Bosses use consistent movesets that evolve across phases—same moves, new timing, range, or follow-ups. | Mastery stays meaningful while tension escalates. [[8]](https://punishedbacklog.com/the-first-berserker-khazan-review/) |

### Modern twists worth noting
- **Black Myth: Wukong** emphasizes speed, movement, and last-millisecond timing shifts rather than stamina starvation. [[9]](https://gamerant.com/black-myth-wukong-boss-fights-how-work-transformations/)
- **Flintlock** removes stamina entirely in favor of movement + firearm interrupts.
- **No Rest for the Wicked** ties dodge type to weight class and uses consumables for stamina/poise recovery. [[10]](https://game8.co/articles/reviews/no-rest-for-the-wicked-review-early-access)

**Prototype priority:** stamina, dodge i-frames, lock-on, light/heavy/charged, poise, and honest boss telegraphs come first. Everything else is polish.

---

## 2. How Voxel / Blocky 3rd-Person Games Handle Story Campaign + Combat

### Hytale — the closest reference
- **Combat:** skill-based melee/ranged/magic with light/heavy combos, stamina, blocking, parrying, directional dodging, weapon-specific signature moves, and multi-phase bosses. [[11]](https://hytale.game/en/combat-system/)
- **World/feel:** block-based Orbis with curated procedural worlds, distinct zones (Forest, Desert, Arctic, Devastation), factions, dungeons, and a planned Adventure Mode story campaign. [[12]](https://hytale.game/en/hytale-the-game/)
- **Takeaway:** voxel + soulsy action + narrative campaign is a proven combination if the world is curated, not purely random.

### Trove — voxel MMO with class variety
- **Combat:** 17 classes, action combat with dodges, charged shots, summons, stealth, and ultimates. [[13]](https://trove.fandom.com/wiki/Class)
- **World/feel:** colorful destructible voxel biomes, personal Cornerstones, Club Worlds, dungeons, repeatable boss events, seasonal campaign events. [[14]](https://gamersnexus.net/news/1609-pax-trove-voxel-rpg-preview)
- **Takeaway:** loot/build variety and event-style campaign content work well in a voxel wrapper.

### Deep Rock Galactic — best-in-class voxel destruction feel
- **Feel:** "Minecraft + Left 4 Dead" class-based co-op with 100% destructible smoothed-voxel caves. [[15]](https://www.gamingonlinux.com/2019/03/deep-rock-galactic-a-1-4-player-co-op-fps-with-a-destructible-environment-works-great-in-steam-play/)
- **Terrain design:** marching-cubes/smoothed voxels hide cube edges while preserving full destruction; different rock hardness changes mining speed and tactics. [[16]](https://deeprockgalactic.fandom.com/wiki/Terrain)
- **Takeaway:** make voxel terrain a combat participant—cover, chokepoints, escape routes, and arena reshaping—not just scenery.

### Roblox action/story-campaign hits
- **Jujutsu Infinite / Kaizen:** story-mode action RPGs with curse techniques, boss fights, PvE missions, PvP. [[17]](https://www.escapistmagazine.com/best-jujutsu-kaisen-games-on-roblox/)
- **Blox Fruits / Grand Piece Online:** open-world progression across islands, ability builds, boss fights, regular updates. [[18]](https://bloxmake.com/blog/top-10-roblox-rpg-games-you-must-play-in-2025-)
- **gamespace 2025 ranking** lists Blox Fruits, Kingdom Life II, Fruit Battlegrounds, The Wild West, and Dress to Impress as top Roblox fighting/adventure titles; these emphasize quests, varied worlds, and PvE/PvP opponents rather than structured dungeons or team PvE. [[19]](https://gamespace.com/all-articles/news/2025s-best-roblox-fighting-and-adventure-games-ranked/)
- **World // ZERO** is a class-based dungeon RPG with multiple classes, Normal/Nightmare dungeons, Infinite Tower, and gear/perk progression tied to dungeon clears. [[20]](https://www.destructoid.com/world-zero-classes-guide-tier-list/)
- **Dungeon Quest** is a co-op dungeon crawler with party scaling, class/role expectations, dungeon-themed gear sets, and upgrade progression through sequential dungeons. [[21]](https://dungeonquestroblox.fandom.com/wiki/Dungeon)
- **Heroes’ Legacy** is a class-based fantasy RPG with Warrior, Mage, Cleric, Rogue, and Ranger classes and multiple dungeon/area locations. [[22]](https://heroes-legacy.fandom.com/wiki/Classes)
- **Takeaway:** blocky/lo-fi visuals support fast, ability-heavy combat and quest/dungeon-based campaigns at massive scale; constant unlocks keep players engaged.

---

## 3. Design Takeaways for Voxelforge

1. **Stamina is the central tension.** Every attack, dodge, block, and sprint drains it; running out should be punishing. Bedrock of methodical, readable combat. [[1]](https://eldenring.wiki.fextralife.com/Stamina) [[2]](https://www.theseus.fi/bitstream/handle/10024/877212/Nguyen_Thuan.pdf)

2. **Equip load should change how the game feels, not just numbers.** Light = longer dodge i-frames, faster recovery; heavy = better poise/absorption but sluggish rolls. Make the trade-off tactile. [[3]](https://eldenring.wiki.fextralife.com/Dodging)

3. **Telegraphs must be honest and scaled to threat.** Quick jabs = short wind-ups; heavy strikes = big, layered cues (animation + sound + VFX). Boss phases should evolve patterns without hiding information. [[7]](https://brokenbuildstudios.com/why-enemy-telegraphs-matter-in-game-design/)

4. **Use poise as commitment armor, not just a stun stat.** Fast weapons interrupt easily; heavy swings grant hyper armor so the player chooses when to trade. Adds matchup depth beyond raw DPS. [[5]](https://darksouls3.wiki.fextralife.com/Poise)

5. **Make voxel terrain a combat participant.** Follow Deep Rock: destructible/blocky terrain can create cover, chokepoints, escape routes, and arena reshaping. Different block hardness changes how players use it. [[16]](https://deeprockgalactic.fandom.com/wiki/Terrain)

6. **Curate a structured campaign inside the voxel sandbox.** Hytale and Trove show that blocky worlds can still deliver factions, dungeons, boss events, and seasonal story arcs. Voxelforge should not rely purely on procedural wandering—mark clear narrative beats and set-piece encounters. [[12]](https://hytale.game/en/hytale-the-game/) [[14]](https://gamersnexus.net/news/1609-pax-trove-voxel-rpg-preview)

7. **Lock-on is mandatory for melee readability, free-aim is mandatory for ranged/AOE.** Let players snap to single targets in close combat and release for environment/target switching. A soft-lock or Z-targeting approach fits third-person voxel camera constraints.

8. **Keep the build loop visible early.** Roblox hits succeed because players constantly see new abilities/loot/destinations. Voxelforge should drip new weapons, mobility options, and voxel-building tools into the campaign so mastery and progression feel tangible within the first hour. [[18]](https://bloxmake.com/blog/top-10-roblox-rpg-games-you-must-play-in-2025-)

---

## 4. Prototype Punch List

- [ ] Stamina bar + drain for attack/dodge/block/sprint
- [ ] Dodge roll with i-frames, scaled by equip load
- [ ] Lock-on camera with soft release
- [ ] Light / heavy / charged attack chain
- [ ] Poise system for player and enemies
- [ ] Parry/guard counter window
- [ ] Honest boss telegraphs + 2-phase boss
- [ ] Destructible voxel terrain with varying hardness
- [ ] Third-person camera that reads well on blocky geometry
- [ ] First story beat + mini-dungeon within 30 minutes

---

## Sources

1. [Elden Ring Wiki — Stamina](https://eldenring.wiki.fextralife.com/Stamina)
2. [Nguyen, T. — How to create a good Souls game (Theseus, 2023)](https://www.theseus.fi/bitstream/handle/10024/877212/Nguyen_Thuan.pdf)
3. [Elden Ring Wiki — Dodging](https://eldenring.wiki.fextralife.com/Dodging)
4. [Dark Souls Wiki — Poise](https://darksouls.wiki/books/mechanics/page/poise)
5. [Dark Souls 3 Wiki — Poise](https://darksouls3.wiki.fextralife.com/Poise)
6. [The Drastik Measure — Windrose PC Review](https://www.thedrastikmeasure.com/2026/05/11/windrose-pc-review/)
7. [Broken Build Studios — Why Enemy Telegraphs Matter](https://brokenbuildstudios.com/why-enemy-telegraphs-matter-in-game-design/)
8. [Punished Backlog — The First Berserker: Khazan Review](https://punishedbacklog.com/the-first-berserker-khazan-review/)
9. [Game Rant — Black Myth: Wukong Boss Fights](https://gamerant.com/black-myth-wukong-boss-fights-how-work-transformations/)
10. [Game8 — No Rest for the Wicked Review](https://game8.co/articles/reviews/no-rest-for-the-wicked-review-early-access)
11. [Hytale — Combat System](https://hytale.game/en/combat-system/)
12. [Hytale — The Game](https://hytale.game/en/hytale-the-game/)
13. [Trove Wiki — Class](https://trove.fandom.com/wiki/Class)
14. [GamersNexus — Trove Voxel RPG Preview](https://gamersnexus.net/news/1609-pax-trove-voxel-rpg-preview)
15. [GamingOnLinux — Deep Rock Galactic destructible environment](https://www.gamingonlinux.com/2019/03/deep-rock-galactic-a-1-4-player-co-op-fps-with-a-destructible-environment-works-great-in-steam-play/)
16. [Deep Rock Galactic Wiki — Terrain](https://deeprockgalactic.fandom.com/wiki/Terrain)
17. [Escapist — Best Jujutsu Kaisen Games on Roblox](https://www.escapistmagazine.com/best-jujutsu-kaisen-games-on-roblox/)
18. [BloxMake — Top 10 Roblox RPG Games 2025](https://bloxmake.com/blog/top-10-roblox-rpg-games-you-must-play-in-2025-)
19. [gamespace — 2025's Best Roblox Fighting and Adventure Games Ranked](https://gamespace.com/all-articles/news/2025s-best-roblox-fighting-and-adventure-games-ranked/)
20. [Destructoid — Best Classes in World // Zero](https://www.destructoid.com/world-zero-classes-guide-tier-list/)
21. [Dungeon Quest Roblox Wiki — Dungeon](https://dungeonquestroblox.fandom.com/wiki/Dungeon)
22. [Heroes' Legacy Wikia — Classes](https://heroes-legacy.fandom.com/wiki/Classes)
