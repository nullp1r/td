# Game Design Foundation

## 1. Core Vision

The game is a persistent, Telegram-native sandbox MMO RPG centered on exploration, discovery, specialization, progression, and interaction with a shared world.

Fishing is the first deeply developed activity and the initial mechanical spine of the game, but the long-term vision is substantially broader. The player is not fundamentally “a fisherman”; the player is an adventurer in a vast world containing regions, biomes, locations, dimensions, civilizations, strange environments, and eventually increasingly absurd forms of exploration.

Fishing itself is intentionally broader than literal fishing. Early gameplay may involve conventional rods, bait, lakes, rivers, oceans, and familiar species. Much later, the same underlying interaction pattern may involve lava, dimensional anomalies, alien ecosystems, supernatural environments, spaceships, or devices capable of catching things that are not conventionally fish at all.

The world should feel as though it always contains another layer beyond what the player currently understands.

---

## 2. Primary Design Pillars

### Discovery Is Core Gameplay

Discovery is not merely an achievement layer attached to the game. It is one of the primary forms of progression.

Players should repeatedly encounter:

- unknown species;
- hidden locations;
- unusual environmental effects;
- undiscovered item interactions;
- secret requirements;
- rare conditions;
- unexplained behaviors;
- unknown crafting relationships;
- unusual events;
- new progression branches.

Some discoveries are explicit. The game may record the first time a species, item, location, recipe, or phenomenon is discovered.

Other discoveries are implicit. Players may observe that a species behaves differently during certain weather, that a particular bait attracts something unusual, or that a location changes under specific conditions without receiving an explicit numerical explanation.

The game should frequently create curiosity before providing explanation.

---

### Capability Progression Over Pure Numerical Progression

Numerical progression is useful, but the most memorable progression should unlock new capabilities.

Examples include:

- reaching a previously inaccessible location;
- fishing in a hostile environment;
- using a new category of equipment;
- detecting invisible or hidden creatures;
- entering another dimension;
- operating new forms of transport;
- understanding previously unreadable information;
- manipulating environmental conditions;
- catching things that were previously impossible to interact with.

The game may contain levels, XP, stats, equipment power, rarity tiers, and other numerical systems, but these should support rather than replace qualitative progression.

---

### The World Has Spatial and Historical Truth

The game should have a coherent underlying world rather than behaving as a collection of unrelated menus.

Locations have relationships, distance, geography, and context.

The interface may compress traversal for convenience, but the world itself should preserve meaningful spatial structure.

A character normally occupies one primary physical location at a time. Travel can take time, although faster transportation and other progression systems may reduce that time.

The game also has a persistent historical timeline.

Important discoveries, world events, first kills, first catches, player-built structures, major transformations, and other significant events may become permanent parts of world history.

The world should be capable of remembering what happened.

---

### Depth Without Mandatory Friction

The game should support meaningful play across a very wide range of session lengths.

A player should be able to perform one useful action in roughly ten seconds while also having goals that require hours, days, weeks, months, or years.

Short sessions should not be a separate game mode. They should be small interactions with the same persistent world.

Complexity should come from meaningful choices, systems, and interactions rather than unnecessary repetition or interface friction.

Depth should not require bad ergonomics.

---

### Telegram Is a First-Class Game Client

The game is not merely a bot that exposes commands.

Telegram DMs, group chats, Rich Messages, ephemeral interactions, Mini Apps, and eventually web interfaces should be treated as different surfaces onto the same game world.

Each surface should be used where it is strongest.

DMs are naturally suited to:

- personal encounters;
- fishing sessions;
- progression;
- inventory interaction;
- private decisions.

Group chats are naturally suited to:

- local events;
- shared encounters;
- competition;
- cooperation;
- social discoveries;
- spontaneous multiplayer interaction.

Mini Apps and web interfaces may later handle interactions that benefit from large spatial layouts, visual maps, complex inventory management, market interfaces, build planning, or other highly visual tasks.

Native Telegram interaction should remain important even after richer interfaces exist.

