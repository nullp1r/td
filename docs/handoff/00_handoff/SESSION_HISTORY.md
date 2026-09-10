# Session History

> **Status:** chronological historical record

## 1. Initial concept

The project started as an advanced text-based MMO RPG inside Telegram, heavily inspired by Terraria's fishing but intended to become much broader. The long-term picture included a vast world, biomes, locations, dimensions, per-chat events, global events, player specialization, and eventually absurd late-game contexts such as catching exotic things with specialized devices in space or other dimensions.

The desired interaction surfaces were already multi-client from the beginning:

- DMs for personal encounters;
- group chats for social/public interactions;
- a future Mini App for richer inventory/crafting/map/shop interactions;
- potentially a website later.

Monetization through Telegram Stars/crypto and eventual NFT gift integration were mentioned as future possibilities, not MVP requirements.

## 2. Foundation decisions

The early questioning established the high-level principles now preserved in `Game Design Foundation`:

- hybrid explicit/implicit discovery;
- spatially coherent world;
- accelerated game clock separate from real-world time;
- capability progression;
- persistent history/world firsts;
- moderate failure rather than catastrophic gear destruction;
- player/environment interaction;
- procedural individual specimens;
- a spectrum of session lengths.

Levels were eventually chosen for MVP, but intentionally as a visible investment/seniority number with no direct gameplay power yet.

## 3. Fishing interaction design

The session developed a much deeper fishing model than “tap Cast, get random fish.” Key ideas included:

- broad reaction timing, not latency-hostile millisecond gates;
- multiple observational text variants;
- species behavior and imperfect information;
- bait power plus species preferences/side effects;
- equipment control;
- multi-step struggle for difficult encounters;
- individual deterministic specimens;
- environment-sensitive pools;
- durable state/idempotency.

A full original specification is preserved in `99_archive/original_design/Fishing System Specification v0.md`.

## 4. Repository and architecture phase

The user provided the public `nullp1r/td` repository/`tdx` branch and asked for architecture to align heavily with root `AGENTS.md`.

Important explicit technical choices:

- SQLite;
- `tdx`/TDLib rather than Bot API wrappers;
- bleeding-edge Rust;
- no hesitation to modify `tdx`/other `td` crates;
- dependency additions judged by real value and weight;
- future scale considered without speculative frameworks.

The design settled on SQLite as authoritative mutable game state, with Telegram/TDLib as input and presentation.

## 5. First playable builds and painful UX bugs

Early playtesting immediately revealed that “working code” was not enough. The user exhausted worms and found themselves trapped in an interface with essentially only Cast/Catch. They also reported:

- no meaningful navigation;
- commands not registering correctly;
- group commands not behaving;
- bot apparently only responding meaningfully in DMs;
- a confusing repeated fishing loop.

This became an important precedent: **soft-lock prevention and navigation are gameplay requirements**, not polish.

Emergency worm foraging, fuller navigation, command registration, and broader panels were added in subsequent iterations.

## 6. Progression/world expansion

The prototype gained:

- Rustwater geography;
- exploration/location unlocks;
- Rusted Key mystery;
- lighthouse/cove progression;
- Mara, Harbor Warden;
- environmental conditions panel;
- one-time objectives;
- rotating specimen contract;
- rod condition/repair;
- meaningful bait power;
- preservation of catch provenance after sale/turn-in.

This phase reached 24 species and schema v3.

## 7. Social/collection expansion

The next phase reached 26 species and schema v4, adding:

- lifetime records/world bests;
- non-power social titles;
- specimen-consuming crafted bait;
- social-only Echo Herring and Rumor Carp;
- per-chat rotating shoals;
- transactional once-per-character participation;
- rate-limit-aware sparse public group updates.

The initial group-result UX still used callback toasts and a telemetry-heavy group card.

## 8. Tech-debt/refactor phase

The growing prototype accumulated a ~3k-line `app.rs` and large Telegram presentation/router files. The user explicitly requested a debt pass and provided `docs/tmp/DIAGNOSTIC_REPORT.md`.

