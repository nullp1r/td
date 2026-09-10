# Current Architecture Direction

> **Status:** settled architecture, implementation may vary by checkout

The complete original architecture document is preserved in `99_archive/original_design/MVP-0 Rust Architecture.md`.

## Core runtime truth

- SQLite is authoritative mutable game state.
- TDLib/Telegram is input + presentation.
- Content definitions are read-only game data.
- Transactions are the concurrency authority for meaningful mutations.

## Persistence

Durable state includes character progression, inventory/items, catches/provenance, discoveries, locations, active encounters, and durable timers.

Do not use Telegram message history as authoritative game state.

## Timer rule

In-memory scheduling may wake the application, but the authoritative timer/deadline lives in SQLite.

A restart must not invalidate important encounters/progress merely because a Tokio sleep disappeared.

## Encounter idempotency

Callbacks contain compact identifiers/steps. The application validates ownership, active phase, and current step against SQLite.

Stale/double callbacks should resolve harmlessly rather than duplicate rewards or corrupt state.

## Reaction timing

The most important presentation/state invariant:

> Start the player's reaction window only after the actionable Telegram presentation succeeds.

Network/client latency before the button exists is not player reaction time.

## Content

Species/bait/rod/location data should be data-driven where clearly useful. Validate at startup and use efficient read-only runtime representations.

Do not create a general condition scripting DSL until repeated content patterns actually require it.

## Refactored application boundaries

The technical-debt pass split the previous `app.rs` monolith into concrete responsibilities:

- angling;
- character/world;
- economy;
- progression/history;
- social.

This was specifically chosen instead of generic `Service`/`Repository` layers.

## Telegram boundary

Telegram was similarly separated into:

- lifecycle/update ingestion;
- callback wire protocol;
- dispatch;
- presentation submodules.

Presentation DTOs are separate from mutation logic.
