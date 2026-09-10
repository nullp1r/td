# Real Compiler/Clippy Diagnostic Follow-Up

> **Status:** historical record of fixes applied after user-supplied diagnostics

The user ran the full diagnostics locally. The reported environment used:

`rustc 1.100.0-nightly` from **2026-09-03**.

The next assistant pass reported fixing the concrete failures below.

## Compiler/type/import fixes

- `character.rs` had lost the `require_character_id` import during the module split.
- `app/tests.rs` had lost `SpeciesId` and `game_day` imports.
- One test depended on private `contract_species`; the test was changed to verify contract behavior without widening the private helper.
- `progression.rs` had an ambiguous `.collect()` target; changed to explicit `Vec<_>`.

## Async/Send fix

A spawned callback future became non-`Send` because `format_args!` (`fmt::Arguments`) survived across `.await`.

The fix constructed the owned TDLib callback-answer request **before** awaiting. This preserved the allocation-conscious `tdx` API without carrying non-Send formatting arguments through a suspension point.

This is an important pattern for future formatting optimizations in spawned async work.

## Strict warning/Clippy cleanup

- removed stale `StruggleAction`, `Content`, `BaitId`, and `RodId` imports;
- fixed `tdx` command-helper test for `clippy::absolute_paths`;
- proactively fixed analogous absolute paths in game code, including `tdx::command`, `crate::ids::EncounterId`, and `crate::content::EncounterWeight` call sites/types as applicable.

## Rustfmt

The diagnostic output contained **98 rustfmt hunks**. Those were applied, including overlaps with compiler fixes.

## Validation caveat

A second full diagnostics run was explicitly requested because the first compile stopped early; fixing early errors can expose previously masked compiler/Clippy failures.

If the user's current tree derives from this fixed archive, run the current diagnostics matrix before doing structural work rather than reapplying these historical patches.
