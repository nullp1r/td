# Tech-Debt and Refactor History

> **Status:** historical + lessons

## Why the pass happened

Feature velocity had produced an `app.rs` around 3k lines plus oversized Telegram routing/presentation files. The user explicitly requested a refactor guided by root `AGENTS.md`, not merely cosmetic splitting.

## Structural result

The pass separated demonstrated responsibilities instead of inventing generic framework layers.

Application:

- `angling`;
- `character`;
- `economy`;
- `progression`;
- `social`;
- view DTOs/tests separated.

Telegram:

- lifecycle;
- callback protocol;
- dispatch;
- presentation modules.

## Actual debt removed

Beyond file splitting, the pass addressed patterns such as:

- per-species/per-bait N+1 SQLite queries;
- repeated title-unlock queries;
- content linear lookup on build-once tables;
- per-cast temporary weighted allocation;
- growing migration-version match;
- unnecessary one-variant intermediary types;
- speculative unused ID types;
- awkward generated `setCommands` boilerplate.

## Diagnostic follow-up lessons

Static reasoning was not enough. The user's real diagnostics exposed integration issues that only the compiler/rustfmt/Clippy matrix could reliably catch.

This is why future major refactors must always finish with the real diagnostic matrix, preferably more than once after early compiler blockers are fixed.

## Do not regress into refactor-for-refactor's-sake

The root guidance is correct: extract a helper/module when it isolates a real lifecycle/protocol/domain responsibility. Do not fragment straight-line mechanics merely to reduce file length.

## Maintainability pass — 2026-09-10

A later review targeted readability rather than another module split. Production `game/src` went from 5,285 to 5,166 lines while explanatory comment/doc-comment lines went from 7 to 74. The main gains came from removing redundant DB/application plumbing, typed SQL-boundary IDs, shared economy mutations, declarative objectives/recipes, clearer persisted-state boundaries, and iterator/tuple-driven presentation.

The pass deliberately did **not** add repository/service/state-machine frameworks or macro-generate the callback wire protocol. See `GAME_MAINTAINABILITY_PASS_2026-09-10.md` for the complete rationale and verification status.

## Maintainability diagnostic follow-up

The first compiler run against the exact packaged maintainability tree found a narrow integration set rather than a design rollback: missing `Clone` on `Copy` view records, unit-vs-TDLib-response match results, one borrow/move test error, and one non-`Send` formatting temporary across `.await`. The follow-up keeps the refactor structure intact, applies the real rustfmt output, and requires one clean diagnostic rerun before further structural work.