---

## 3. Interaction Philosophy

The game should be as dynamic as Telegram allows while treating message traffic and rate limits as real design constraints.

Complex gameplay should not require excessive message volume.

Where possible, one structured interaction should represent multiple changes in state.

State should primarily live in interactive interfaces.

Important events should enter conversation history.

For example:

- an active fishing encounter may update dynamically;
- a new species discovery may create a persistent visible event;
- an ordinary inventory adjustment does not need a new chat message;
- a major group catch may deserve a public announcement.

In group chats, shared public state and private player interaction should be separated where appropriate.

A group event may have one public representation while individual players receive personalized or ephemeral interactions.

---

## 4. Fishing Philosophy

Fishing is the first major activity and should be deep enough to support the initial game by itself.

Ordinary catches should usually be lightweight.

Interesting catches should introduce additional decisions.

Exceptional catches may become multi-step encounters.

The intended rhythm is approximately:

- most catches require one meaningful reaction;
- some catches require several decisions;
- rare or important encounters may temporarily become full encounters.

Reaction time should matter, but it should not be so strict that latency, notifications, accessibility, or client behavior dominate valuable progression.

Timing should usually influence quality, opportunity, or encounter state rather than acting as a binary millisecond-scale gate.

Fishing should expose imperfect information.

Players may infer what they are dealing with from:

- pull strength;
- movement;
- depth;
- environmental reactions;
- unusual sounds;
- line behavior;
- visual or textual clues;
- weather;
- time;
- location.

Experienced players should become better partly because they understand the world.

---

## 5. Items and Bait

Items should be systemic rather than excessively hardcoded.

The long-term item model should allow unexpected interactions.

An item does not necessarily need to belong to a rigid “bait” category to be usable as bait.

Potential bait behavior may depend on properties such as:

- biological origin;
- material;
- freshness;
- smell;
- movement;
- size;
- elemental properties;
- magical properties;
- rarity;
- preparation;
- environmental compatibility.

Players should not initially be overwhelmed with all underlying properties.

Simple player-facing information can sit on top of deeper hidden or discoverable systems.

Eventually, bait combinations, preparation, crafting, enchantment, unusual side effects, and strange item interactions may become substantial systems.

---

## 6. Randomness and Procedural Content

Randomness should create variation, surprise, collecting depth, and replayability.

It should not become a substitute for design.

Different systems may use different forms of randomness.

Some outcomes may be:

- purely rare;
- condition-based;
- progressively made more likely through knowledge;
- guaranteed after solving a discovery puzzle;
- procedurally unique;
- influenced by hidden environmental variables.

Mandatory progression should generally avoid extreme uncontrolled RNG.

Extraordinary optional specimens, cosmetic variants, unusual combinations, record-setting catches, and collector-oriented outcomes may be extremely rare.

The player should often be able to convert knowledge into better odds.

Procedural generation may eventually apply to:

- fish size;
- weight;
- morphology;
- coloration;
- mutations;
- traits;
- provenance;
- unusual effects;
- generated encounters;
- generated locations;
- artifacts.

Procedural systems should create gameplay consequences rather than only producing cosmetic rarity scores.

---

## 7. Character Progression

Characters gain XP and levels.

Initially, level is primarily a persistent measure of investment and experience rather than a direct source of gameplay power.

Level may later support:

- prestige;
- social signaling;
- event eligibility;
- recognition;
- giveaways;
- broad experience-based systems.

Core power should come from a broader combination of:

- capabilities;
- equipment;
- talents;
- traits;
- knowledge;
- professions;
- reputation;
- discoveries;
- infrastructure;
- wealth;
- social relationships.

No character should be able to master everything.

Specialization should emerge from meaningful opportunity costs, investment, equipment choices, talents, professions, reputation, location, knowledge, and economic decisions.

Rigid permanent classes are not required.

---

## 8. Failure and Loss

Failure should matter without routinely destroying months of progress.

The game should generally prefer:

