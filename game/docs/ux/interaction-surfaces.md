# Interaction surfaces

Rustwater uses multiple Telegram-native surfaces over one authoritative world. This document records the product role of each surface; [`../telegram/platform.md`](../telegram/platform.md) records verified platform facts and [`../telegram/rich-messages.md`](../telegram/rich-messages.md) records empirical Rich Message behavior.

## 1. Private DM — implemented core surface

Best for persistent personal play: fishing, exploration/travel, inventory/equipment/crafting, journal/records/titles, NPC scenes and longer private choices.

The current Home is **scene-first rather than sitemap-first**. Location/conditions/current thread and the immediate in-world actions lead; kit/progression details are secondary. The first-catch state deliberately exposes fewer navigation destinations so new players are not introduced to every subsystem at equal weight.

Only `/start` and `/help` are advertised as normal private commands.

### Mutable panel vs durable moment

Rustwater deliberately uses two message lifetimes:

- **panel/state** — routine navigation and ordinary catches can replace the current game message;
- **event/history** — new species, world records, relics and newly discovered locations remain as durable chat messages, followed by a fresh current-scene panel.

This preserves the low-noise single-interface idea without making every memorable event disappear on the next tap.

## 2. Rich Messages — implemented default UI

Rich Messages are the primary high-fidelity panel format. Current rules from cross-client testing are:

- use prose/scene composition for fiction, observation and encounters;
- use **compact tables with buttons inside cells** for dense transactional collections such as gear, shops, milestones, titles and crafting;
- use `● / ○` narrowly for genuine selected/unselected controls, not as a generic marker for locations/equipment/completion;
- use disabled buttons to communicate unavailable/current states;
- avoid interactive controls nested in list items because Android composition differed from Desktop;
- avoid UI that needs rapid repeated message edits; Telegram edit latency/rate limits make high-frequency steppers poor controls;
- never issue an edit when authoritative state did not change (`MESSAGE_NOT_MODIFIED`).

The complete compatibility matrix and caveats are in [`../telegram/rich-messages.md`](../telegram/rich-messages.md).

## 3. Group chat — implemented social surface

Current `/fish` uses:

- one public chat-local clue/event card;
- private ephemeral interpretation choices;
- persistent personal consequences;
- sparse public edits;
- persistent chat familiarity that gradually reveals hints, identity and eventually the mastered read.

This surface should remain mechanically distinct from DM fishing while sharing the same world/catch history.

Group `/help` is ephemeral; the event card itself also explains how to inspect the activity.

## 4. Classic inline mode — implemented initial sharing surface

A user can invoke an inline-enabled Rustwater bot from another chat and send a catch card. Empty query returns recent catches; text filters by species name; internal **Share this catch** buttons use an exact historical catch ID.

This is the first implemented version of the broader “game objects travel naturally through Telegram” loop. Strong future result types include location postcards, artwork/gallery pieces, profiles/titles, lore discoveries and event invitations.

Inline mode must be enabled operationally through BotFather `/setinline` for the deployed bot.

## 5. Ephemeral per-user group messages — implemented

Use for personal reads, choices, results, private Journal/Records views and other consequences that should happen inside a social conversation without spamming its public history.

Do not use ephemerality for facts meant to become durable world/conversation history.

## 6. Guest Mode — strong direction, prototype before committing mechanics

Guest Mode is distinct from classic inline mode: a supported chat can mention a guest-enabled bot even when the bot is not a member, and the bot can post a result using constrained trigger/reply context.

Promising uses include lightweight challenges, explaining a shared catch/art card, showing a profile/postcard, or summoning a small social encounter. Do not build mechanics that assume full arbitrary chat history.

## 7. Reply keyboard — possible, not default

The persistent custom keyboard below the composer is available but is not a good default sitemap for Rustwater. Reconsider it only for a concrete high-frequency interaction where an always-present action pad is objectively better.

## 8. Private-chat topics — exploratory

Topics may eventually help organize long-running NPC/story/market threads, but can also fragment an interface that benefits from directness. Adopt only after a real information-architecture problem appears.

## 9. Mini App — later dense/spatial surface

A Mini App becomes attractive when messages are objectively poor for the task: interactive maps, very large inventory/market layouts, complex builds, building planning or dense visual galleries. It remains a controller/view over the same server-side game state.

## Surface-selection rule

- personal sustained interaction → DM;
- shared moment/state → public group card;
- private consequence of shared moment → ephemeral group state;
- shareable immutable game object → inline mode;
- lightweight arbitrary-chat invocation → Guest Mode;
- dense/spatial interaction that outgrows messages → Mini App.

Do not force a mechanic into a surface merely because that Telegram feature is newer.
