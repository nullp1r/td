# Rustwater / Telegram MMO — Session Handoff Documentation

> **Purpose:** preserve the product, game-design, UX, Telegram-platform, architecture, implementation-history, and user-preference context accumulated during the long design/implementation session.
>
> **Audience:** future human developers and AI coding agents.
>
> **Placement:** this archive is designed to be extracted into `game/docs/`.

## Read this first

The repository root `AGENTS.md` remains the highest-priority engineering guidance. It is intentionally not duplicated here because it should evolve in one place.

For a new developer or AI agent, read in this order:

1. `00_handoff/START_HERE.md`
2. `00_handoff/CURRENT_STATE.md`
3. `00_handoff/USER_PREFERENCES.md`
4. `01_product/PRODUCT_VISION.md`
5. `01_product/DECISIONS.md`
6. `03_ux/UX_PRINCIPLES.md`
7. `05_technical/ARCHITECTURE.md`
8. The subsystem document relevant to the task.
9. `99_archive/` only when deeper historical rationale is useful.

## Status vocabulary

Documents use four kinds of status:

- **Settled direction** — treat as a real project decision until consciously changed.
- **Current target** — intended next-state design, but implementation may lag.
- **Open** — deliberately undecided; do not invent permanence prematurely.
- **Historical** — records what happened, including implementations that may no longer exist in the current checkout.

A major lesson from this session is that **code state and design state must not be conflated**. Several good UX changes were implemented in a working tree that was never packaged before the chat was rewound. Those changes are documented here as reimplementation targets, not as assumptions about the current repository.

## Package map

```text
00_handoff/     context, chronology, user preferences, artifact lineage
01_product/     north-star product vision, decisions, MVP/roadmap, open questions
02_game_design/ gameplay-system summaries and current content direction
03_ux/          player-facing UI/UX, navigation, commands, screen blueprints
04_telegram/    Telegram 2026 capabilities, Rich Messages, ephemeral UX, tdx
05_technical/   architecture, persistence, scaling, diagnostics, refactor history
06_roadmap/     next implementation/playtest steps and long-term systems
07_reference/   glossary, decision log, reading map
99_archive/     original design documents and original diagnostic report
```

## Core one-sentence description

A **persistent Telegram-native sandbox MMO RPG** about exploration, discovery, specialization, collection, and interaction with a shared world, with fishing as the first deeply developed activity rather than the final scope of the game.

The project should always make the player feel that there is **another layer beyond what they currently understand**.
