# MVP-0 Specification

## 1. Purpose

MVP-0 is the first version intended for the developer and a small group of close friends.

Its purpose is not to prove monetization, retention, economy balance, social scalability, or long-term content depth.

Its purpose is to answer one question:

**Is the core Telegram-native exploration and fishing loop compelling enough to keep building?**

MVP-0 should demonstrate:

- a persistent character;
- a coherent small world;
- responsive Telegram-native fishing;
- meaningful variation between catches;
- discovery;
- lightweight progression;
- equipment choices;
- travel;
- one small mystery that connects fishing to the broader world.

---

## 2. Primary Player Loop

The core loop is:

**prepare → travel → fish → react → catch or fail → inspect → discover → improve → reach something new**

For MVP-0, most playtime should occur inside this loop.

The game should already hint that fishing is only one interaction with a much larger world.

---

## 3. Scope

### Included

MVP-0 contains:

- account bootstrap;
- one playable character per account;
- persistent XP and level;
- persistent inventory;
- currency;
- equipment;
- bait;
- fishing;
- catch generation;
- reaction timing;
- fish size and weight variation;
- basic weather;
- accelerated game time;
- several locations;
- timed travel;
- discovery tracking;
- one shop;
- simple selling;
- one small exploration/discovery chain;
- Rich Message presentation;
- persistent game state.

### Explicitly excluded

MVP-0 does not require:

- combat;
- health or death;
- classes;
- talent trees;
- character stats;
- professions;
- crafting;
- multiplayer trading;
- auction house;
- guilds;
- PvP;
- group-chat gameplay;
- global events;
- NPC reputation;
- procedural locations;
- player construction;
- ecosystem simulation;
- Mini App;
- website;
- Stars;
- crypto;
- NFTs;
- Telegram collectible integrations.

These are deferred, not rejected.

---

# 4. World

MVP-0 uses one small canonical region.

Example:

## Rustwater

A harbor settlement and its immediate surroundings.

### Locations

1. **Old Harbor**
   - starting location;
   - common fish;
   - forgiving conditions;
   - shop access.

2. **Broken Breakwater**
   - deeper water;
   - unlocked through exploration;
   - different species and loot table;
   - source of the Rusted Key.

3. **Reed Pond**
   - freshwater location;
   - different bait preferences;
   - demonstrates biome/location dependence.

4. **Old Lighthouse**
   - initially inaccessible;
   - connected to the Rusted Key;
   - serves as the end of the first mystery.

The world may internally represent adjacency and distance even if the interface offers direct destination selection.

---

# 5. Character

For MVP-0, a character contains:

- ID;
- name;
- current location;
- current travel state;
- XP;
- level;
- currency;
- inventory;
- equipped rod;
- equipped line or secondary fishing equipment if implemented;
- selected bait;
- discoveries;
- timestamps and other persistence metadata.

Only one character is exposed per account during MVP-0.

The data model should not assume that this will remain permanent.

---

# 6. XP and Levels

XP is awarded for gameplay actions such as:

- successful catches;
- first discoveries;
- unusually good catches;
- exploration;
- completing the lighthouse mystery.

Level currently has little or no direct mechanical effect.

It primarily indicates accumulated investment.

The XP curve should be simple and easy to rebalance.

Example:

- Level 1 → 2: 50 XP
- Level 2 → 3: 100 XP
- Level 3 → 4: 175 XP

Exact numbers are not important for MVP-0.

---

# 7. Fishing Session

A fishing session is tied to:

- character;
- location;
- selected equipment;
- selected bait;
- current environmental state.

The player initiates fishing through the current location interface.

## Basic flow

**Cast**
→ waiting state
→ bite/encounter
→ reaction opportunity
→ success/failure
→ result
→ return to ready state

The first catch for a new character is intentionally forgiving.

---

# 8. Fishing Encounter Types

MVP-0 should support at least three encounter complexities.

## Simple encounter

Most catches.

Example:

> The float disappears beneath the water.

Action:

**Reel**

Reaction timing influences outcome or quality.

---

## Intermediate encounter

Occasional catches.

Example:

> Something heavy takes the bait.

Player may choose:

- Reel;
- Wait;
- Give Line.

One or two state transitions occur before resolution.

---

## Exceptional encounter

Rare.

Example:

> Your rod bends violently toward the water.

Several decisions may occur.

The encounter may expose:

- tension;
- direction;
- equipment weakness;
- fish behavior.

MVP-0 only needs one or two exceptional encounter templates.

---

# 9. Reaction Timing

Reaction time matters, but uses broad timing bands.

Example:

- Excellent;
- Good;
- Late;
- Missed.

Timing may modify:

- catch success;
- escape probability;
- fish damage/stress if such a concept is added later;
- size/quality modifiers;
- encounter advantage.

