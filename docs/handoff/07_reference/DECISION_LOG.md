# Chronological Decision Log

> **Status:** historical rationale index

## Foundation

- Player is an adventurer; fishing is the first activity.
- Hybrid explicit/implicit discovery chosen.
- World geography should be real even if UI travel is compressed.
- Game clock separate/accelerated; real-world events can coexist.
- Capability progression preferred.
- Failure tuned closer to Terraria/WoW than Minecraft catastrophic loss.
- Procedural individual specimens encouraged.

## Character

- XP + level implemented.
- Level intentionally non-power initially.
- No eventual universal mastery.
- Exact stats/traits/talents left open.
- Multiple characters deferred.

## Fishing

- Telegram rate limits treated as design constraint.
- Broad timing windows.
- Many observation variants.
- Bait power + hidden preferences/side effects.
- Anything may eventually become bait.
- Difficult catches can have multi-step struggle.

## Architecture

- SQLite selected.
- `tdx`/TDLib is first-class.
- `tdx` and other `td` crates may change freely for better design.
- Bleeding-edge Rust explicitly preferred.
- Concrete implementations before generic DSL/framework.

## Product/playtesting

- First private-friends prototype, then broader core-mechanics MVP, then progressively wider audience.
- After enough broad questioning, developer/AI can choose sensible prototype answers instead of reopening everything.

## UX correction

- Player UI must not be developer documentation.
- Rich formatting/emojis should be used deliberately.
- In-message buttons are primary actions.
- Commands are entry points, not navigation.
- Back + Home should be common.
- Group play should use ephemeral personalized responses rather than force DMs.
- `/game` replaced conceptually by `/fish`.

## `tdx` formatting correction — 2026-09-10

- Native relative date/time entities replace bot-edited countdown refresh loops when Telegram can own the live rendering.
- Tables are headerless by default; headers are opt-in.
- Tuples are the fixed heterogeneous composition primitive in ordinary text, rich text, and table rows.
- Arrays and `Vec`s cover homogeneous composition.
- Composition operators, `plain`, `concat`, and `empty` were removed rather than retained as compatibility syntax.
- Public formatting macros/builders and `.inline()`-style methods were rejected; internal macros are used only for repetitive tuple trait implementations.
- Ordinary multiline text should stream into one destination buffer.

See `04_telegram/TDX_FORMATTING_COMPOSITION_2026-09-10.md`.

## Game maintainability correction — 2026-09-10

- Every game source module has a responsibility-level module doc; comments are for invariants/contracts/policy, not narration.
- Production `game/src` must remain smaller after the readability pass; the measured result is 5,285 → 5,190 lines while comment/doc-comment lines rise 7 → 72.
- SQLite values become typed domain IDs at the query boundary.
- `Db` transports jobs/results; `App` owns application error policy.
- Closed gameplay definitions such as objectives and recipes should be declarative while persisted numeric IDs remain explicit/stable.
- Callback byte IDs and persisted state tags stay explicit where auditability is more valuable than macro-generated brevity.
- Do not introduce repositories, state-machine frameworks, compatibility layers, or one-use helpers merely to shorten files.

See `05_technical/GAME_MAINTAINABILITY_PASS_2026-09-10.md`.

