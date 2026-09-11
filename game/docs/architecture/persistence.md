# Persistence, concurrency and durable history

## SQLite is authoritative

Current mutable game truth lives in SQLite. Telegram messages, Tokio tasks and in-memory timers are not substitutes for durable state.

The current runtime uses one controlled SQLite worker/connection path rather than an ORM/pool abstraction with no demonstrated need. The connection enables foreign keys, WAL journal mode and `synchronous = FULL`; changing those durability/concurrency choices should be deliberate and measured rather than incidental.

## Transaction authority

Meaningful multi-step mutations should be atomic.

Current examples include:

- consume bait + create fishing encounter + first durable timer;
- catch item/history + discovery/world-first + XP/progression;
- objective reward + claim marker;
- contract specimen removal + claim + reward;
- crafting specimen removal + `craft_consumptions` history + produced bait;
- group `(chat, cycle, character)` claim + persistent catch.

Do not acknowledge success to Telegram before the authoritative transaction has committed.

## Durable timers

Important delayed state lives in the `timers` table.

The timer driver may sleep in memory, but an in-memory notification/sleep is only a scheduling optimization. SQLite is the authority.

Consequences:

- process restart does not erase a bite/deadline;
- waking the driver means “re-read the DB,” not “this event definitely exists”;
- stale timer rows are validated against entity/step state before mutation;
- a bounded drain prevents a timer backlog from monopolizing the runtime.

## Presentation/reaction ordering

For reaction-sensitive gameplay:

> persist that the action became available / start the deadline only after Telegram successfully presents the actionable state.

The current `mark_presented` flow protects the player from losing reaction time to network/server delivery latency.

## Encounter idempotency

`fishing_encounters` stores a monotonic `step`/phase and callbacks carry the expected state.

An application action validates:

- character ownership;
- active encounter;
- phase;
- expected step;
- deadline where applicable.

Duplicate/stale deliveries must fail harmlessly rather than duplicate rewards/consumption.

## Catch/item history

`items` represents ownership/usability; `catches` preserves the individual catch record.

A specimen can be removed from usable inventory while its catch/provenance remains.

Current removal/history concepts:

- sale (`removal_kind = 1`);
- Harbor contract (`removal_kind = 2`);
- crafting/material use (`removed_at_ms` plus `craft_consumptions`; removal kind intentionally null in current implementation).

This distinction is important for records, provenance, future trophies/trading and world history.

## Discovery history

- `discoveries` — per-character first discovery of a typed subject;
- `global_discoveries` — first known character to discover a typed subject.

Do not infer world-first/history from chat history.

Use explicit history/provenance tables for facts the game actually needs. Do **not** turn the database into a generic event-sourced log and rebuild all current state from every mutation; telemetry/observability is separate from authoritative game history. Likewise, avoid a second mutable in-memory cache becoming another source of truth. Immutable validated content is the main intentional cache.

## Group uniqueness

`group_event_claims` uses `(chat_id, cycle, character_id)` as its primary key. This is the authority for one participation per character per shared shoal.

The public button is shared by everyone; uniqueness must therefore be server/database state, not disabled-button UI state.

## Schema version

Current SQLite `PRAGMA user_version = 5`.

### v1 — core

- accounts/characters;
- stack inventory;
- individual items/catches;
- discoveries/global discoveries;
- fishing encounters;
- durable timers.

### v2 — progression geography/equipment

- discovered character locations;
- owned rods;
- fishing special encounter ID;
- catch item removal timestamp (originally sale timestamp).

### v3 — world/progression loop

- generalized `removed_at_ms`;
- removal reason for sale/contract;
- rod condition;
- objective claims;
- contract claims.

### v4 — social/crafting/identity

- equipped character title ID;
- craft consumption history;
- group event claims.

### v5 — group reads

- group-shoal approach choice;
- persisted whether that private read matched the shoal behavior.

Schema changes must be forward migrations. Persisted numeric enums/IDs used in existing rows must not be casually renumbered.

## Database transfer/runtime files

A transferred development tree may include `game.sqlite3` plus WAL/SHM and a `.tdx-session` directory. These can contain live mutable development state. Treat these as mutable runtime state, not documentation or source.

Never alter/delete such state merely for a documentation/refactor task. Make deliberate backups before migration experiments.
