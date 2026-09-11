# Current implementation state

This file describes what exists in the current project tree. It is not a roadmap.

The implementation is now an MVP-oriented vertical slice inside the larger product vision in [`vision.md`](vision.md): fishing is the first doorway into Rustwater, not the player's permanent identity or the entire world.

## Runtime and world

Implemented:

- one persistent character per Telegram account;
- XP, levels and coins;
- accelerated deterministic game clock: **one game day = two real hours**;
- deterministic weather slots every **15 real minutes**;
- Clear, Rain and Fog weather; Morning, Day, Evening and Night;
- persistent location discovery/travel;
- persistent species/relic discoveries and global first discoveries;
- SQLite schema version **5** with forward migrations from earlier versions.

Current region:

| ID | Location | Fishable | Role |
|---:|---|:---:|---|
| 1 | Old Harbor | yes | starter hub, Mara Reed, common harbor fishing |
| 2 | Broken Breakwater | yes | deeper encounters and Rusted Key thread |
| 3 | Reed Pond | yes | freshwater/condition-driven catches |
| 4 | Old Lighthouse | no | exploration/progression node |
| 5 | Lighthouse Cove | yes | hidden cove unlocked through the lighthouse thread |

`game/content/game.json` remains authoritative for exact content values and encounter weights.

## Private DM UX

The DM surface is **scene-first rather than system-menu-first**:

- Home leads with current place, weather/day part, current thread and immediate actions;
- new characters see a reduced secondary navigation set rather than the full subsystem sitemap;
- character/gear/progression bookkeeping is collapsed under secondary detail;
- inventory/shop/board/titles/crafting use compact Rich Message tables with in-cell actions, disabled state buttons and `● / ○` selection markers;
- Journal names only discovered species rather than presenting a long table of `???` rows;
- Conditions presents what the character has learned about the current water before exposing exact clock/forecast detail;
- Mara is consistently presented as **Mara Reed · Harbor Warden** and reacts to the implemented lighthouse/relic progression.

The interaction grammar is based on empirical Desktop/Android Rich Message testing documented in [`telegram/rich-messages.md`](telegram/rich-messages.md).

## Message lifetime: panels vs moments

Routine state continues to use one mutable game panel. Memorable facts no longer have to disappear with it.

Current durable-moment behavior:

- a new species;
- a new world-record specimen;
- the Rusted Key;
- a newly discovered location.

Those results remain as chat history and a fresh Home/current-scene panel is sent beneath them. Ordinary catches/navigation continue reusing the current panel. This preserves low chat noise while giving the play session memory.

## Fishing

Implemented:

- 26 species, 7 bait types and 4 rods;
- condition/time/bait-aware encounter selection;
- bait Fishing Power changes wait time independently from attraction weighting;
- durable SQLite timers for bite/deadline state;
- reaction bands (`Excellent`, `Good`, `Late`, `Missed`);
- reaction clock starts only after Telegram successfully presents the actionable state;
- deliberately forgiving first catch;
- persistent rod condition/effective control;
- harder catches can enter a Pull/Give Line struggle;
- deterministic individual specimen length/weight from a stable seed;
- sale value varies with generated specimen;
- personal-best and world-record significance is computed against catch history before insertion;
- stale/duplicate encounter callbacks are rejected authoritatively.

The bite UI now states the real rule: reel quickly. It no longer implies there is a hidden “feel right” timing optimum when faster reaction is mechanically better.

The two social-only species are Echo Herring and Rumor Carp.

## Discovery/world thread

The Rusted Key remains the strongest concrete non-fish thread:

1. discover/travel to Broken Breakwater;
2. fish there until a special encounter surfaces the Rusted Key (with anti-drought support after repeated successful catches);
3. the lighthouse crest changes Mara/exploration context;
4. exploration opens Old Lighthouse;
5. the key opens the lighthouse service route;
6. exploration reveals Lighthouse Cove and its species.

This proves the intended pattern: an ordinary activity can expose an object whose consequence is geography, character context and mystery rather than only currency.

