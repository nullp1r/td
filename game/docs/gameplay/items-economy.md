# Items, bait, crafting and economy

## Item philosophy

Items should be systemic enough to participate in surprising interactions without forcing a universal item framework before real content needs it.

A fish/specimen is a meaningful individual object when identity matters. Its history should not disappear merely because it leaves usable inventory.

## Provenance

Current catches persist facts including:

- species;
- individual length/weight;
- seed + generator version;
- catcher/owner;
- caught time;
- location;
- bait;
- weather;
- fictional game time.

The history survives sale, contract turn-in and crafting consumption. Usable inventory state changes; the catch ledger remains historical truth.

This is foundational for records, trophies, trading, relationships to notable items, world history, and any future external representation.

## Bait as a use, not a rigid taxonomy

Long term, an object may be usable as bait because of what it **is** and how it was prepared, not because its definition belongs to a hardcoded “Bait” class.

Potential properties include:

- biological/material origin;
- freshness;
- smell;
- movement;
- size;
- elemental/supernatural properties;
- rarity;
- preparation;
- environmental compatibility.

The player-facing early game should expose only the amount of complexity that creates useful decisions.

### Current bait model

Current bait has a simple Fishing Power value, species-attraction modifiers, buyability/pack price, and stack selection.

Current bait set:

- Worm;
- Bread;
- Glow Larva;
- Rotten Meat;
- Silver Spoon;
- Fish Chunks (crafted-only);
- Crab Paste (crafted-only).

This already proves two distinct effects: wait-time power and encounter attraction.

Future bait can also affect aggression, size, specimen properties, junk/non-fish outcomes, environmental interactions, equipment risk or special encounter rules.

## Equipment

### Current rods

- Old Wooden Rod — base control 10, starter;
- Reinforced Rod — control 14, 45 coins;
- Light Rod — control 11, 35 coins;
- Breakwater Rod — control 18, 95 coins.

Rods are permanently owned once acquired. Condition wears on successful catches; harder catches wear more. Condition never destroys the rod and currently reduces effective control only modestly. Repair restores owned rods for a deliberately low cost and is blocked during an active encounter so capability cannot change mid-fight.

### Long-term equipment dimensions

Potential dimensions when content needs them:

- control/power;
- sensitivity/information quality;
- range/depth;
- durability;
- environmental compatibility;
- tags/capabilities;
- line/hook/reel/bobber/accessory/module categories;
- specialized devices that are only metaphorically rods.

Do not prebuild every equipment slot.

## Crafting/preparation

### Current concrete recipes

- **Cut bait** — consume the smallest stored catch; produce 4 Fish Chunks.
- **Crab paste** — consume a stored Mud Crab; produce 3 Crab Paste.

Recipe IDs are persisted in `craft_consumptions`, so renumbering existing IDs would reinterpret history.

The current implementation deliberately uses a tiny declarative recipe table rather than a generic crafting engine.

### Direction

Preparation, combinations, enchantment, processing, unusual ingredients and cross-system recipes can become deep systems. Add abstraction only once multiple concrete recipes/activities prove recurring semantics.

## Economy

The early economy should stay understandable:

- one soft currency (coins);
- catch value influenced by individual specimen size;
- selling catches;
- meaningful gear/bait purchases;
- deliberately cheap repair/recovery;
- no accidental irreversible traps;
- emergency worm forage when all bait is exhausted.

A dynamic market is not required to prove the core game.

Long term, player trading, markets, reputation pricing, provenance value, rare collectors and world infrastructure may deepen the economy.

## External value/ownership

Telegram Stars, collectible gifts, crypto/NFT/external asset systems are exploratory future layers.

Rules:

- the internal RPG/economy must stand on its own;
- external ownership must not define the base item ontology;
- monetization must not make core discovery/progression pay-to-win;
- durable provenance/versioning is valuable regardless of whether external representation ever happens.

## Failure/loss

Ordinary economy/equipment failure should prefer recoverable consequences: consumed bait, missed opportunity, wear, repair cost, expedition cost, etc.

Catastrophic loss can exist only as a clearly communicated high-risk system, not as a surprise consequence of latency or routine play.
