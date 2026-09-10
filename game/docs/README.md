# Rustwater game documentation

This directory is the durable knowledge base for the game: what the product is trying to become, what exists today, which decisions are intentional, which questions are still open, and which technical invariants must survive implementation changes.

It is **not** a changelog, session transcript, archive registry, or prediction of what somebody must build next.

## Start here

A developer or AI with no previous conversation context should read, in order:

1. [`vision.md`](vision.md) — the finished-game north star and design philosophy.
2. [`current-state.md`](current-state.md) — what the code and content implement now.
3. [`decisions.md`](decisions.md) — durable product/engineering decisions and their status.
4. [`open-questions.md`](open-questions.md) — intentionally unresolved choices; do not “fix” them by guessing.
5. The subsystem documents relevant to the work.

For character/art work, read [`narrative/mara-reed.md`](narrative/mara-reed.md) and [`art/visual-direction.md`](art/visual-direction.md) early. Mara Reed is the current visual face of the game.

For Telegram/client work, read [`ux/principles.md`](ux/principles.md), [`ux/interaction-surfaces.md`](ux/interaction-surfaces.md), [`telegram/platform.md`](telegram/platform.md), and [`telegram/tdx.md`](telegram/tdx.md).

For backend/game-state work, start with [`architecture/overview.md`](architecture/overview.md) and [`architecture/persistence.md`](architecture/persistence.md).

## Status vocabulary

The docs use these words deliberately:

- **Implemented** — present in the current source/content/schema.
- **Invariant** — current behavior or architecture that must not change accidentally.
- **Direction** — part of the intended finished product, but not a promise that it is implemented now.
- **Provisional** — a current implementation/design that is expected to evolve.
- **Open** — intentionally undecided; evidence or playtesting should resolve it later.
- **Platform fact** — externally verified Telegram behavior; include a source and verification date.

When a document discusses both current and future design, it must label the distinction clearly.

## Documentation is part of the change

Updating relevant documentation is mandatory when code or design changes alter any of these:

- player-visible behavior or UX;
- game rules, progression, content semantics, world/lore canon;
- persistence/schema/concurrency invariants;
- Telegram/TDLib assumptions;
- public `tdx` conventions used by the game;
- a settled decision or open question.

Do **not** create a session log merely to prove documentation was touched. Edit the durable document so it describes the current truth. Conversation chronology, temporary archive names, hashes, and debugging incidents are normally noise.

If source and docs disagree, inspect the implementation and the intended decision, then reconcile them immediately. Code is authoritative for what currently executes; these docs are authoritative for durable intent and context that code alone cannot express.

## Package map

- [`vision.md`](vision.md) — product north star, inspirations, scope philosophy.
- [`current-state.md`](current-state.md) — implemented vertical slice and verification confidence.
- [`decisions.md`](decisions.md) — settled/provisional decisions.
- [`open-questions.md`](open-questions.md) — unresolved design space.
- [`world.md`](world.md) — spatial/historical world model and discovery philosophy.
- [`gameplay/fishing.md`](gameplay/fishing.md) — current fishing mechanics and long-term encounter model.
- [`gameplay/progression.md`](gameplay/progression.md) — XP, capabilities, specialization, titles, reputation direction.
- [`gameplay/items-economy.md`](gameplay/items-economy.md) — specimens, provenance, bait, crafting, economy.
- [`gameplay/social.md`](gameplay/social.md) — MMO/social design and current group shoals.
- [`narrative/characters-and-relationships.md`](narrative/characters-and-relationships.md) — NPC, lore, dialogue, reputation, relationship and romance direction.
- [`narrative/mara-reed.md`](narrative/mara-reed.md) — canonical Mara Reed character/mascot specification.
- [`narrative/character-spec-template.md`](narrative/character-spec-template.md) — compact consistency template for future recurring characters.
- [`art/visual-direction.md`](art/visual-direction.md) — Rustwater visual identity and reusable prompt language.
- [`art/generative-media.md`](art/generative-media.md) — AI-art pipeline principles and art-unlock mechanic direction.
- [`ux/principles.md`](ux/principles.md) — player-facing interaction principles.
- [`ux/interaction-surfaces.md`](ux/interaction-surfaces.md) — DM, group, inline, Guest Mode, keyboards, Mini App and sharing strategy.
- [`telegram/platform.md`](telegram/platform.md) — verified Telegram capabilities and constraints.
- [`telegram/tdx.md`](telegram/tdx.md) — project-specific `tdx` conventions and evolution rules.
- [`architecture/overview.md`](architecture/overview.md) — code/runtime architecture.
- [`architecture/persistence.md`](architecture/persistence.md) — SQLite, transactions, durable timers, history/provenance.
- [`architecture/content-and-determinism.md`](architecture/content-and-determinism.md) — content validation, typed IDs, RNG/versioning.
- [`development/workflow.md`](development/workflow.md) — collaboration, docs discipline and project transfer.
- [`development/diagnostics.md`](development/diagnostics.md) — reproducible verification matrix.
- [`development/playtesting.md`](development/playtesting.md) — how to test the product rather than merely the code.
- [`reference/glossary.md`](reference/glossary.md) — project terms.

There is deliberately no “next task” document. Future work is chosen from current evidence, ideas, and priorities at the time.
