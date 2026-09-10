# Next Iteration — Recommended Plan

> **Status:** current handoff recommendation

The UX/social Telegram pass, `tdx` tuple-formatting redesign, and 2026-09-10 game maintainability pass are implemented in source. The next iteration should be **diagnostics + playtesting**, not another structural rewrite.

## 0. Verify that diagnostics are from this exact handoff

From the extracted project root:

```sh
python3 tools/handoff.py verify-tree
./tools/collect-diagnostics.sh
```

`verify-tree` exists because one earlier diagnostic report was accidentally collected from an older checkout. Do not act on compiler output until the source tree matches the embedded handoff manifest.

## 1. Repair compiler/rustfmt/strict-Clippy fallout

Pay particular attention to the maintainability changes around typed SQLite IDs, `Db::job`/`App::run_db`, private fixture constants, angling timer state, and group-shoal transactions. The artifact environment cannot compile Rust.

The user already identified and fixed one real `tdx` issue: `Styled<T>: IntoRichText` must implement `into_rich_text` explicitly; that fix is present in this tree.

## 2. Playtest the Telegram UX

Exercise at least:

- Home → Cast → Bite → Catch/Struggle → Cast again;
- exploration/travel and the Rusted Key progression;
- Inventory, tackle stall, repairs, bait selection, crafting;
- Mara, Harbor Board objectives/contracts, Journal, Records, Titles;
- group `/fish`, in-message help, ephemeral catch results, Journal/Records from the group result;
- headerless tables on current Telegram clients;
- native relative shoal timestamp updating while the message sits idle.

Treat confusion, stale presentation, inaccessible navigation, or misleading player copy as product defects.

## 3. Re-run diagnostics after fixes

A green compiler/test/rustfmt/Clippy matrix is the gate before adding more systems. Preserve the maintainability constraints from `05_technical/GAME_MAINTAINABILITY_PASS_2026-09-10.md`: no speculative frameworks, keep persisted protocol/state mappings explicit, and do not widen production APIs merely for tests.