Timing should not use extremely narrow latency-sensitive windows.

The game should record actual timing data during testing so later balance decisions can be based on real Telegram behavior.

---

# 10. Encounter Observations

Fishing narration should expose underlying encounter state.

Examples:

> The line moves slowly toward deeper water.

> The rod tip starts vibrating.

> Something small repeatedly pecks at the bait.

> A sudden pull nearly tears the rod from your hands.

These observations should not be purely decorative.

They should correlate with properties such as:

- size;
- aggression;
- movement pattern;
- depth;
- species family;
- rarity;
- environmental affinity.

Multiple text variants should exist for common observations to avoid repetitive output.

---

# 11. Fish

MVP-0 should have approximately 15–25 species.

The exact count matters less than diversity.

Species should demonstrate different mechanics.

Example categories:

### Common
- Harbor Perch
- Silver Minnow
- Mud Carp

### Conditional
- Night Perch
- Rain Eel

### Location-specific
- Reed Pike
- Breakwater Grouper

### Bait-sensitive
- Breadfin
- Worm Goby

### Rare
- Glass Minnow

### Currently difficult
- Old Harbor Sturgeon

At least one species should be encountered before the player is realistically able to land it.

---

# 12. Individual Catch Generation

A caught fish is an individual object.

At minimum, store:

- species;
- size;
- weight;
- catch location;
- catch time;
- catcher;
- bait used;
- relevant environmental conditions.

Optional early fields:

- quality;
- sex;
- procedural variation seed.

Individual catches should already have persistent identity if practical.

This leaves room for future:

- records;
- provenance;
- trading;
- trophies;
- mutations;
- unique specimens;
- externally represented assets.

---

# 13. Bait

MVP-0 should contain approximately 5–8 bait items.

Each bait has:

- base fishing power;
- one or more attraction modifiers;
- optional side effects.

Examples:

### Worm
Reliable general-purpose bait.

### Bread
Weak overall, but preferred by several small species.

### Glow Larva
Improves attraction in darkness but may discourage ordinary fish.

### Rotten Meat
Poor conventional bait, but attracts unusual scavengers.

The player-facing UI should remain simple.

Most hidden bait interactions do not need to be explicitly displayed.

---

# 14. Equipment

MVP-0 should contain approximately 4–6 meaningful fishing equipment options.

Avoid purely linear upgrades where possible.

Example:

### Old Wooden Rod
Balanced starter equipment.

### Reinforced Rod
Better control against heavy catches.

### Light Rod
Better reaction handling for small and fast species.

### Long-Cast Rod
Accesses deeper water or different catch pools.

Equipment should modify capabilities or encounter behavior, not only provide larger numbers.

---

# 15. Inventory

The inventory stores:

- fish;
- bait;
- equipment;
- miscellaneous items;
- key items.

For MVP-0, inventory management should remain simple.

No weight limit is required unless testing suggests it creates useful decisions.

Stackable items and unique items should be distinguished.

Fish may either remain individual objects or be stackable only where provenance does not matter.

Prefer individual representation if implementation cost is reasonable.

---

# 16. Selling and Currency

The player can sell ordinary catches.

The economy only needs one soft currency.

Prices should be influenced primarily by:

- species;
- size;
- rarity.

No dynamic market is required.

The player should earn enough currency during the first session to make one meaningful purchase.

---

# 17. Shop

One tackle shop is sufficient.

It should introduce meaningful choice rather than obvious linear progression.

Example early options:

- reinforced line;
- small hook;
- bread bait;
- better rod.

The player should usually be unable to buy everything immediately.

---

# 18. Time

The game has an accelerated fictional clock.

Game time affects:

- available species;
- catch probabilities;
- fish behavior;
- potentially size.

The game clock is independent from the real-world clock.

MVP-0 only needs:

- day;
- evening;
- night;
- morning.

Exact simulation complexity should remain low.

---

# 19. Weather

MVP-0 contains a small weather system.

Example states:

- clear;
- rain;
- heavy rain;
- fog.

Weather changes periodically.

Weather may affect:

- species eligibility;
- catch probability;
- encounter behavior.

The player should be able to observe current weather clearly.

At least one discoverable species interaction should depend on weather.

---

# 20. Travel

Characters physically occupy one location at a time.

Travel between locations takes time.

Early journeys should be short enough for testing.

Example:

- Old Harbor → Reed Pond: 45 seconds;
- Old Harbor → Broken Breakwater: 20 seconds.

While traveling, the character cannot begin location-dependent fishing.

The interface should clearly show:

- destination;
- remaining travel time;
- arrival result.

MVP-0 does not need manual directional navigation unless it is trivial to add.

