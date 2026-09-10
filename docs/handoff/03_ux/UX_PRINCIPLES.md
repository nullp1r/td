# UX Principles

> **Status:** settled after playtesting feedback

This directory represents one of the most important corrections made during the session.

## 1. Every normal screen is for a player

The developer is also a player. Normal UI must not contain implementation commentary simply because the current audience is small.

Avoid player-facing phrases such as:

- “this build”;
- “MVP”;
- “generic crafting engine”;
- internal provenance/schema explanations;
- implementation guarantees;
- debug/world telemetry unless the player has a reason to care.

Developer/admin/debug information belongs in isolated developer tools/logs.

## 2. Lead with the player's current situation

Bad:

> Rustwater · world pulse
> Characters: 12
> Species found: 18/26

Good:

> 🎣 **Fishing in this chat**
> A shoal of **Echo Herring** is passing through. Everyone gets one cast before it moves on.

Explain the activity before showing state.

## 3. Rich structure is part of the game

Use Telegram Rich Messages confidently:

- headings;
- bold/marked emphasis;
- block quotes for observations/hints;
- compact tables for genuinely tabular stats;
- dividers/details where useful;
- in-message button rows;
- tasteful emoji/icon anchors.

Plain text is fine when plain text is the clearest thing; the mistake was defaulting almost everything to undifferentiated prose despite having a rich UI toolkit.

## 4. Emojis should clarify, not decorate every noun

The user wants visible accents, not emoji soup.

Good uses:

- 🎣 primary fishing action;
- ⚡ immediate bite/urgency;
- 📖 discovery/journal;
- 🪙 currency/value;
- ✨ XP/progress;
- 🏠 Home;
- ← semantic Back;
- 🌦 conditions;
- 🎉 a meaningful success.

## 5. Actions and navigation have different jobs

Target hierarchy:

- **In-message Rich Message buttons:** primary/contextual actions.
- **Conventional inline keyboard below:** navigation and secondary actions.

Do not duplicate the same action in both places without a concrete reason.

## 6. Back + Home

Most screens should have a meaningful semantic parent plus Home.

Do not implement a generic stateful browser-history stack yet. It creates session/navigation state without demonstrated need.

Examples:

- Tackle stall → `← Inventory` + `🏠 Home`
- Prepare bait → `← Inventory` + `🏠 Home`
- Records → `← Journal` + `🏠 Home`
- Titles → `← Records` + `🏠 Home`
- Conditions → `← Map/Locations` + `🏠 Home`

## 7. Commands are not the menu

Private commands should be minimal entry points. Target: `/start`, `/help`.

Group commands should serve chat-level purposes. Target: `/fish`, `/help`, with future additions like ratings only if they truly belong as commands.

## 8. Social state should remain social

Use public group messages for shared state and noteworthy events, ephemeral messages for private results, and DMs only when that surface is genuinely superior.

## 9. Rate limits remain UX constraints

A beautiful interaction that floods a group is a bad Telegram UX. Coalesce public updates and use per-user ephemeral views rather than many public messages.