- lost opportunity;
- consumed resources;
- damaged equipment;
- durability loss;
- temporary debuffs;
- expedition failure;
- recovery costs;
- positioning consequences.

Ordinary mistakes, client lag, or network problems should not regularly destroy irreplaceable or highly valuable progression items.

High-risk systems may exist later, but they should be clearly intentional and preferably opt-in.

Failure should create stories and consequences, not make players regret playing.

---

## 9. World Interaction

Players should be able to influence the environment.

This influence can exist at several levels.

Frequent temporary effects may include:

- attracting creatures;
- changing local conditions;
- activating structures;
- summoning events;
- influencing ecosystems.

Persistent effects may include:

- improving locations;
- building infrastructure;
- establishing outposts;
- repairing structures;
- opening transport routes;
- contributing to communal projects.

Permanent world transformations should be rarer.

Examples may include:

- opening a new dimension;
- creating a permanent settlement;
- permanently changing a biome;
- completing a server-wide historical event.

The default principle is:

**frequent temporary influence, meaningful persistent construction, rare permanent transformation.**

---

## 10. Time and Events

Real-world calendar time may matter for:

- holidays;
- anniversaries;
- tournaments;
- scheduled events;
- limited-time content.

The game simulation should not be forced to use real-world clock speed.

Game day/night cycles, weather, moons, seasons, dimensions, and planetary cycles may operate independently.

Players should not be permanently locked out of major content because they habitually play at the same real-world time.

---

## 11. Economy

The game should first have a coherent native game economy.

External monetary systems should be layered on later.

Possible future integrations include:

- Telegram Stars;
- cryptocurrency;
- Telegram collectible gifts;
- NFTs;
- externally owned digital assets.

The game must remain internally coherent even if all external integrations are removed.

The game determines what an item is.

External systems may later represent ownership, transfer, monetization, or special interaction with some game objects.

Real-value systems should not define the basic game before the underlying RPG is compelling.

---

## 12. Development Philosophy

The project should be designed for the long-term game while implemented according to immediate needs.

The guiding rule is:

**Design for the infinite game; implement the tiny game.**

Long-term vision should influence:

- architectural boundaries;
- domain concepts;
- ownership models;
- extensibility;
- data representation.

It should not justify implementing speculative systems before they are required.

The initial codebase should avoid assumptions such as:

- every activity is fishing;
- every account has exactly one character forever;
- every item has a fixed use;
- every interaction happens through one Telegram message format;
- every location is only a menu choice.

At the same time, the MVP should not attempt to implement a generalized MMO engine.

Abstractions should be earned by real use cases.

---

## 13. Initial Vertical Slice

The first playable version should prove the foundational interaction before expanding scope.

A likely first slice contains:

- one character;
- one small region;
- several fishing locations;
- a small species pool;
- junk and treasure catches;
- several baits;
- several equipment choices;
- inventory;
- currency;
- XP and levels;
- game time;
- simple weather;
- basic travel;
- discovery tracking;
- one shop;
- one small hidden progression thread.

A representative first-session progression is:

**start → first catch → first species discovery → behavioral variation → earn currency → choose equipment → encounter something too powerful → explore nearby geography → discover a new fishing spot → catch an unusual non-fish item → gain a clue leading deeper into the world**

The first slice does not require combat, guilds, player trading, NFT integration, global events, elaborate crafting, or procedural world generation.

Those systems remain part of the long-term vision without being MVP requirements.

---

## 14. Core Design Test

When evaluating a new mechanic, ask:

1. Does it create a meaningful decision, discovery, capability, or interaction?
2. Does it connect to the broader world rather than existing as an isolated subsystem?
3. Can a new player understand its surface without understanding its full depth?
4. Can experienced players gain advantage through knowledge rather than only raw numbers?
5. Does it preserve reasonable Telegram ergonomics?
6. Does it produce interesting future interactions with other systems?
7. Does it create depth without unnecessary permanent complexity?
8. Is it needed now, or are we implementing it only because the distant future might use it?

The game should continually make the player feel:

**“There is more here than I currently understand.”**