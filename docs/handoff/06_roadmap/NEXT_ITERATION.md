# Next Iteration — Recommended Plan

> **Status:** final session handoff; verify once, then resume playtesting-driven development

The restored Telegram UX, `tdx` tuple-formatting redesign, native relative-time/table fixes, and game maintainability pass are integrated. The latest real toolchain run passed all build/test configurations and exposed only rustfmt/strict-Clippy cleanup; every reported finding is repaired in the final handoff tree.

## 0. Verify this exact handoff

From the extracted project root:

```sh
python3 tools/handoff.py verify-tree
./tools/collect-diagnostics.sh
```

Do not diagnose a different checkout. `verify-tree` must pass first. The artifact environment that produced the ZIP has no Rust compiler, so one real rerun is still required.

## 1. If diagnostics are green

Do **not** start another broad refactor. Continue from the user's newest playtest findings. In particular, exercise:

- Home → Cast → Bite → Catch/Struggle → Cast again;
- exploration/travel and Rusted Key → lighthouse progression;
- Inventory, tackle stall, repairs, bait selection, crafting;
- Mara, Harbor Board objectives/contracts, Journal, Records, Titles;
- group `/fish`, in-message help, ephemeral catch results, Journal/Records from the group result;
- headerless tables on current Telegram clients;
- native relative shoal timestamps while a message sits idle.

Treat confusing navigation, stale presentation, inaccessible actions, awkward copy, unnecessary chat noise, and missing opportunities to use Telegram-native UI as product bugs.

## 2. If diagnostics still fail

Fix only the concrete failures first. Preserve the decisions in:

- `00_handoff/SESSION_CLOSE_2026-09-10.md`;
- `04_telegram/TDX_FORMATTING_COMPOSITION_2026-09-10.md`;
- `05_technical/GAME_MAINTAINABILITY_PASS_2026-09-10.md`.

Do not reintroduce removed formatting compatibility APIs (`plain`, `concat`, composition `Add`/`AddAssign`, old header-taking `table(...)`) or widen game architecture merely to silence tests/lints.