The underlying world representation should leave room for it later.

---

# 21. Exploration

MVP-0 should contain a lightweight Explore action.

Exploration may:

- reveal locations;
- reveal environmental clues;
- trigger small discoveries;
- find the Old Lighthouse;
- unlock Broken Breakwater.

Exploration should not become a separate large system yet.

Its purpose is to prove that fishing exists inside a world.

---

# 22. Discovery System

Discovery is persistent.

MVP-0 should track at least:

- species discovered;
- locations discovered;
- special items discovered;
- one or more hidden interactions if appropriate.

A journal/bestiary interface should show progression.

Example:

## Old Harbor

Species discovered: **4 / ?**

- Harbor Perch
- Silver Minnow
- Mud Carp
- Glass Minnow
- ???
- ???

Unknown entries may be shown selectively.

Some categories should reveal missing entries.

Others may hide even the total count.

This distinction should itself become a future design tool.

---

# 23. Rusted Key Mystery

MVP-0 contains one small world-progression thread.

Suggested flow:

**Explore Old Harbor**
→ discover Broken Breakwater
→ fish there
→ catch Rusted Key
→ inspect key
→ notice lighthouse emblem
→ locate Old Lighthouse
→ use key
→ receive a meaningful discovery/reward

The reward should demonstrate that mysteries can lead to real progression.

Possible reward:

- unique rod component;
- new fishing location;
- rare bait recipe;
- permanent capability;
- access to a hidden dock.

For MVP-0, the simplest strong reward is:

**unlock a hidden fishing location behind the lighthouse.**

This immediately loops discovery back into fishing.

---

# 24. Rich Message UX

The main gameplay interface should use Telegram Rich Messages where appropriate.

Key interfaces:

- current location;
- fishing encounter;
- catch result;
- inventory;
- shop;
- discovery journal;
- travel state.

The game should minimize unnecessary new messages.

Prefer:

- updating existing state;
- structured presentation;
- embedded actions.

New persistent conversation messages should be reserved for meaningful events such as:

- first discovery;
- level-up;
- rare catch;
- major failure;
- location discovery.

---

# 25. Persistence

All meaningful state survives restarts.

At minimum persist:

- accounts;
- characters;
- inventories;
- items;
- catches;
- discoveries;
- currency;
- XP;
- level;
- current location;
- active travel;
- active fishing encounter if practical;
- world time;
- weather or world-state seed.

A server restart should not invalidate important player progress.

---

# 26. Content Representation

Content should be data-driven where this is clearly useful.

Species, bait, equipment, locations, and basic conditions should preferably be defined as content data rather than duplicated directly throughout gameplay code.

However, MVP-0 should not introduce an elaborate custom scripting language.

Use concrete implementations first.

Generalize only when repeated patterns become clear.

---

# 27. Telemetry

MVP-0 should record enough information to evaluate the game.

Important events include:

- session start;
- casts;
- encounters;
- reaction times;
- catch success;
- catch failure;
- species;
- bait;
- equipment;
- location;
- travel;
- purchases;
- sales;
- discoveries;
- level-ups.

The goal is not product analytics infrastructure.

Simple structured event logging is sufficient.

The most important early metric is qualitative:

**Do testers voluntarily continue fishing after they understand the mechanic?**

---

# 28. Developer Controls

Because MVP-0 is primarily a development environment, include simple administrative/debug controls.

Useful capabilities:

- give item;
- give currency;
- teleport;
- set weather;
- set game time;
- force species encounter;
- inspect character state;
- reset character;
- reload content if architecture permits.

These controls can save enormous iteration time.

They should remain isolated from ordinary player commands.

---

# 29. Completion Criteria

MVP-0 is complete when a new tester can:

1. create a character;
2. begin fishing without external instruction;
3. successfully catch several species;
4. experience both success and failure;
5. notice meaningful variation between encounters;
6. discover something new;
7. earn and spend currency;
8. choose an equipment upgrade;
9. travel or explore to another location;
10. encounter something they currently cannot handle;
11. catch the Rusted Key;
12. follow its clue;
13. unlock something through that discovery;
14. leave the game and return later with all meaningful state intact.

It should also be possible for the developer to rapidly alter content and balance during testing.

---

# 30. MVP-0 Success Test

MVP-0 is successful if several testers independently exhibit behaviors such as:

- fishing repeatedly without being instructed to;
- deliberately testing different bait;
- asking what an unknown species might be;
- returning because weather/time changed;
- discussing theories;
- pursuing a catch that escaped;
- caring about a particularly large specimen;
- noticing hidden mechanical relationships;
- wanting to know what lies beyond the currently available world.

The strongest signal is not:

**“This bot works.”**

It is:

**“I want to see what else is in here.”**