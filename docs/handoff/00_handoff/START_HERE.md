# Start Here

> **Status:** living handoff
> **Audience:** future human/AI developers

## Latest implementation note

The 2026-09-10 session is closed with the restored/audited Telegram UX, `tdx` tuple-formatting redesign, native relative timestamps, and the game maintainability pass integrated. **Read `SESSION_CLOSE_2026-09-10.md` immediately after this file.** It records the final real compiler diagnostics, the fixes applied afterward, and the exact next-session procedure. The original UX request remains preserved verbatim in `ORIGINAL_UX_REQUEST_AND_AUDIT_2026-09-09.md`.

## The most important context

This project began as “Terraria fishing, but absurdly deep, persistent, social, and native to Telegram.” It quickly became clear that the intended product is much broader: a sandbox MMO RPG in which fishing is the first fully developed profession/activity and a proving ground for the underlying world, discovery, item, persistence, and social systems.

The project is intentionally ambitious in **long-term depth**, but development must remain disciplined. The user repeatedly emphasized both of these truths:

1. The architecture and data model should not block the eventual huge game.
2. Scope creep must not prevent an actually playable MVP and frequent playtesting.

When those goals appear to conflict, prefer a **small concrete implementation with clean extensibility through demonstrated patterns**, not speculative frameworks.

## Before touching code

1. Read the repository root `AGENTS.md` completely.
2. Inspect the current checkout; do not assume the implementation status described in historical notes exactly matches the tree.
3. Run the diagnostics matrix in `05_technical/DIAGNOSTICS.md` if the task is technical.
4. If the task touches Telegram features added in 2026, refresh/verify the TDLib schema first. `tdx` is allowed and expected to evolve.
5. If the task touches player UI, read the entire `03_ux/` directory first.

## How to reason about uncertain design

The user does **not** want every uncertain mechanic formalized before the game is playable. Several questions — stats, full automation, exact specialization mechanisms, multiple characters, late-game travel rules — were deliberately left open because playtesting is expected to provide better answers.

Once enough broad context exists, the user is comfortable with the developer/AI choosing a sensible concrete answer for the prototype rather than restarting an endless questionnaire. Prefer reversible concrete implementations and explain the tradeoff.

## Non-negotiable product principles

- The player is an adventurer, not fundamentally “a fisherman.”
- Discovery is a primary form of progression.
- Capability progression matters more than pure number growth.
- The world has spatial and historical truth.
- No character should ultimately be able to master everything.
- Failure matters, but ordinary mistakes/lag should not destroy months of progress.
- Telegram is a first-class client, not a command-line transport for a game that conceptually lives elsewhere.
- UX/ergonomics matter as much as mechanical depth.
- Group chats should contain real social gameplay, not only links back to DMs.
- Telegram rate limits are a design constraint, especially in groups.
- `tdx` must never be treated as a feature ceiling; improve it when Telegram exposes something valuable.
- Player-facing copy must be written for players, not for the developer or an implementation report.

## What “done” means for an iteration

A ZIP should be handed off only after the relevant implementation is integrated coherently and the available diagnostics have been run. One of the painful lessons of this session was that packaging intermediate integration states creates more work than it saves.
