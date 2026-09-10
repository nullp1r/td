# Current implementation state

This file describes what exists in the current project tree. It is not a roadmap.

The implementation is an early but coherent vertical slice inside the much larger product vision in [`vision.md`](vision.md).

## Runtime and world

Implemented:

- one persistent character per Telegram account in the current schema;
- XP, levels and coins;
- one accelerated deterministic game clock: **one game day = two real hours**;
- deterministic weather slots every **15 real minutes**;
- Clear, Rain and Fog weather;
- Morning, Day, Evening and Night day parts;
- persistent location discovery and travel;
- persistent species/relic discoveries and global first discoveries;
- SQLite schema version **4** with in-place migrations from earlier versions.

The current Rustwater region contains five locations:

| ID | Location | Fishable | Role |
|---:|---|:---:|---|
| 1 | Old Harbor | yes | starter hub, Mara, common harbor fishing |
| 2 | Broken Breakwater | yes | stronger/deeper encounters, Rusted Key thread |
| 3 | Reed Pond | yes | freshwater species and condition-driven catches |
| 4 | Old Lighthouse | no | exploration/progression node |
| 5 | Lighthouse Cove | yes | hidden cove unlocked through the lighthouse thread |

`game/content/game.json` is authoritative for exact content values and encounter weights.

## Fishing

Implemented:

- 26 species;
- 7 bait types;
- 4 rods;
- condition/time/bait-aware encounter selection;
- bait Fishing Power shortening bite wait independently from species-attraction weighting;
- durable bite/deadline timers in SQLite;
- reaction bands (`Excellent`, `Good`, `Late`, `Missed`) rather than subsecond twitch gates;
- the reaction clock begins **only after the actionable Telegram presentation succeeds**;
- deliberately forgiving first catch;
- rod condition and effective-control calculation;
- harder encounters that may enter a short Pull/Give Line struggle phase;
- deterministic individual specimen length/weight from a stable seed;
- specimen sale value that varies with generated size/weight;
- idempotent encounter-step validation against stale/double callbacks.

The current species are:

Harbor Perch, Silver Minnow, Mud Crab, Harbor Mackerel, Glass Minnow, Old Harbor Sturgeon, Breakwater Grouper, Rain Eel, Blackwater Skate, Storm Eel, Reed Carp, Worm Goby, Reed Pike, Fog Loach, Pale Crown Carp, Lighthouse Angler, Signal Eel, Bellglass Ray, Dock Lanternfish, Rustscale Sole, Dawn Reedling, Keeper's Koi, Pale Threadfin, Cove Goby, Echo Herring, and Rumor Carp.

The last two are currently social/group-shoal species.

## Bait and equipment

Current bait:

| ID | Bait | Acquisition |
|---:|---|---|
| 1 | Worm | starter/shop/emergency forage |
| 2 | Bread | shop |
| 3 | Glow Larva | shop/objective reward |
| 4 | Rotten Meat | shop |
| 5 | Silver Spoon | shop |
| 6 | Fish Chunks | crafted |
| 7 | Crab Paste | crafted |

Current rods:

| ID | Rod | Base control | Buy price |
|---:|---|---:|---:|
| 1 | Old Wooden Rod | 10 | starter |
| 2 | Reinforced Rod | 14 | 45 |
| 3 | Light Rod | 11 | 35 |
| 4 | Breakwater Rod | 18 | 95 |

Owned rods have persistent condition. Wear never destroys a rod and currently reduces control modestly. The tackle stall can repair owned rods; repair is blocked during an active encounter so capability cannot change mid-fight.

If every bait stack reaches zero, the player can dig for three worms to avoid a soft lock.

## Discovery and world thread

The Rusted Key is the current non-fish discovery thread:

1. fish at Broken Breakwater;
2. a Rusted Key can be pulled up as a special encounter;
3. successful Breakwater catches anti-drought the key after repeated misses;
4. the key/clue opens the Old Lighthouse progression;
5. exploration reveals Lighthouse Cove and its species.

This is an important proof of concept: fishing can surface a world object whose consequence is exploration and new geography rather than only another specimen.

## NPC, Harbor Board and progression loops

