# `tdx` Evolution Principles for the Game

> **Status:** settled technical preference

## `tdx` is part of the product stack

The user owns `tdx` and explicitly stated that any `tdx` or other `td`-family change is acceptable when it improves the game cleanly.

Do not write awkward game-level workarounds merely to avoid changing `tdx`.

## Existing successful examples

During the refactor, repeated command-registration boilerplate led to small `tdx::command` helpers for generated `botCommand`/`setCommands` requests.

The later UX pass extended the same principle to Rich Message button rows/styles, ephemeral callback responses, and ephemeral-command replies when the group-help flow demonstrated the need.

The 2026-09-10 formatting pass then removed an ergonomic mismatch in the wrapper itself: tuple/array/`Vec` composition is now shared by ordinary and rich text, tables are headerless by default with optional `.header(...)`, and native client-updated relative timestamps replace stale countdown strings. See `TDX_FORMATTING_COMPOSITION_2026-09-10.md` for the settled API and rationale.

## What good `tdx` helpers look like

- serve a demonstrated caller;
- stay thin over generated TDLib types;
- preserve editability/access to the generated request;
- avoid policy that belongs to the game;
- avoid builders/wrappers for hypothetical future consumers;
- reduce common error-prone generated-API boilerplate;
- stay dependency-light.

Formatting helpers should additionally follow one composition rule instead of maintaining compatibility syntaxes. Tuples represent fixed heterogeneous composition; arrays and vectors represent homogeneous composition. Use macro helpers internally for tuple-implementation boilerplate, not public formatting macros.

## Schema freshness

This project tracks Telegram/TDLib aggressively. Generated API shapes can change even between relatively close TDLib versions.

When a Telegram feature exists but local generated types do not:

1. inspect current upstream `td_api.tl`;
2. run `./td/fetch` / repository generation flow;
3. update `tdx` ergonomics if needed;
4. update callers/tests together.

Backward compatibility with stale generated APIs is not a primary goal.