The refactor split application responsibilities into concrete modules rather than generic service/repository layers, reduced N+1 SQL patterns, simplified content lookup, and added a small `tdx::command` helper because generated `setCommands` boilerplate was a real consumer need.

## 9. Real compiler diagnostics

The user ran the requested diagnostics script locally. The environment reported `rustc 1.100.0-nightly` from 2026-09-03.

The next assistant pass fixed concrete errors/Clippy/rustfmt fallout from the refactor, including missing imports, private-test coupling, ambiguous collect target, non-Send `format_args!` across await, strict `absolute_paths`, and 98 rustfmt hunks.

A second diagnostics pass was still recommended because early compiler failures can mask later diagnostics.

## 10. UX reckoning

The user then identified the most important remaining problem: the prototype felt like software written for its developer rather than a game written for players.

Specific complaints:

- `/help` was a giant developer-facing wall of text;
- `/game` in groups was mysterious and intimidating;
- `/help` and `/game` effectively did the same thing in groups;
- Rich Messages were dramatically underused despite `tdx` work;
- almost no tasteful emoji accents;
- promised in-message buttons had not appeared;
- navigation lacked Back/semantic hierarchy;
- private command catalog duplicated the interface;
- group play was still too DM-centric;
- Telegram's newest features were not being exploited enough.

This led to a major UX redesign.

## 11. Lost/unpackaged UX implementation

The assistant verified and began implementing current Telegram features:

- Rich Message button rows and button styles;
- ephemeral group messages visible only to one user;
- styled conventional markup helpers;
- `tdx` helpers around callback-triggered ephemeral sending;
- group `/fish` + `/help` redesign;
- private per-user group catch results;
- Home/fishing/catch/struggle/exploration hierarchy rewrite;
- semantic Back/Home navigation;
- private command reduction toward `/start` + `/help`;
- player-facing help rewrite.

The work was **not packaged** before the chat had to be rewound, so these changes must be reimplemented/finished from the documented design rather than assumed present.

## 12. `tdx` formatting ergonomics and game maintainability — 2026-09-10

Playtesting exposed stale plain-text countdowns, fake empty table headers, and awkward `concat([plain(...)])` rich composition. `tdx` was redesigned around tuple composition, headerless-by-default tables, and Telegram-native relative timestamps. The user subsequently found one compiler issue in the new `Styled<T>: IntoRichText` implementation; the corrected implementation now defines `into_rich_text` and delegates `append_to` through it.

The user then requested a systematic readability pass over the game crate: the code had almost no explanatory comments and several application paths were difficult to follow. The resulting source review covered every production module, removed redundant plumbing/dead projection state, clarified database/domain boundaries and persisted state-machine invariants, consolidated repeated economy mechanics, made recipes/objectives declarative, and reduced presenter boilerplate. Production `game/src` decreased from 5,285 to 5,157 lines while explanatory comment/doc-comment lines increased from 7 to 74.

The pass intentionally stopped short of generic repositories/state-machine frameworks or macro-generated callback protocol values. Compiler/rustfmt/Clippy validation remains the user's next gate because the artifact environment has no Rust toolchain.

The first real diagnostics from that exact maintainability handoff passed tree verification and exposed a small integration set: `Copy`/`Clone` derives, side-effect match return types, a tuple test borrow/move conflict, one spawned non-`Send` formatting lifetime, and rustfmt drift. Those issues were repaired without changing gameplay, schema, migrations, or content; the resulting candidate awaits one clean diagnostic rerun.

## 13. Final stabilization diagnostics

A final real diagnostics run from an exact verified handoff tree passed all default/all-features/no-default build and test configurations. Its only failing gates were rustfmt and strict Clippy. The session-closing source fixes applied every reported issue: one `tdx` test assertion lint, dead `Rod.description`, test-only `CastStarted`, by-value timer snapshots, two absolute paths, callback semicolon style, two trailing format commas, one manual empty `String`, and the final rustfmt layout hunk.

The final artifact still needs one toolchain rerun because the packaging environment has no Rust compiler. After a green rerun, work should return to playtest-driven product development rather than another broad cleanup pass. `SESSION_CLOSE_2026-09-10.md` is the canonical resume document.
