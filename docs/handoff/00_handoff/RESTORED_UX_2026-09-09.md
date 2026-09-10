# Restored UX Implementation — 2026-09-09

> **Status:** source reimplementation complete; Rust compilation/Clippy/rustfmt still requires a compiler-equipped environment.

This artifact reimplements the UX work described in `LOST_UX_IMPLEMENTATION.md` against the checkout that was supplied on 2026-09-09.

## `tdx`

Restored native helpers over the generated 2026 TDLib API:

- Rich Message `inputPageBlockButtonRow` construction;
- default/primary/success/danger callback buttons inside Rich Messages;
- primary/success/danger styles for conventional inline-keyboard callbacks;
- callback-triggered `sendEphemeralMessage` construction targeted to the user who pressed the button;
- ephemeral bot-command definitions and ephemeral replies to incoming group commands/messages;
- request/composition tests for these generated fields;
- `td/fetch` schema verification for the button-style, Rich Message button-row, and ephemeral-message constructors before installing a refreshed schema.

No generic UI framework or application policy was added to `tdx`.

## Group UX

Restored the documented group model:

- registered commands are `/fish` and `/help` only;
- `/game` routing is removed;
- `/fish` sends an activity-first shoal card with native Rich Message `Cast once` and `How it works` actions;
- `/help` is group-specific onboarding rather than the private help/game-status surface, and is registered/replied to ephemerally so it does not clutter the group; the same private guide is reachable from the shoal card so command-menu discovery is not required;
- a successful shared cast sends a per-user ephemeral Rich Message inside the group;
- the public card explicitly says the shared cast is free and does not touch rod/bait/coins;
- the ephemeral catch result offers read-only Journal and Records views without leaving the group;
- if ephemeral delivery fails, the callback falls back to a concise toast without rolling back the already committed catch;
- routine catches create no extra public messages;
- the shared card is edited only at powers-of-two participation milestones or a world first;
- the obsolete world-status/telemetry application query and view were removed, including its extra aggregate database reads.

## Private UX

Restored the main documented hierarchy:

- Home is player-first and uses in-message Cast/Explore/contextual actions;
- Bite, struggle, catch, relic, escape, and exploration surfaces use native Rich Message actions and clearer hierarchy;
- destructive actions use danger styling and affirmative actions use primary/success styling where appropriate;
- catch/discovery copy emphasizes the specimen, observation, XP, and discovery rather than implementation state;
- Help was rewritten for ordinary players and no longer contains a command catalog;
- private command registration/routing is reduced to `/start` and `/help`;
- inventory, tackle, board, crafting, titles, Locations, and Conditions now keep contextual/mutating actions inside the Rich Message and reserve conventional keyboards for navigation/secondary destinations;
- secondary panels use semantic parent/Home navigation where a stable semantic parent exists (for example Conditions → Locations, stall/crafting → Inventory, Records → Journal, Titles → Records);
- remaining developer-facing copy found in the touched progression/error surfaces was removed;
- a precision audit against the original user message is preserved in `ORIGINAL_UX_REQUEST_AND_AUDIT_2026-09-09.md`.

## Validation state

This environment has no `rustc`, `cargo`, or `rustfmt`, so the source has **not** been compiler-validated here. Static source checks and schema-name checks were performed, but they are not substitutes for Rust diagnostics.

Run `tools/collect-diagnostics.sh` on a compiler-equipped machine and return the resulting diagnostic bundle if anything fails. The first follow-up task should be to fix every compiler/rustfmt/Clippy issue from that run before adding gameplay systems.

## Handoff tooling added

- `tools/collect-diagnostics.sh [output]` runs the complete diagnostic matrix and continues after failures. It defaults Cargo to offline mode; set `HANDOFF_CARGO_OFFLINE=0` if dependency fetching is intentionally allowed. SQLite integrity checks run against a temporary DB/WAL/SHM copy so collecting diagnostics does not mutate the handed-off runtime database files.
- `python3 tools/handoff.py pack [output.zip]` creates a ZIP with an embedded SHA-256 manifest. `.tdx-session` and common `.env` files are excluded by default.
- `python3 tools/handoff.py verify archive.zip` verifies every archived file against that manifest.
- `python3 tools/handoff.py unpack archive.zip destination/` extracts with zip-slip protection.
- `tools/make-handoff.sh [output.zip]` runs diagnostics and then packages the project even when diagnostics report failures, so the failing report is still returned with the source.

Use `--include-session` or `--include-secrets` only when those sensitive runtime files are deliberately required for a handoff.
