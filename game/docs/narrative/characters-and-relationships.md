# Characters, dialogue, reputation and relationships

## Direction

The finished game is expected to contain **many memorable NPC characters with deep lore, dialogue, reputations and relationships**, potentially including romance.

This is a major part of the game vision, not peripheral quest flavor.

The current implementation has one concrete NPC—Mara—and deliberately does **not** contain a generic dialogue/relationship engine yet. Build concrete characters and scenes first; abstractions should emerge from repeated real requirements.

## NPC design principles

NPCs should be people embedded in the world rather than quest vending machines.

A strong recurring character has:

- a place and role in the world;
- personal goals, concerns and knowledge;
- a recognizable voice;
- relationships to other people/places/factions;
- state that can change;
- memories or reactions to meaningful player/world events;
- information the player can earn rather than receive from a static encyclopedia;
- visual identity consistent across portraits/scenes/outfits;
- reasons to exist even when they have no reward to hand out.

Dialogue should be allowed to react to discoveries, world history, location, reputation, relationships, prior choices, time/events and capabilities.

## Dialogue

Early implementation should stay concrete and authored. A generic branching-dialogue DSL is not justified merely because many NPCs are planned.

When repeated content proves the need, structured dialogue state may eventually capture concepts such as:

- prerequisites/conditions;
- remembered facts/choices;
- relationship/reputation thresholds;
- world event flags;
- one-time vs repeatable lines;
- choice consequences;
- scene/art references;
- alternate presentation/expressions.

The player should not see every underlying condition. Dialogue can itself be discovery.

## Reputation

Reputation is a likely progression layer, but the exact model is open.

Potential scopes:

- individual NPC trust;
- factions/organizations;
- towns/communities/regions;
- professions or research groups;
- hidden reputations inferred from behavior.

Reputation should have consequences beyond a meter. It may change information, prices, access, assistance, quests, social standing, locations, equipment, scenes or world responses.

## Relationships

Relationships may be one of the ways characters specialize and become embedded in the world.

Meaningful relationship progress can unlock:

- personal history/lore;
- private dialogue;
- unique knowledge/clues;
- access to restricted people/places;
- help in encounters/projects;
- items or preparation techniques;
- choices and consequences;
- character scenes/CGs/art gallery unlocks;
- alternate outfits/portraits;
- future capabilities tied to trust/social networks.

Avoid reducing every relationship to one universally increasing “friendship number.” Different characters may care about different behaviors and remember different events.

## Romance

Romance is explicitly within the possible finished-game scope for suitable NPCs.

Current rules:

- romanceable characters, if any, are clearly adult;
- romance is character/narrative content, not a mandatory progression tax;
- exact romanceable cast, exclusivity, consequences and relationship structure are **open**;
- **Mara Reed is not automatically romanceable because she is the mascot**;
- do not sexualize core character identity merely to advertise romance.

If romance is added, it should respect the same standard as other deep relationships: character agency, coherent progression, stateful consequences, memorable scenes and integration with world/lore.

## Visual storytelling

Characters are expected to have a substantial visual layer:

- canonical reference/model art;
- portraits and expression variants;
- outfit/weather/event variants;
- scene illustrations/CGs;
- relationship milestone art;
- collectible/unlockable images.

Visual state should reflect narrative state when feasible, while canonical design anchors prevent generative drift. See [`../art/generative-media.md`](../art/generative-media.md).

## Current NPC: Mara Reed

The in-game `Mara Reed · Harbor Warden` is the canonical **Mara Reed**. She currently reacts to the Rusted Key/lighthouse progression and points the player toward Harbor Board content.

She is also the current face/mascot and recurring visual viewpoint character for Rustwater. See [`mara-reed.md`](mara-reed.md) for the full specification.

For future recurring characters, use [`character-spec-template.md`](character-spec-template.md) as a compact consistency checklist. It is a documentation template, not a proposed runtime dialogue schema.