## Mara Reed and Harbor Board

Mara Reed is the concrete recurring Harbor Warden/guide described in [`narrative/mara-reed.md`](narrative/mara-reed.md). There is deliberately no generic dialogue/relationship DSL yet.

Current Harbor Board milestones:

| Objective | Requirement | Reward |
|---|---|---|
| First Haul | catch 5 fish | 25 coins + 20 XP |
| Field Notes | discover 6 species | 5 Glow Larvae + 30 XP |
| Deep Water | land a difficulty-11+ fish at Broken Breakwater | 50 coins + 35 XP |

A deterministic harbor contract changes every fictional game day. It consumes the **smallest** matching stored specimen and awards coins + XP so record specimens are not sacrificed automatically.

## Inventory/economy/records

Implemented:

- usable catch inventory, bait stacks and owned rods;
- catch provenance/history preserved after sale/contract/crafting removal;
- Journal species discovery + personal bests;
- lifetime Records independent of current inventory;
- current world bests;
- title identity with no direct stat power;
- two concrete bait-preparation recipes;
- emergency free worms when every bait stack reaches zero.

Current titles: No title, Angler, Naturalist, Relic Hunter, Harbor Hand, Covekeeper and Shoalbound. `Angler` is an earned title, not the canonical player identity.

## Group/social gameplay

Group `/fish` is now a read-and-choice loop:

- deterministic per-chat **20-minute** shoal cycle;
- one public shared clue card;
- **Read the water** opens a per-user ephemeral decision;
- player chooses **Let it drift** or **Hold steady**;
- each species has a learnable correct approach;
- correct read selects the stronger of two deterministic specimen rolls; wrong read can still land the weaker specimen;
- one claim per character/chat/cycle is enforced in SQLite;
- catches are ordinary persistent character specimens with XP/discovery/record consequences;
- group participation never consumes private rod/bait/coins;
- routine individual detail stays ephemeral; public edits are sparse.

Persistent **chat familiarity** is derived from historical catches in that Telegram chat:

| Historical catches | Standing | Information unlocked |
|---:|---|---|
| 0–4 | Ripple | raw clue |
| 5–14 | Current | collective-memory hint |
| 15–39 | Tidebound | familiar species identified |
| 40+ | Harbor Chorus | mastered read explicitly remembered |

This makes repeated group play change capability/knowledge rather than merely a cosmetic counter.

## Inline sharing

Classic inline mode is implemented for catch cards:

- empty query → recent historical catches;
- text query → species-name filter over recent catches;
- exact `catch:<item_id>` → one owned historical catch;
- private catch results/Records provide switch-to-inline sharing affordances;
- a catch remains shareable after sale, contract turn-in or crafting because the immutable catch ledger persists.

Operational requirement: enable inline mode for the deployed bot with BotFather `/setinline`.

## Telegram commands

Private advertised commands remain deliberately minimal:

- `/start`
- `/help`

Group advertised commands:

- `/fish`
- `/help` (ephemeral path)

Normal play stays button/Rich-Message driven rather than duplicating every screen as a slash command.

## Architecture

- single Rust process;
- `tdx`/TDLib Telegram transport;
- `App` transactional application boundary;
- one controlled SQLite worker/connection path;
- SQLite authoritative for mutable game state;
- immutable validated content in memory;
- durable SQLite timers with in-memory wakeups only as scheduling hints;
- typed domain IDs;
- deterministic pure fishing/world helpers;
- stable callback wire tags;
- view DTOs separating mutation from Telegram presentation.

No ORM, generic repository/service framework, actor system, event bus, generic quest engine, dialogue DSL or condition scripting language has been introduced.

## Verification confidence

The v0→v5 migration chain has been executed successfully against SQLite in the current artifact environment. Documentation integrity is checked with `tools/check-docs.py`.

This artifact environment does **not** contain a Rust toolchain and cannot resolve the public Rust distribution host, so the exact current source tree still requires the Rust validation matrix on a development machine:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets
```

See [`development/diagnostics.md`](development/diagnostics.md).
