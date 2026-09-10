# Interaction surfaces

Rustwater should use multiple Telegram-native surfaces onto one authoritative world. This document records the product role of each surface; [`../telegram/platform.md`](../telegram/platform.md) records verified platform facts.

## 1. Private DM — implemented core surface

Best for:

- persistent personal play;
- fishing encounters;
- exploration/travel;
- inventory/equipment/crafting;
- journal/records/titles;
- NPC conversations and future relationship scenes;
- private choices;
- longer sessions.

Current navigation is Rich Message/inline-button driven with only `/start` and `/help` advertised as normal commands.

### Reply keyboard: possible, not default

Telegram also supports the persistent “custom keyboard” below the composer. Current Rustwater does **not** need it as a permanent sitemap; Rich Message + inline navigation is cleaner.

Reconsider it only for a concrete mode that benefits from an always-present action pad, e.g. movement, a repeated encounter vocabulary, accessibility, or another high-frequency interaction where reopening a panel is worse.

## 2. Group chat — implemented social surface

Best for:

- shared events/state;
- cooperation/competition;
- public discoveries and rare announcements;
- chat-local phenomena;
- a public invitation into the game.

Current `/fish` demonstrates the preferred structure:

- one public shared Rich Message;
- one safe primary action;
- per-user ephemeral outcome;
- sparse public edits;
- persistent consequences still belong to the player's character.

Group `/help` is ephemeral and the event card itself includes **How it works**, so slash-command discovery is not required.

## 3. Classic inline mode — committed direction

**Definitely part of the intended Rustwater surface.**

A user types the bot username plus a query in any chat and chooses a result to send. This is ideal for letting a player's game identity/content travel naturally into conversations where the bot may not otherwise be active.

Strong result candidates:

- catch/specimen card;
- personal/world record card;
- unlocked location postcard;
- unlocked artwork/gallery piece;
- player profile/title;
- discovery/lore card;
- event invitation/challenge;
- future market/trade listing if socially appropriate.

Empty-query results should still offer useful recent/favorite/shareable objects rather than requiring users to memorize a grammar.

Rustwater screens can include switch-to-inline buttons for “Share” actions.

## 4. Guest Mode — strong direction, prototype before committing mechanics

Telegram's 2026 **Guest Mode** is distinct from classic inline mode. A user can mention a guest-enabled bot in a supported chat even when the bot is not a member, and the bot can directly post a guest response.

This is the mention-based surface that can be confused with classic inline mode; Rustwater should treat them as distinct interaction models.

Promising Rustwater uses:

- mention Rustwater while replying to a catch/art card and ask for context;
- challenge another person from an arbitrary chat;
- summon a lightweight social encounter;
- show a player's unlocked postcard/record/profile;
- resolve a world-related query using the triggering/replied-to message as context.

Guest Mode receives constrained context by design, so mechanics must not assume access to entire chat history/member state.

Treat this as a social/discovery surface, not a second full UI until a prototype proves what feels good.

## 5. Ephemeral per-user group messages — implemented

Ephemeral group messages are particularly important to Rustwater because they create a private interaction layer **inside a social conversation**.

Use for:

- personal catch results;
- read-only journal/records from a group result;
- private choices/hints;
- personalized consequences of a public event;
- future inventory/relationship/event decisions that should not spam the group.

Do not use ephemerality for information that should become durable world/conversation history.

## 6. Rich Messages — implemented foundation

Rich Messages are the default high-fidelity Telegram game panel and can combine typography, tables, media, maps/collages/slideshows and in-message actions.

The new image-heavy vision makes this surface even more important: an NPC portrait/location image/CG can live in the same structured game object as dialogue/state/actions.

## 7. Private-chat topics — exploratory

Telegram bots can support forum-style topics in private chats. This might eventually help organize long-running domains such as:

- NPC/story threads;
- expeditions;
- trading/market activity;
- separate persistent activity workspaces.

It may also fragment a game interface that is currently pleasantly direct. Do not adopt it because the feature exists; test a real information-architecture problem first.

## 8. Mini App — later dense/spatial surface

A Mini App becomes attractive when Telegram messages are objectively poor for the task:

- interactive world map;
- large inventory/equipment layouts;
- markets/trading;
- crafting/build planning;
- galleries/collections with many images;
- complex character builds;
- visual infrastructure/world editing.

It should remain a view/controller over the same application/world state. Do not move core game rules into the client.

## 9. Web/other clients — future

A website or companion interface can eventually expose profiles, world maps, records, galleries, markets or community history. Same rule: one world, multiple surfaces.

## Surface-selection rule

Choose the surface that makes the interaction feel native and low-friction:

- personal sustained interaction → DM;
- shared moment → public group state;
- private consequence of shared moment → ephemeral group state;
- shareable game object → inline mode;
- lightweight invocation from an arbitrary chat → Guest Mode;
- dense/spatial interface → Mini App;
- durable public showcase/reference → web if/when useful.

Do not force a feature into a surface merely because that surface is newer.
