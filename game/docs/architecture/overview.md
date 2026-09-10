# Architecture overview

## Goal

Keep the current game compact and understandable while preserving the invariants that let it grow into a much larger persistent world.

The architecture is intentionally **not** a speculative MMO framework.

## Runtime shape

Current high-level flow:

```text
Telegram / TDLib
      │
      ▼
  tdx transport
      │
      ▼
telegram dispatch ──► telegram presentation
      │                    ▲
      ▼                    │
     App ───────────────► view DTOs
      │
      ▼
 SQLite worker  ◄──── immutable validated content
      │
      └──── durable timers / persistent world state
```

Key properties:

- single Rust process;
- TDLib owns Telegram networking/session behavior;
- `tdx` provides ergonomic typed Telegram operations;
- `game::telegram` handles lifecycle/update routing/callback protocol/presentation;
- `App` is the transactional game/application boundary;
- SQLite is authoritative mutable game state;
- JSON content is immutable validated game definition data;
- durable deadlines/timers live in SQLite;
- deterministic world/fishing helpers are pure where practical.

## Source map

### Application/game state

- `src/app/mod.rs` — shared App boundary, application error type, common DB helpers and catch/progression primitives.
- `src/app/angling.rs` — private fishing persisted state machine/timers.
- `src/app/character.rs` — character bootstrap, world/location/exploration/conditions.
- `src/app/economy.rs` — inventory, rods, bait, sales, repairs, preparation/crafting.
- `src/app/progression.rs` — journal, records, titles, Mara, Harbor Board objectives/contracts.
- `src/app/social.rs` — group-chat shoals and transactional per-cycle claims.

These concrete modules are preferred over generic `Service`/`Repository` layers.

### Pure/domain definitions

- `src/content.rs` — content structs, loading, sorting/validation.
- `src/fishing.rs` — deterministic selection/timing/control/specimen mechanics.
- `src/world.rs` — deterministic accelerated time/weather.
- `src/rng.rs` — deterministic SplitMix64-based RNG.
- `src/ids.rs` — typed domain IDs.
- `src/view.rs` — transport-agnostic presentation projections.

### Persistence/runtime

- `src/db.rs` — one dedicated SQLite worker/connection path.
- `src/timer.rs` — durable timer driver; in-memory wake signals only invalidate sleeps.
- `migrations/*.sql` — authoritative schema evolution.

### Telegram

- `src/telegram/mod.rs` — lifecycle/update ingestion/command setup.
- `src/telegram/callback.rs` — compact callback wire encoding/decoding.
- `src/telegram/dispatch.rs` — update/action orchestration.
- `src/telegram/present/*` — Rich Message/player-facing presentation.

## State boundary

Game mutation belongs behind `App`; Telegram presenters receive view values rather than owning SQL/game logic.

Do not let Telegram message state become the canonical game state. The current fishing encounter row stores the Telegram `chat_id`/`message_id` needed to edit that encounter's active panel, but those values are **presentation routing metadata**, not proof that the encounter exists or is actionable. SQLite encounter/step/phase state remains authoritative.

This coupling is acceptable for the current one-client vertical slice. If Rustwater gains multiple simultaneous clients/panels or replacement-message recovery that makes the binding lifecycle independently meaningful, move presentation bindings into their own concrete persistence model rather than spreading Telegram identifiers deeper into the game domain. Do not add that abstraction before a real consumer requires it.

## Error boundary

`App::run_db` centralizes the application/error flattening around database-worker jobs. The DB worker transports arbitrary `Send` results; it does not encode game-specific error policy.

Keep infrastructure errors and domain/application errors explicit enough for presenters to render useful outcomes without creating a hierarchy of generic service errors.

## Typed IDs

Decode persisted integer identifiers into typed domain IDs (`LocationId`, `SpeciesId`, `BaitId`, `RodId`, etc.) near the query boundary. Convert back to raw integers only where persistence/protocol fields actually require them.

This keeps state-machine/domain code from accidentally mixing semantically different `u32`s.

## Activity exclusivity

The current game uses the existence of an active fishing encounter as a concrete conflict guard: travel, exploration and rod repair cannot mutate incompatible state mid-encounter.

As genuinely different long-lived physical activities appear (travel, combat, expeditions, exploration, etc.), avoid growing a bag of unrelated booleans such as `is_fishing` / `is_traveling` / `is_exploring`. Once a second real activity proves the need, model mutually incompatible character activity/state explicitly at the application/persistence boundary. Do not generalize the current fishing guard before then.

## Callback wire protocol

Callback numeric tags are deliberately explicit rather than macro-generated. Already-sent Telegram messages can outlive a process/build, making callback bytes a persistent protocol surface.

When changing callback encoding:

- preserve existing tag meaning unless old messages are intentionally invalidated;
- validate encounter ID/step/ownership/phase against SQLite;
- keep payload compact;
- stale/double callbacks must not duplicate consequences.

## Observability

The current binary uses structured `tracing`, including TDLib log/error bridging and timer/action error context. Keep operational logs useful for state-transition/timing problems without logging credentials or complete Telegram message contents by default. IDs such as character/chat/encounter/timer and the action/transition are generally more useful diagnostic context than raw conversation text.

As gameplay timing becomes more nuanced, instrumentation should support empirical tuning rather than guessing about Telegram delivery latency; see [`../development/playtesting.md`](../development/playtesting.md).

## What is deliberately absent

Do not add these simply because the final game will be large:

- ORM;
- generic repository/service interfaces;
- actor framework;
- event bus;
- generic entity/component graph;
- cache layer without measured need;
- generic quest/dialogue engine;
- condition scripting DSL;
- generalized state-machine framework.

Add abstraction after repeated concrete consumers reveal stable common semantics.

## Scaling direction

For a long time, useful scaling comes from:

- data-driven content;
- efficient immutable lookup;
- compact typed state;
- transactions/idempotency;
- deterministic generation;
- preserved history/provenance;
- Telegram message coalescing;
- separating public/shared and private/personal presentation.

If SQLite eventually becomes the bottleneck, migrate storage while preserving the application invariants described in [`persistence.md`](persistence.md) rather than leaking Telegram transport concerns into the model.
