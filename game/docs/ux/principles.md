# UX and player-copy principles

## Player-first rule

Every normal game screen is for a player, including when the only current tester happens to be the developer.

Do not leak implementation language into ordinary UI:

- “MVP”, “prototype”, “this build”;
- schema/provenance guarantees stated as engineering notes;
- loaded-content/debug telemetry;
- release-note language;
- internal architecture terminology.

Debug/admin information belongs in explicit developer tooling, not the player path.

## Lead with the player's situation

A screen should first answer the question the player has in that moment: where am I, what happened, what can I do, what is unusual, what is at stake?

Do not lead a new group participant with world telemetry/counts before explaining the activity.

Mystery is good; unexplained interface consequences are not.

## Rich structure is gameplay UI

Use Telegram Rich Messages confidently when they improve hierarchy:

- headings;
- bold/marked emphasis;
- block quotations for observations/hints/dialogue;
- compact tables for genuinely tabular information;
- details/sections/dividers where useful;
- media blocks;
- in-message buttons;
- restrained visual accents/custom emoji when available.

Plain text remains correct when it is clearest. The mistake is treating all game state as an undifferentiated wall of text despite having richer primitives.

## Emoji language

Emoji should clarify role and hierarchy, not decorate every noun.

Current useful anchors include:

- 🎣 fishing/action;
- ⚡ urgency/bite;
- 📖 discovery/journal;
- 🪙 value/currency;
- ✨ XP/progress;
- 🏠 Home;
- 🌦 conditions;
- 🎉 notable success.

Custom Rustwater emoji may later become a stronger branded vocabulary; essential meaning still needs text/standard fallback.

## Actions vs navigation

The preferred hierarchy is:

- **Rich Message buttons:** primary/contextual actions that belong to the content itself;
- **conventional inline keyboard:** navigation, semantic parent/Home and secondary actions.

Do not duplicate the same action in both layers without a reason.

## Semantic parent + Home

Deeper screens should normally have an obvious conceptual parent and Home, for example:

- Tackle stall → Inventory + Home;
- Prepare bait → Inventory + Home;
- Records → Journal + Home;
- Titles → Records + Home;
- Conditions → Locations/Map + Home.

Do not maintain a generic browser-history stack until real playtesting shows semantic navigation is insufficient. History would introduce stale navigation/session state and restart semantics without current benefit.

## Commands are entry points, not a sitemap

Current private command surface is intentionally small: `/start`, `/help`.

Current group surface: `/fish`, `/help`.

Most gameplay should be discoverable through the interface itself. New commands should earn their place as Telegram-level shortcuts or genuinely chat-level actions.

## Keep social play social

When a player enters an activity from a group, prefer keeping useful personal results/interactions in that group via ephemeral state rather than reflexively sending them to DM.

DM remains excellent for long private sessions, but is not a substitute for social design.

## Message traffic is UX

Telegram rate limits and conversational clutter are visible product constraints.

A technically correct interaction that sends a public message for every routine participant is bad group UX. Coalesce shared state, edit sparingly, and use ephemeral/private results where appropriate.

A useful history rule is **panels for state; messages for events**. Mutable things such as encounter state, inventory, conditions or travel status usually belong in an edited panel. Notable discoveries, exceptional catches, major world events and other moments worth remembering may deserve persistent conversation history.

Callback acknowledgement is latency-sensitive UI plumbing. Ordinary callbacks should be acknowledged promptly instead of waiting behind cosmetic work. Group callbacks may deliberately own acknowledgement when an ephemeral response needs to use the callback context and fall back to a toast/alert.

## Copy voice

Player copy should be:

- concrete;
- concise enough for Telegram;
- atmospheric without purple-prose overload;
- confident about in-world terminology only after context exists;
- specific about consequences when a button might cost/consume/change something;
- willing to leave world mysteries unexplained when the uncertainty is intentional.

Prefer “A shoal is passing through; everyone gets one free cast” over abstract system terminology.

## Observation vs instruction

Fishing/encounter observations are gameplay information. Present the clue distinctly from the instruction/action.

Example shape:

> *The line snaps toward the rocks.*
>
> Choose how to respond.

The player should be able to learn the world from repeated observations rather than only obey generic prompts.

## Visual-first future

As image-backed screens expand, retain the same principles:

- image establishes place/character/emotion;
- Rich Message structure explains actionable state;
- controls remain obvious;
- alt/fallback copy still makes the state comprehensible;
- images do not become excuses for hidden consequences or unreadable UI.
