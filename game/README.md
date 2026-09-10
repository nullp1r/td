# Telegram MMO — MVP-0 social + collection slice

This package is a Telegram-native RPG slice built directly on `tdx`, with SQLite as authoritative mutable state.

The current loop is roughly:

`/start → explore Rustwater → fish/react/struggle → collect specimens → discover species → keep records/titles → sell/contract/craft specimens → improve tackle → find the Rusted Key → open Lighthouse Cove → join rotating group-chat shoals`

It is still one deliberately small region, but it now has several overlapping progression loops instead of a single fishing state machine.

## Current gameplay

### World and exploration

- **5 canonical locations**:
  - Old Harbor;
  - Broken Breakwater;
  - Reed Pond;
  - Old Lighthouse;
  - Lighthouse Cove.
- Only Old Harbor is known initially.
- `Explore` discovers the nearby Breakwater and Reed Pond.
- Undiscovered locations cannot be travelled to through stale/forged callbacks.
- The Old Lighthouse and Lighthouse Cove are unlocked through the Rusted Key chain.
- Old Lighthouse is deliberately non-fishable: locations can exist for world/progression purposes rather than being fishing menus.

### Fishing

- **26 species** loaded from validated JSON content, including two social-only species that do not appear in solo location encounter tables.
- Weighted species pools by location.
- Weather, fictional game time, bait preference, bait Fishing Power, rod control, rod condition, and species conditions matter.
- Fishing Power now shortens bite delay; species-specific bait multipliers separately influence *what* bites.
- Durable SQLite bite/deadline timers.
- Reaction windows begin **only after the actionable Telegram edit succeeds**.
- Broad reaction bands rather than millisecond-perfect gates.
- First catch remains deliberately forgiving.
- Powerful species can enter a second **struggle phase**:
  - `Pull`;
  - `Give line`;
  - readable behavioral clue;
  - stale/double interaction protection through encounter steps.
- Deterministic individual size/weight generation.
- Large specimens sell for more than ordinary specimens of the same species.

### Conditions / environmental knowledge

`/conditions` exposes a compact planning surface:

- exact fictional game time;
- current day part;
- current weather;
- next deterministic weather state and time until change;
- time until the next day-part transition;
- currently active species **only among species the character has already discovered**.

Unknown species remain unknown. This is intended to make knowledge itself useful without turning the game into an exposed probability table.

### Mara / Harbor Warden

Old Harbor now has the first concrete NPC, **Mara, Harbor Warden**.

Her dialogue changes with world progression:

- before major exploration;
- after finding the Rusted Key;
- after learning about the lighthouse;
- after opening Lighthouse Cove.

She also points the player toward the current harbor contract and milestone board. This is deliberately concrete rather than a generic dialogue/quest engine.

### Harbor board: one-time milestones

The **Harbor Board** includes three persistent objectives whose progress is derived from real game history:

1. **First Haul** — catch five fish.
2. **Field Notes** — discover six species.
3. **Deep Water** — land a difficulty-11+ fish at Broken Breakwater.

Rewards are transactionally claimable once. They currently grant combinations of coins, XP, and Glow Larvae.

### Rotating harbor contract

Every fictional game day (currently two real hours), the harbor board requests one ordinary Old Harbor species.

- The request rotates deterministically among accessible common species.
- A matching **stored individual specimen** is required.
- A contract can be completed once per cycle.
- Turn-in consumes the **smallest** matching stored specimen automatically, protecting better/record specimens.
- The specimen is removed from usable inventory but its catch/provenance record remains in SQLite.
- Contract rewards grant coins + XP.

This is the first repeatable progression loop beyond free fishing/economy grinding.

### Rusted Key / lighthouse mystery

Broken Breakwater fishing can eventually drag up:

**Rusted Key** — *Corroded almost beyond recognition. A faded lighthouse emblem is still visible beneath the salt.*

The key has a small chance to appear early. If it does not, actual successful Breakwater fishing anti-droughts it: after four ordinary catches there, the next cast becomes the key encounter.

No character-level gate is involved. Levels remain informational/seniority progression in this build.

Following the clue unlocks Old Lighthouse and then Lighthouse Cove, which now has several distinct conditional species plus an always-available common resident.

### Character / progression

- Account → character persistence.
- XP and levels.
- Level currently does **not** gate or strengthen gameplay.
- Coins.
- Persistent discovery journal.
- Personal-best weights per species.
- Location-discovery progress.
- Relic discovery tracking.
- Transactional world-first species/relic records.

### Equipment and economy

- 4 rods with distinct control, permanent ownership, purchase prices, condition, and equipping.
- Rod condition wears slowly on successful catches; harder catches wear more.
- Condition never destroys a rod and only reduces effective control modestly.
- The tackle stall repairs all owned rods for a deliberately low cost.
- Repair is blocked during an active fishing encounter so encounter capability cannot change mid-fight.
- 7 bait types with Fishing Power, species preferences, and selection. Five are shop bait; Fish Chunks and Crab Paste are crafted-only.
- Catch value scales with generated specimen size/weight.
- Selling or contract-turning a catch no longer deletes provenance:
  - removed catches disappear from usable Inventory;
  - catch history and personal records remain;
  - `items.removed_at_ms` records removal;
  - `items.removal_kind` distinguishes sale vs contract turn-in.