The current Old Harbor NPC is **Mara, Harbor Warden**. Her full canonical identity is now **Mara Reed**; see [`narrative/mara-reed.md`](narrative/mara-reed.md). Current implementation uses concrete progression-sensitive dialogue rather than a generic dialogue engine.

The Harbor Board has three one-time milestones:

| Objective | Requirement | Reward |
|---|---|---|
| First Haul | catch 5 fish | 25 coins + 20 XP |
| Field Notes | discover 6 species | 5 Glow Larvae + 30 XP |
| Deep Water | land a difficulty-11+ fish at Broken Breakwater | 50 coins + 35 XP |

A deterministic rotating harbor contract changes every fictional game day (currently every two real hours). It asks for one ordinary Old Harbor species, consumes the **smallest** matching stored specimen, and grants coins + XP. Better/record specimens are intentionally protected.

## Inventory, records, titles and crafting

Implemented:

- inventory of usable catches, bait and rods;
- catch provenance retained after sale/contract/crafting removal;
- journal with species discovery and personal bests;
- lifetime records independent of whether a specimen remains in inventory;
- current world bests for known species;
- titles as social identity with no direct gameplay power;
- two concrete preparation recipes.

Current titles:

- No title;
- Angler;
- Naturalist;
- Relic Hunter;
- Harbor Hand;
- Covekeeper;
- Shoalbound.

Current crafting:

- **Cut bait** — consumes the smallest stored catch, produces 4 Fish Chunks;
- **Crab paste** — consumes one stored Mud Crab, produces 3 Crab Paste.

Consumed specimens disappear from usable inventory but remain historical catches.

## Social/group gameplay

`/fish` in a group exposes the current shared shoal:

- deterministic per-chat **20-minute** real-time cycle;
- Echo Herring or Rumor Carp;
- one free claim per character/chat/cycle;
- group casts do not consume/wear the player's private rod, bait, or coins;
- every result is still a real persistent specimen with XP/discovery/world-first consequences;
- one shared Rich Message represents the public event;
- **Cast once** and **How it works** are in-message actions;
- routine individual results are per-user ephemeral Rich Messages when supported, with callback toast fallback;
- public card edits are sparse (participation milestones such as 1/2/4/8… or noteworthy outcomes) to avoid chat spam;
- rotation time uses Telegram's native client-updated relative timestamp rather than periodic bot countdown edits;
- participation can unlock Shoalbound.

## Telegram UX currently implemented

Private advertised commands are intentionally minimal:

- `/start`
- `/help`

Normal fishing, exploration, travel, inventory, shop, crafting, journal, records, titles, conditions, NPC interaction, and Harbor Board navigation happen through native UI rather than a duplicate slash-command sitemap.

Group advertised commands:

- `/fish`
- `/help` (ephemeral command path)

`/game` and the old telemetry-heavy group panel are removed.

Rich Messages use semantic hierarchy, embedded contextual buttons, and a conventional inline keyboard primarily for navigation/secondary actions. Deeper screens use semantic parent + Home rather than a stateful browser-history stack.

## Architecture currently implemented

- single Rust process;
- `tdx`/TDLib for Telegram transport;
- `App` as transactional application boundary;
- one controlled SQLite worker/connection path;
- SQLite authoritative for mutable gameplay state;
- immutable validated content loaded in memory;
- durable SQLite timers with in-memory wakeups only as scheduling hints;
- typed domain IDs at application/query boundaries;
- deterministic pure fishing/world helpers;
- explicit stable callback wire tags;
- view DTOs separate game mutation from Telegram presentation.

No ORM, generic repository/service framework, actor system, event bus, cache/entity graph, generic quest engine, dialogue DSL, or condition scripting language exists.

## Verification confidence

The most recent external Rust-toolchain diagnostics available in this project showed the workspace build/test configurations passing before a final narrow strict-Clippy/rustfmt cleanup. The current source includes that cleanup, but this exact post-cleanup tree has not been rerun with Rust in the artifact environment used for documentation work.

Run the matrix in [`development/diagnostics.md`](development/diagnostics.md) whenever current verification confidence matters. Generated `diagnostics*.txt` files are transient outputs, not project documentation.
