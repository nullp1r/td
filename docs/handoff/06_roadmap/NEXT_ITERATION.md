# Next Iteration — Recommended Plan

> **Status:** diagnostics-fix candidate; rerun + playtest

The UX/social Telegram pass, `tdx` tuple-formatting redesign, and 2026-09-10 game maintainability pass are implemented. The first real diagnostics from the exact maintainability handoff have been repaired. The next iteration should be a **clean diagnostics rerun followed by playtesting**, not another structural rewrite.

## 0. Verify this exact handoff

From the extracted project root:

```sh
python3 tools/handoff.py verify-tree
./tools/collect-diagnostics.sh
```

Do not act on compiler output unless `verify-tree` first confirms the source tree matches the embedded handoff manifest.

## 1. What the first diagnostics repaired

The exact-tree run found and this candidate fixes:

- missing `Clone` alongside `Copy` on three scalar view records;
- side-effect Telegram `match` arms accidentally returning TDLib response values;
- a tuple-composition test borrowing and moving the same `Text`;
- a spawned callback future made non-`Send` by formatting temporaries surviving into an awaited presenter expression;
- the rustfmt/EOF-whitespace diff produced by the real toolchain.

The user's required `Styled<T>: IntoRichText::into_rich_text` correction remains present. No gameplay/content/schema changes were made in this repair.

## 2. Playtest the Telegram UX

Exercise at least:

- Home → Cast → Bite → Catch/Struggle → Cast again;
- exploration/travel and the Rusted Key progression;
- Inventory, tackle stall, repairs, bait selection, crafting;
- Mara, Harbor Board objectives/contracts, Journal, Records, Titles;
- group `/fish`, in-message help, ephemeral catch results, Journal/Records from the group result;
- headerless tables on current Telegram clients;
- native relative shoal timestamp updating while the message sits idle.

Treat confusion, stale presentation, inaccessible navigation, or misleading player copy as product defects. Preserve the maintainability constraints in `05_technical/GAME_MAINTAINABILITY_PASS_2026-09-10.md`.