- If every bait stack reaches zero, `Dig for 3 worms` remains an emergency no-soft-lock path.

### Records and titles

`/records` is a lifetime catch ledger rather than an inventory view:

- lifetime catch count and landed mass;
- heaviest personal specimen;
- personal bests per species;
- current world best for those species;
- sold, contract-turned, and crafted specimens remain part of the record.

The **Titles** screen adds non-power social identity. Current titles are derived from real persistent accomplishments such as catch count, species discovery, the Rusted Key, harbor milestones, Lighthouse Cove, and participation in a shared group shoal. An equipped title appears on Home but grants no gameplay power.

### Tackle preparation / crafting

The **Tackle Preparation** screen introduces the first deliberately small crafting loop without a generic recipe engine:

- **Cut bait** consumes the smallest stored catch and produces 4 Fish Chunks;
- **Crab paste** consumes one stored Mud Crab and produces 3 Crab Paste;
- crafted bait cannot be bought from the shop;
- consumed specimens leave usable Inventory but remain in historical records via `craft_consumptions`.

This makes catches useful as inputs instead of reducing every specimen to sale value.

### Group-chat shared shoals

`/fish` in a group creates the chat-local fishing activity.

- Every group chat has a deterministic shoal that rotates every 20 real minutes.
- The current social species is either **Echo Herring** or **Rumor Carp**.
- Every character can cast into that chat's shoal once per cycle.
- A successful group cast creates a real persistent individual specimen, discovery, XP award, and world-first record when applicable.
- Participation is enforced transactionally by `(chat_id, cycle, character_id)` uniqueness.
- The shared Rich Message contains native **Cast once** and **How it works** buttons. Help therefore remains discoverable without relying on the slash-command menu. Routine results are delivered as per-user ephemeral Rich Messages inside the group, with a callback toast as the fallback when ephemeral delivery fails.
- The public shoal card is edited only at sparse participation milestones (1, 2, 4, 8…) or a world first, avoiding group-chat message spam.
- Joining a shoal unlocks the **Shoalbound** title.

### Telegram surfaces

Private command registration is intentionally small:

- `/start` — create/open the character and enter the main interface;
- `/help` — player-facing onboarding.

Fishing, exploration, inventory, travel, progression, records, crafting, shop, conditions, NPC interaction, and titles are reached through the native interface rather than a duplicate slash-command sitemap. Primary contextual actions use buttons embedded directly in Rich Messages; deeper panels use semantic parent/Home navigation.

Groups/supergroups register:

- `/fish` — show/start the shared shoal activity;
- `/help` — an ephemeral command that privately explains group fishing in player language.

`/game` and the old world-telemetry group panel are removed. The public `/fish` card explicitly says the shared cast is free and does not touch the player’s rod, bait, or coins. Group catches remain persistent character catches while routine presentation stays private to the participant; the ephemeral result also exposes Journal and Records without forcing a DM detour.

## Persistence / schema

Schema version is now **4**.

`0002_progression.sql` added location/rod ownership, fishing special encounters, and historical catch disposition.

`0003_world_loop.sql` adds/changes:

- renames `items.sold_at_ms` → `items.removed_at_ms`;
- `items.removal_kind` (`1 = sold`, `2 = contract`);
- `character_rods.condition`;
- `objective_claims`;
- `contract_claims`.

`0004_social_crafting.sql` adds:

- `characters.title_id`;
- `craft_consumptions` for historical specimen use;
- `group_event_claims` for per-chat/per-cycle social participation.

Fresh databases and existing schema-v1/v2/v3 databases migrate to v4 in place. Back up a development DB you care about before running a new build anyway.

## Architecture invariants retained

- SQLite is authoritative mutable game state.
- Telegram/TDLib is input + presentation, not game truth.
- Meaningful mutations are transactional.
- Delayed gameplay state lives in durable SQLite timers.
- Callbacks are compact and validated against authoritative owner/encounter/step state.
- Reaction timing starts after successful presentation.
- Rich Message presentation is separate from gameplay state.
- No ORM, actor framework, event bus, cache layer, generic quest engine, or condition DSL has been added.
- Objectives, Mara, contracts, and the Rusted Key chain are concrete because they are the first demonstrated consumers.

## Configuration

Set credentials in the environment. Do not commit them to `run` or the repository.

```sh
export TELEGRAM_API_ID=...
export TELEGRAM_API_HASH=...
export TELEGRAM_BOT_TOKEN=...

# Optional:
export GAME_DB=game.sqlite3
export GAME_CONTENT=content/game.json
export TDLIB_SESSION=.tdx-session
```

Then:

```sh
./game/run
```

or from the workspace root:

```sh
cargo run -p game --release
```

## Local verification

Run with the repository's current Rust toolchain:

```sh
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

The artifact environment used to edit this package still has no Rust toolchain, so those commands cannot be run here. In this environment I validate migrations/content/SQL independently and do a manual Rust pass; local Cargo checks remain required before trusting the build.
