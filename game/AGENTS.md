# Repository guidance

Apply the engineering principles from `nullp1r/td`'s `AGENTS.md` unless a demonstrated game requirement requires otherwise.

## Project invariants

- Track current Rust aggressively. Current MSRV: 1.98.1. Nightly features are welcome when they produce a concrete correctness, clarity, efficiency, or line-count benefit.
- Backward compatibility is not a goal during early development. Prefer the clean end state and update callers/tests/content together.
- Start from demonstrated gameplay/application needs. No speculative frameworks, traits, wrappers, compatibility layers, generic event buses, ORMs, actor systems, or distributed infrastructure.
- SQLite is authoritative for mutable game state. Telegram is input/presentation, never game truth.
- Meaningful game mutations are transactional. Telegram output occurs only after commit.
- Delayed gameplay is durable database state. Sleeping Tokio tasks are never authoritative timers.
- Callback/timer delivery is treated as duplicate/stale-capable. Encounter `step` values guard state transitions.
- Reaction timing starts only after the actionable Telegram state is successfully presented. Server-side output delay must not consume the player's reaction window.
- Keep Telegram/TDLib types out of game/domain state. Render committed game state through `tdx` at the boundary.
- Depend on `tdx` directly. If a generally useful Telegram-boundary capability is missing, improve the `td` family instead of building an application-local compatibility wrapper.
- Content is validated at startup. Invalid cross-references or impossible ranges are startup errors, not runtime fallbacks.
- Random procedural outcomes are seed-driven and versioned where persistence matters.
- Prefer compact data, typed IDs, flat storage and direct SQL over pointer-rich object graphs or repository abstractions.
- Preserve errors and ordering. Expected gameplay failures are values/results; infrastructure failures are errors.
- Keep public API surface tiny. This is an application, not a framework.
- Give every source module a short responsibility-level module doc. Add comments for invariants, persistence/protocol contracts, ordering requirements, and non-obvious policy; do not narrate self-evident statements.
- Keep entry methods at one abstraction level. Extract a private step when it gives a state transition, SQL projection, or domain decision a useful name; do not split straight-line code merely to shorten functions.
- Decode SQLite integers into typed domain IDs at the query boundary. Unwrap IDs back to raw integers only when binding persistence/protocol fields that actually store integers.
- Prefer declarative static definitions for small closed gameplay tables (objectives, recipes, titles) over repeated control-flow branches, while keeping persisted numeric IDs explicit and stable.
- Treat production line count as design feedback. Readability work should remove duplicated mechanics and obsolete state rather than hiding complexity in helpers or macros solely to make files shorter.

## Verification

Before committing meaningful changes, run:

```sh
cargo fmt --all --check
cargo check --all-targets
cargo test
cargo clippy --all-targets
```

Credentialed Telegram integration tests should stay isolated/ignored and must close TDLib sessions gracefully.
