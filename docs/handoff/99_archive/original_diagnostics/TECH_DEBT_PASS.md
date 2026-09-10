# Tech-debt pass — September 2026

This pass started from `docs/tmp/DIAGNOSTIC_REPORT.md` and then reviewed the current `game` and the directly affected `tdx` surface under the root `AGENTS.md` rules.

## Diagnostic report status

The reported compiler blockers were fixed first:

1. The ragged struggle keyboard no longer relies on incompatible fixed-size array rows.
2. Private-chat command registration now supplies the generated optional command scope correctly.
3. Group-chat command registration now supplies the generated optional command scope correctly.
4. Group command dispatch no longer calls unstable/redundant `str::as_str` on `&str`.
5. Private command dispatch no longer calls unstable/redundant `str::as_str` on `&str`.

The reported Clippy/lint issues were also removed:

- the manual divisibility test uses the standard integer API;
- bool-to-integer conversion uses `u32::from`;
- the test-only raw `panic!` was removed;
- the unfulfilled lint expectation on `CastStarted::due_at_ms` was removed;
- no replacement broad lint suppression was added.

The Telegram-only manifest dependencies are optional behind the `telegram` feature, so non-Telegram builds do not unnecessarily activate them.

The workspace `rust-version` is `1.98.1`.

## Application structure

The old ~3,100-line `game/src/app.rs` was removed. The largest current production game module is under 600 lines; the old ~1,000-line Telegram presentation monolith is also gone. Application behavior is now grouped by demonstrated responsibilities:

```text
game/src/app/
├── mod.rs          shared application types, transaction helpers, catch recording
├── angling.rs      fishing lifecycle, reaction/struggle resolution, timers
├── character.rs    account/character state, world movement/exploration
├── economy.rs      inventory, shop, rods, bait, selling, crafting
├── progression.rs  discoveries, records, tasks, contracts, titles, NPC state
├── social.rs       per-chat shoal/world-status mechanics
└── tests.rs        application integration tests
```

Application result/view DTOs live in `game/src/view.rs`, rather than being mixed into mutation code.

This is intentionally not split into `Service`, `Repository`, generic event-bus, actor, or internal framework layers. SQLite remains the one mutable-state implementation and concrete game operations remain concrete.

## Telegram structure

The old ~1,000-line presentation file and ~500-line mixed runtime/router file were split into the actual Telegram responsibilities:

```text
game/src/telegram/
├── mod.rs              TDLib lifecycle, update ingestion, timer presentation
├── callback.rs         compact callback wire protocol
├── dispatch.rs         Telegram input -> application operations
└── present/
    ├── mod.rs          shared presentation helpers
    ├── core.rs         home/inventory/help
    ├── fishing.rs      cast/bite/struggle/catch panels
    ├── progression.rs  shop/tasks/NPC/titles/crafting
    ├── social.rs       group shoals
    └── world.rs        locations/conditions/journal/records
```

The receive loop remains a hot ingestion boundary. Game/database work is dispatched out of it, and Telegram remains input/presentation rather than authoritative game state.

## `tdx` change

Command registration exposed repeated generated-API boilerplate that belongs at the TDLib application boundary, so `tdx::command` now contains two small helpers:

- `command::definition(...)` builds a generated `botCommand`;
- `command::set(scope, commands)` builds a generated `setCommands` with the optional scope represented correctly.

The returned request remains the generated TDLib request and is still directly editable. No parallel wrapper model was introduced.

A `tdx` test verifies that the generated registration request remains editable and contains the intended scope/command.

## Persistence and query cleanup

SQLite remains authoritative and owned by one dedicated worker thread.

Changes in this pass include:

- migrations are an ordered table instead of a growing version `match`;
- each migration is applied transactionally;
- a database newer than the binary and an invalid negative schema version have explicit errors;
- worker-thread creation preserves the original `io::Error` as the source;
- repeated bait/rod/discovery/title lookups were replaced with compact snapshot queries;
- the title panel derives all unlock inputs from one database snapshot instead of issuing a query per title;
- journal and record views use grouped history queries instead of per-species SQL loops;
- world status obtains its related counts in one SQLite statement;
- catch provenance remains authoritative even after sale, contract use, or crafting.

