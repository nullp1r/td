# Current State at the End of the Session

> **Status:** historical handoff + next-state target
> **Important:** verify the actual checkout before relying on any implementation claim.

## Closest known implementation lineage

The project progressed through several increasingly complete prototype slices:

1. Basic Telegram fishing prototype.
2. Navigation/command repairs after early playtesting exposed an almost closed `Cast → Catch → Cast` loop and DM-only/command-registration problems.
3. A broader progression slice with world exploration, Mara, tasks/contracts, conditions, bait power, rod wear/repair, and richer persistent history.
4. A social/collection slice with records, titles, crafting, social-only fish, and per-chat shoals.
5. A large technical-debt/refactor pass splitting the original monolithic application/Telegram files and removing N+1 query patterns.
6. A real compiler/Clippy/rustfmt diagnostics follow-up against `rustc 1.100.0-nightly (2026-09-03)`.
7. A **player-UX redesign** that implemented new Telegram Rich Message button and ephemeral-message support in a working tree, but was never packaged before the conversation had to be rewound.

## Known database/content milestone before the UX pass

The most complete packaged gameplay slice before the later UX work used schema **v4** and included, at minimum:

- account/character persistence;
- XP + level, with level intentionally non-power-bearing for now;
- 5 Rustwater locations;
- 26 species, including 2 social-only species;
- individual catch provenance and deterministic specimens;
- weather + accelerated game time;
- location/discovery journal;
- Rusted Key → lighthouse → hidden cove progression;
- Mara, Harbor Warden;
- three one-time harbor milestones;
- one rotating harbor contract per game-day cycle;
- shop/economy;
- multiple rods, rod condition, low-cost repair;
- shop bait + crafted bait;
- records/world bests;
- non-power titles;
- specimen-consuming bait preparation while preserving catch history;
- per-chat rotating group shoals with once-per-character participation;
- global discoveries/world firsts;
- durable SQLite timers and idempotent encounter steps.

See `05_technical/IMPLEMENTATION_STATUS.md` and `02_game_design/CONTENT_AND_FEATURE_STATUS.md` for a fuller matrix.

## Refactor status

The large refactor replaced the original ~3k-line `app.rs` with concrete modules around demonstrated responsibilities:

```text
app/
  angling
  character
  economy
  progression
  social
  tests
```

Telegram responsibilities were similarly split between runtime, callback protocol, dispatch, and presentation submodules.

The refactor also moved several repeated database reads into grouped/snapshot queries and switched build-once/read-many content tables toward sorted vectors/binary search.

## Diagnostics-fixed refactor artifact

A later compiler run exposed and then fixed concrete integration problems caused by the refactor. A reply reported an archive named:

`td_refactored_diagfix1.zip`

with SHA-256:

`141490095991c7af42c02cd5b8f0de02aa194fa68df4aa98ad15561d40a0a3a2`

The fixes included:

- missing `require_character_id` import;
- missing `SpeciesId` / `game_day` test imports;
- removing a test dependency on private `contract_species` rather than widening it;
- explicit `Vec<_>` collection target in progression;
- fixing a non-`Send` callback future caused by `format_args!` living across `.await`;
- stale import cleanup;
- `clippy::absolute_paths` fixes in both `tdx` tests and analogous game code;
- applying all 98 rustfmt hunks emitted by the real diagnostic run.

A second diagnostic matrix was still requested because the first compiler failure could have masked later errors.

## UX redesign and `tdx` formatting status

The lost Telegram UX pass was reimplemented and packaged on 2026-09-09, then audited against the user's original UX request. The user confirmed the first restored UX build worked before the subsequent audit changes.

The current source tree additionally contains the 2026-09-10 `tdx` formatting redesign:

- tuple composition for heterogeneous ordinary and rich text;
- array/`Vec` composition for homogeneous ordinary and rich text;
- single-destination-buffer `lines(...)` composition;
- composition `Add`/`AddAssign`, `plain`, `concat`, and `empty` removed;
- headerless `table()` with optional `.header(...)`;
- heterogeneous tuple table rows with `cell(...)` reserved for explicit layout metadata;
- native `dateTimeFormattingTypeRelative` rendering through `relative_time(...)`;
- group-shoal countdown migrated to an absolute Telegram relative timestamp plus fallback text.

See `04_telegram/TDX_FORMATTING_COMPOSITION_2026-09-10.md` for the exact API and rationale.

The user later found one concrete formatting-trait integration error while playtesting: `Styled<T>: IntoRichText` needed an explicit `into_rich_text` implementation in addition to `append_to`. That correction is incorporated in the current tree.

## Maintainability pass — 2026-09-10

The current tree also contains a crate-wide readability/maintainability pass over `game`. It reduced production `game/src` from 5,285 to 5,157 lines while increasing comment/doc-comment lines from 7 to 74. The pass removed redundant database/application plumbing, moved persisted values toward typed IDs at SQL boundaries, consolidated repeated bait/economy behavior, made small gameplay tables declarative, clarified durable timer/angling invariants, reduced presenter boilerplate, and added responsibility-level docs to every source module.

See `05_technical/GAME_MAINTAINABILITY_PASS_2026-09-10.md` for exact decisions and rejected abstractions.

Two real diagnostic cycles were then run from exact verified handoff trees. The latest run passed all default/all-feature/no-default **build and test** gates; the remaining failures were rustfmt plus strict Clippy style/dead-state findings. The current tree applies every actionable finding from that report, including removing the unused `CastStarted` projection and unused rod-description payload rather than suppressing warnings. See `SESSION_CLOSE_2026-09-10.md` for the authoritative final status.

## Immediate rule for the next developer

Run `python3 tools/handoff.py verify-tree` and `tools/collect-diagnostics.sh` on this final packaged tree. If the matrix is green, continue from new playtest evidence. Do **not** re-do the UX, `tdx` composition redesign, or maintainability pass unless diagnostics or playtesting expose a real regression.
