# Architectural foundations

## Locked: one canonical world with explicit scope

There is one canonical world. Mutable state must make its scope explicit instead of relying on accidental table ownership or UI convention.

Relevant scopes include:

- **global** — world-wide state/events;
- **local** — region/location/object state;
- **party** — state shared by a temporary consensual group;
- **personal** — character-specific knowledge, encounter resolution or progression.

A shared condition may expose personal opportunities. Example: a canonical low tide exposes quay stones for everyone while each character may resolve an ordinary search encounter independently. Conversely, a genuinely unique washed-up crate may be one shared object for which only one canonical open transition can win.

## Locked: account and character separation

Telegram identity belongs to an account layer; in-world state belongs to characters. The data model must support one account owning multiple characters even if the MVP initially exposes only one playable character.

Account concerns include Telegram routing identity, preferences, moderation/entitlements and notification policy. Character concerns include in-world name, location, inventory, skills/capabilities, knowledge, tasks, reputation, relationships, guild membership and records.

Telegram display name may seed a character-name suggestion but is not the fictional identity or primary key.

## Locked: geography is more than a menu

World geography combines:

- hierarchical containment (world/region/location/etc.);
- coordinates for spatial relationships and regional systems;
- explicit route edges for actual traversability.

Coordinates can support distance, maps, weather regions and proximity. Routes encode what can actually be travelled, including duration, requirements, hazards, transport mode and temporary closure. Geometric proximity never implies traversability.

## Locked: real timestamps plus durable meaningful events

Persist authoritative time using absolute timestamps. Fictional/calendar presentation may later transform those timestamps, but persistent world correctness does not depend on a private player clock.

Use interval/timestamp state rather than constant simulation ticks where possible. Delayed state such as travel records `started_at`/`arrives_at`; schedulers are notification machinery, not simulation truth.

Keep ordinary current-state tables. Do **not** fully event-source the database. Emit durable domain events for meaningful history such as discoveries, significant acquisition, trade, death, records, task completion, location discovery, world events and party formation.

These events may feed histories, NPC reactions, contextual traces, notifications, profiles and social cards.

## Locked: structured definitions are separate from localized rich text

Structured game definitions reference text/content keys rather than embedding final English everywhere. Dialogue and knowledge articles should render through the same internal rich-text/document layer used by Telegram presentation rather than storing TDLib types in game state.

The hard decision is the boundary, not the syntax of the authored files. Exact formats remain open.

## Implemented foundations to preserve

The current codebase already contains principles worth carrying through the reset:

- SQLite is authoritative mutable game state; Telegram is presentation/input.
- Meaningful mutations commit transactionally before Telegram output.
- Delayed gameplay is durable database state rather than sleeping Tokio tasks.
- Duplicate/stale callbacks are expected and guarded by persistent encounter state/step semantics.
- Reaction windows start only after actionable Telegram state is successfully presented.
- Content is validated at startup.
- Procedural outcomes that must persist are seed-driven/versioned.

The new architecture may replace the concrete fishing tables/types while preserving these invariants.

## Rejected foundations

- Compatibility layers solely to keep pre-reset migrations/content/schema alive.
- Full event sourcing without a demonstrated requirement.
- Telegram/TDLib generated types as domain persistence objects.
- Every physical thing globally exclusive by default.
- Every interaction privately instanced by default.
- A continuously ticking simulation when timestamp reconciliation is sufficient.
