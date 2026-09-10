# World, Geography, and Discovery

> **Status:** settled foundation + current Rustwater example

## Spatial truth

The world is not a bag of menu nodes. Locations have physical relationships, distance, context, and history.

For convenience, Telegram may offer a “travel to X” action, but that UI compression should not erase the underlying geography. Directional movement can be introduced later when it adds gameplay value.

A character normally has one primary physical location.

## Historical truth

The world should remember meaningful events:

- discoveries;
- world-first catches;
- major group events;
- location unlocks;
- future player-built transformations;
- notable global achievements.

The early implementation already demonstrated persistent global discoveries/world firsts.

## Discovery layers

### Explicit

The game tells the player something new happened and records it:

- new species;
- location;
- relic/key item;
- recipe;
- title/milestone;
- world first.

### Implicit

Players notice relationships without receiving a numeric explanation:

- a species appears in fog;
- a bait attracts an unusual family;
- an object/environment reacts differently at a certain time;
- a location clue suggests another destination.

Knowledge should convert into better decisions/odds.

## Completion UI as a design tool

Some collections may show known missing entries (`4 / ?`, visible blanks, etc.) to encourage directed search.

Other secrets may hide their total count entirely.

This distinction should be intentional rather than globally standardized.

## Rustwater vertical slice

The implemented/prototyped world thread included:

- Old Harbor;
- Broken Breakwater;
- Reed Pond;
- Old Lighthouse;
- Lighthouse Cove.

The Rusted Key loop demonstrated how a non-fish fishing result can lead into exploration and then unlock new fishing content.

Rustwater should be explained to players as a place, not assumed as lore they already understand.