No ORM, SQL framework, connection pool, cache, or repository-trait layer was added.

## Content/runtime cleanup

Content is parsed once, sorted once, validated once, and then read through binary search on compact vectors.

The fishing hot path no longer allocates a temporary weighted encounter vector on every cast. Selection performs a two-pass weighted draw over the validated encounter slice.

Small build-once/read-many ID snapshots similarly use sorted vectors and binary search rather than hash collections where hashing does not buy anything.

## Gameplay-state cleanup

The two oversized application operations left after the initial split were decomposed by real behavior:

- exploration snapshots known locations once and delegates the actual finding decision to a pure helper;
- reel resolution separates relic resolution, fish resolution, and final persisted catch creation.

Reaction-window semantics remain unchanged: a reaction deadline starts only after the actionable Telegram presentation succeeds.

Durable gameplay delays remain represented in SQLite; Tokio sleeps only drive the durable timer table and are never the authoritative timer state.

## Other cleanup

- speculative unused ID newtypes were removed;
- the one-use/one-variant intermediary abstractions found during the pass were collapsed;
- shared helpers were moved to the nearest common module ancestor rather than widening them with `pub(super)`/`pub(crate)`;
- the changed `game`/`tdx` source contains no `#[allow(...)]` or `#[expect(...)]` debt suppressions;
- no new generic framework or compatibility layer was introduced.

## Static verification performed in the artifact environment

This environment does not contain `rustc`, `cargo`, `rustfmt`, or `clippy`, and external toolchain download is unavailable. Therefore this report does **not** claim compiler verification.

The following independent checks were run successfully after the final edits:

- root, `game`, and `tdx` manifests parse as TOML;
- fresh and every existing v1/v2/v3 database schema migrate to schema v4;
- 146 directly extractable SQL statements in `game/src` prepare successfully against the v4 SQLite schema;
- the bundled content JSON validates all current cross-references and numeric invariants for 26 species, 7 baits, 4 rods, and 5 locations;
- Rust source delimiter/comment/string balance passes across 60 `game`/`tdx` source/test files;
- no changed Rust line exceeds the repository's 160-column hard ceiling;
- no `pub(crate)`/`pub(super)` widening exists in the changed game/command-helper code;
- no `panic!`, `todo!`, `unimplemented!`, broad `#[allow]`, or `#[expect]` suppression remains in the changed production surface;
- the source tree contains no runtime SQLite/session artifacts or real Telegram credentials.

## Required local verification

The repository-wide acceptance commands from the root `AGENTS.md` still need to be run on a machine with the configured Rust 1.98.1/nightly toolchain:

```sh
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

Any diagnostics from those commands should be treated as the authoritative final gate. This static pass was deliberately structured to minimize the likely remaining compiler/Clippy surface, but it is not a substitute for those commands.

## Compiler diagnostic follow-up

A full local diagnostic run was subsequently provided from WSL2 using `rustc 1.100.0-nightly (2026-09-03)`, Cargo 1.100.0-nightly, rustfmt 1.10.0-nightly, and Clippy 0.1.100. The run exercised formatting, default/all-feature workspace builds and tests, strict workspace Clippy, the game without default features, and the dependency feature graph.

The first diagnostic iteration exposed refactor integration errors that static validation could not detect. This follow-up fixes them:

- `character.rs` again imports the shared `require_character_id` helper after the module split;
- stale imports left by the split were removed;
- application tests explicitly import `SpeciesId` and `game_day` and no longer reach across a module boundary for the private `contract_species` helper;
- journal species collection has an explicit `Vec<_>` target so inference is unambiguous;
- the group callback constructs the owned TDLib callback-answer request before `.await`, so `fmt::Arguments` does not live across the spawned future suspension point;
- the `tdx` command-helper test imports `command` instead of using Clippy-denied absolute paths;
- analogous absolute paths in game command registration and two game types were proactively shortened before the next Clippy run;
- the rustfmt diff emitted by the diagnostic run was applied across the affected game/tdx files.

The artifact environment still lacks a Rust toolchain, so these fixes require one more local diagnostic run to expose any diagnostics that were previously masked by compilation failure. The local run remains the authoritative acceptance gate.
