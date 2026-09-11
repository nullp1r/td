# NPCs, dialogue, knowledge and tasks

This area remains intentionally unresolved. The reset should not begin a permanent NPC/dialogue schema until this document's open questions have been worked through with concrete examples.

## Locked inhabited-world direction

The starting region needs multiple kinds of inhabitants, not one mascot/tutorial character. Expected roles include traders, service staff, task/quest sources, profession/skill experts, social/lore characters, memorable largely nonfunctional inhabitants and people whose relevance emerges later.

NPCs should feel like inhabitants with their own knowledge, opinions, concerns, relationships and plausible presence rather than menus wearing portraits.

A real dialogue system and localization system are required. The eventual in-game knowledge base should support rich-text articles through the same document/rich-text layer used elsewhere.

## Locked Mara direction

Mara Reed is a useful character-design success, not the game itself and not the only important NPC. “Harbor Warden” is rejected as her defining title/role.

She should be a competent local guide/expert/helper with her own life and reasons to be in particular places. She is not a permanent tutorial service and should not teleport into every newcomer scene merely because onboarding needs exposition.

Her exact occupation, title (if any), routine and long-term recurring role remain open.

Legacy durable docs currently call Mara the `Harbor Warden` and recurring visual face/mascot because that describes the fishing-era implementation/content. Those labels are **not reset-target commitments**. Preserve the successful character identity/design work, not her obsolete product centrality/title.

A possible companion/pet/local-guide connection through Mara was floated as an idea and explicitly **not locked**.

## Open conflict: guaranteed early Mara versus canonical presence

See [`20-identity-onboarding.md`](20-identity-onboarding.md). Earlier direction expected the player and Mara to meet near the beginning; later shared-world design rejects teleporting/phasing Mara for each newcomer.

This must be resolved deliberately while designing NPC presence/onboarding. Do not hide the contradiction by making Mara personal state.

## Leaning: NPC presence respects canonical world state

Specific NPCs should have canonical location/state where their physical presence matters. If Mara is elsewhere, a newcomer can learn about her, seek her out, meet someone else first, or progress through another route.

Separate service availability from individual NPC availability. A basic service can stay reliable through staffing/infrastructure while named characters remain free to move, sleep, travel or be unavailable.

## Required dialogue capabilities to design

The dialogue model likely needs to express or integrate:

- conditional opening lines;
- player response choices;
- character/world/relationship conditions;
- item/capability/knowledge checks;
- actions/effects;
- discovered topics;
- repeatable ambient lines versus one-time beats;
- NPC memory/relationship reactions;
- links into knowledge articles;
- rich Telegram presentation;
- localization without embedding final English in structural definitions.

These are requirements, not yet a commitment to a particular node-graph/DSL/schema.

## Knowledge direction

Knowledge is character state, not an inventory item. It can be learned through exploration, items, dialogue, other players or world events and may change what the character understands or can do.

The database may know facts the character does not. Presentation and affordances should respect character knowledge rather than leaking omniscient server state.

A knowledge fact may unlock:

- interpretation of an item/inscription;
- dialogue topic;
- location/POI recognition;
- route/opportunity;
- capability such as reading a script;
- more precise presentation of another player's/world trace.

## Open: tasks/quests

Do not assume the replacement system is a classic quest log with immutable scripted objectives. We need to determine how to represent:

- explicit requests from NPCs;
- mysteries and self-directed goals;
- world-state objectives that several players can affect;
- party/cooperative objectives;
- tasks whose item can be traded/lost;
- alternate solutions enabled by capabilities;
- failure/abandonment/recovery;
- progress that may exist before a formal task is discovered.

Task/quest objects are real items subject to the normal item/ownership rules described in [`40-items-capabilities.md`](40-items-capabilities.md), not a separate protected pseudo-inventory.

## Open questions for the next design session

- What is the smallest dialogue model that supports believable recurring NPCs without becoming a visual-scripting language?
- What does an NPC remember permanently versus derive from domain events/current relationship state?
- How do characters learn/share knowledge, and can another player teach a fact directly?
- How do rumors differ from verified knowledge?
- How do task objectives refer to world outcomes rather than hardcoded action sequences?
- Which NPC schedule/state needs canonical persistence versus deterministic derivation from time/content?
- What happens when multiple players interact with the same NPC around the same world event?
