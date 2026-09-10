# Product vision

## North star

Rustwater is a **Telegram-native persistent sandbox MMO RPG** about exploration, discovery, specialization, progression, collection, relationships, and interaction with one coherent shared world.

Fishing is the first deeply developed activity and the mechanical spine of the current vertical slice. It is **not** the finished product definition and the player is not fundamentally “a fisherman.” The player is an adventurer whose first window into a much larger world happens to be a rod, a harbor, strange water, and things hidden beneath it.

The central emotional response is:

> **I want to see what else is in here.**

Curiosity should usually arrive before explanation. A system is doing its job when it makes players form theories, pursue clues, compare experiences, and wonder what lies beyond the part they already understand.

## The infinite-game thought experiment

The long-term ceiling is intentionally absurd. A veteran might eventually cross continents, build infrastructure, pilot specialized vehicles, enter hostile or supernatural environments, traverse dimensions, or travel to remote worlds and use a device that is only conceptually descended from a fishing rod to pull an entity from an alien ecosystem.

That is a direction test, not an implementation backlog. Early Rustwater should remain concrete, legible, and intimate. The architecture and domain language should avoid assumptions that make the larger game impossible, while the code should implement only systems with real current consumers.

**Design for the infinite game; implement the tiny game.**

## Design pillars

### Discovery is core progression

Discovery is gameplay, not an achievement layer stapled on top. Players should encounter unknown species, places, people, behaviors, environmental conditions, item interactions, recipes, lore, relics, events, and progression branches.

Some discoveries are explicit and recorded. Others remain implicit until the player learns a relationship by observation: a creature appears in fog, an NPC responds differently after an event, a bait behaves strangely in one biome, or a location changes at a particular time.

Knowledge should convert into better decisions and odds. A veteran can be stronger because they understand the world, not only because a number is larger.

### Capability progression over generic number inflation

Levels, XP, rarity, stats, equipment values, and money are useful, but memorable progression should change **what the character can do**.

Examples include reaching a new coast, surviving a hostile environment, detecting hidden creatures, operating transport, understanding unknown information, accessing a profession, manipulating local conditions, entering another dimension, or interacting with something that was previously impossible.

Level currently represents investment/seniority more than direct power. Do not casually turn it into a universal damage multiplier because other RPGs do.

### The world has spatial and historical truth

The game world is not a bag of screens. Locations have relationships, distance, context, and history. A character normally occupies one primary physical place. UI may compress travel for Telegram ergonomics, but convenience must not erase geography.

The world also remembers. Discoveries, world firsts, player-built changes, group events, major transformations, relationships, and other significant events can become persistent history.

### Depth without mandatory friction

The same world should support:

- roughly ten seconds: one meaningful action;
- minutes: a short encounter/check-in;
- an hour: a genuine progression step;
- an evening: exploration, social play or a focused goal;
- days/weeks/months: projects, collections, relationships, events and specialization;
- years: identity, world history, collections and social goals.

Short play is not a separate idle game. It is a small interaction with the same persistent character and world.

Complexity should come from meaningful systems and choices, not bad ergonomics or repetitive maintenance.

### Telegram is a first-class game client

Rustwater is not a web game hidden behind a bot command prompt. Telegram's native surfaces are part of the design vocabulary: DMs, group chats, Rich Messages, ephemeral results, inline mode, Guest Mode, custom emoji, native buttons, topics where appropriate, and eventually Mini Apps when a dense/spatial interface genuinely benefits from them.

A future website or Mini App should expose the same authoritative world, not fork it into a separate game.

### Social play belongs in the world

MMO does not mean “many players each DM the bot.” Shared environments, public events, chat-local phenomena, cooperation, competition, reputation, relationships, world history, and indirect interaction through the environment are first-class directions.

Public chat traffic must remain intentional. Personalized ephemeral interactions are especially valuable because they let shared events stay social without flooding the conversation.

### Characters and narrative are first-class systems

The finished game is expected to contain many memorable NPCs with deep lore, stateful dialogue, reputations, relationships, and potentially romance. NPCs should feel like people embedded in places and history, not quest vending machines.

Relationship progress can itself be a capability: trust can reveal knowledge, unlock access, change choices, open scenes, alter quests, or connect the player to factions and places.

See [`narrative/characters-and-relationships.md`](narrative/characters-and-relationships.md).

### Visual media is part of the game, not decoration

The intended finished product is highly visual. Many substantive screens may use curated generative imagery: location vistas, character portraits, expressions/outfits, story scenes, discoveries, relics, exceptional specimens, weather/time variants, event art, and collectible gallery pieces.

Unlocking images can itself become progression. Art can be a reward for discovery, relationship milestones, lore, rare catches, achievements, locations, titles, or events—and something worth sharing through Telegram-native surfaces.

See [`art/generative-media.md`](art/generative-media.md).

## Influences and taste

Useful reference points are:

- **Terraria** — discovery, overlapping systems, surprising item interactions, bizarre progression, systemic fishing.
- **World of Warcraft** — a persistent social world, long-lived identity, specialization, layered progression, real-world event cadence alongside in-world time.
- **Skyrim / Fallout: New Vegas / League of Legends talents** — traits, talents and meaningful character differentiation rather than only stat inflation.
- **Cookie Clicker** — only as inspiration for absurd long horizons and a sense that there is always another layer, not as a reason to replace active play with idle automation.
- **Minecraft** — useful as a sandbox comparison, but catastrophic accidental loss is explicitly not the desired frustration model.

These are directional references, not templates to copy.

## Failure philosophy

Failure should create consequences and stories without casually destroying months of investment because of latency, an unclear interface, or one mistake.

Prefer lost opportunity, consumed resources, wear, temporary debuffs, expedition failure, recovery costs, and positioning consequences. High-risk loss may exist later, but it should be explicit and preferably opt-in.

## Time and events

The simulation can run on an accelerated fictional clock independent of wall-clock speed. Real-world calendar time can still drive holidays, tournaments, anniversaries, seasons, or other events.

Major content should not become permanently inaccessible simply because a player tends to play at the same real-world hour.

## Economy and external ownership

First build a coherent internal RPG economy. Telegram Stars, collectible gifts, cryptocurrency, NFTs or other external ownership/monetization layers may be explored later, but the game must still make sense if all of them disappear.

External systems may represent ownership, transfer, monetization, or special interaction; they do not define what an in-world item fundamentally is. Avoid pay-to-win pressure on core discovery and progression.

## Core design test

When evaluating a mechanic, ask:

1. Does it create a meaningful decision, discovery, capability, relationship, or interaction?
2. Does it connect to the broader world rather than live as an isolated subsystem?
3. Can a new player understand the surface without understanding all hidden depth?
4. Can experienced players gain advantage through knowledge rather than only raw numbers?
5. Does it respect Telegram ergonomics and message-rate constraints?
6. Does it create interesting future interactions with other systems?
7. Does it create depth without unnecessary permanent complexity?
8. Is it useful now, or are we implementing it only because the distant future might need it?

The game should keep creating the feeling that **there is more here than I currently understand**.
