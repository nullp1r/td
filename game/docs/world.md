# World, geography and discovery

## World model

Rustwater begins as a small coastal region, but the finished game is meant to inhabit one coherent world capable of expanding far beyond it.

The world has two forms of truth:

### Spatial truth

Locations have physical relationships, distance, context and traversal constraints. The Telegram UI may provide a direct “Travel to X” action, but that is an interface compression, not proof that geography is meaningless.

A character normally occupies one primary physical location. Later capabilities can introduce routes, vehicles, hostile environments, underwater/vertical layers, continents, dimensions or remote worlds without replacing the concept of place.

### Historical truth

Meaningful things that happen can become part of persistent history:

- discoveries and world firsts;
- relationship/faction changes;
- event outcomes;
- location unlocks;
- player-built infrastructure;
- communal projects;
- rare permanent world transformations.

Telegram message history is never the authoritative history. Game state records what matters.

## Discovery model

Discovery has explicit and implicit layers.

### Explicit discovery

The game acknowledges and may record the first discovery of things such as:

- species;
- locations;
- relics/key objects;
- recipes;
- NPC/lore entries;
- titles/milestones;
- world firsts;
- future phenomena, factions, technologies or dimensions.

### Implicit discovery

The player notices a relationship without receiving an exposed formula:

- a species appears only in fog;
- a strange bait changes what approaches;
- an NPC knows more after a world event;
- a structure reacts at a specific time;
- a clue points toward another coastline;
- a behavior pattern hints at the correct encounter action.

Knowledge should improve decisions and odds. The UI should give enough evidence to learn without reducing the world to a probability spreadsheet.

## Completion UI and mystery

Collection/completion interfaces are design tools, not one universal policy.

Some collections may intentionally show blanks, unknown counts or partial silhouettes to create a directed hunt. Other secrets should not reveal that an entry exists until it is discovered.

Choose the degree of visibility based on what produces the better mystery.

## Player influence

The long-term world should let players affect their environment at several scales.

**Frequent temporary effects** might attract creatures, alter a local condition, activate a structure, summon an event or influence an ecosystem.

**Meaningful persistent construction** might improve a location, repair infrastructure, establish an outpost, open a route or contribute to a communal project.

**Rare permanent transformation** might found a settlement, permanently change a biome, open a dimension or complete a server-wide historical event.

The default rule is:

> frequent temporary influence → meaningful persistent construction → rare permanent transformation

## Current Rustwater vertical slice

Implemented geography:

- **Old Harbor** — weathered starter harbor, Mara's post and ordinary harbor fishing;
- **Broken Breakwater** — black/deep water and the first serious equipment/progression pressure;
- **Reed Pond** — freshwater ecology with different condition/bait relationships;
- **Old Lighthouse** — non-fishable exploration node;
- **Lighthouse Cove** — hidden fishable location below the cliffs.

The **Rusted Key** thread is the current miniature of the intended world loop: a non-fish fishing result carries a clue, exploration turns that clue into a place, and the new place changes the fishing/discovery space.

Rustwater should always be introduced to players as a place they are entering. Never assume the name itself explains the world.

## Time and weather

Current implementation uses a deterministic accelerated clock rather than persisted world ticks:

- one game day: 2 real hours;
- weather slot: 15 real minutes;
- day parts: Morning, Day, Evening, Night;
- weather: Clear, Rain, Fog.

The same timestamp always yields the same environment. This is currently simple by design and can evolve into richer seasons, moons, planetary/dimensional cycles, or world events later.

Real-world calendars can separately drive holidays, tournaments, anniversaries and scheduled social events.

## Beyond Rustwater

Long-term world directions include:

- connected regions/continents;
- underground, underwater and vertical spaces;
- settlements/civilizations/factions;
- dangerous environments requiring capabilities;
- transport and player-built infrastructure;
- supernatural/technological regions;
- dimensions and alien ecosystems;
- local, chat-local, regional and global event layers.

None of these is a commitment to build a generic world engine now. New world abstractions are earned by actual content.
