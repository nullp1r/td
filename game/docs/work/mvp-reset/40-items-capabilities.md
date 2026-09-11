# Items, inventory and capabilities

## Locked: hybrid item identity

The new model must support both:

- **stackable commodities** where per-unit identity has no value;
- **unique item instances** where condition, origin, ownership or history matters.

Do not reduce inventory to `(character_id, item_type, quantity)`.

Significant instances may carry finder/origin/location/time, condition, inscription/modification, ownership/trade history, discovery context and task relevance. Provenance is a foundation, not an achievement layer added later.

Rarity, specimen/instance quality, significance, provenance and market value are separate concepts. A common type can be historically important; a rare type can be an unremarkable specimen.

## Locked: activity gear and task objects are ordinary items

Fishing rods, bait and similar activity-specific equipment are ordinary items in the same item model as other tools/materials. Do not preserve special top-level “fishing inventory” semantics.

Task/quest objects, when they exist, are also real items subject to ordinary ownership/location rules. They may be traded, exchanged, consumed or discarded when their physical/world semantics permit it. Do not solve task robustness primarily with magical “quest item cannot drop” protection.

## Locked: mundane junk exists and needs bulk ergonomics

The world should produce genuinely low-value/embarrassing/mundane finds as well as useful or mysterious ones. Not every boot, spoon or bottle should secretly hide a quest.

Inventory/economy UX must make junk manageable at scale: aggregate value and bulk sell/dispose behavior are expected where appropriate. Exact classification (intrinsic, suggested, player-marked), pricing and recovery rules remain open.

## Leaning: container inventory

Preferred model:

```text
character
  ├─ equipped positions
  └─ inventory containers
       ├─ backpack
       ├─ pouch
       └─ ...
```

Containers can provide capacity and later impose useful storage rules. MVP ergonomics should likely be slot-oriented because Telegram needs immediately legible inventory pressure. Item weight may exist as metadata for contextual systems without initially imposing a global carry-weight spreadsheet.

This remains a strong design candidate rather than an explicitly locked rule until real inventory UX/schema work starts.

## Leaning: equipment is item state

Equipped objects remain canonical owned item instances assigned to typed equipment positions rather than becoming a separate class of object. Keep the exposed slot taxonomy small until real gameplay requires more.

## Leaning: capabilities and composable requirements

World interactions should normally require capabilities, not exact item types. A crowbar may provide `pry`; a lantern may provide `illuminate`; later another tool, NPC or party member may satisfy the same capability differently.

Capability sources can include:

- carried/equipped items;
- learned skills;
- learned knowledge;
- character inherent state;
- party-member contribution;
- NPC assistance;
- environmental machinery/support;
- temporary effects.

Requirements should be composable (`all`/`any`/threshold/state predicates) so content can support alternate solutions without hardcoding “has_crowbar” into every interaction.

When the player clearly satisfies a world rule, the action should normally be deterministically available. RNG belongs in uncertain consequences, not arbitrary failure that contradicts understood capability rules.

Where useful for learnability, the UI should make it possible to understand *why* a new affordance exists (for example, a crowbar enabling `Pry`) without necessarily exposing internal numeric thresholds.

Procedural item variation should remain physically/world-logically interpretable: material, size, condition, maker, inscription, modification and origin are better default axes than arbitrary adjective/stat soup.

## Leaning: inventory presentation is query/view oriented

Telegram does not need to imitate a drag-and-drop grid. Useful views over the same inventory may include `Recent`, `All`, `Equipment`, `Tools`, `Consumables`, `Materials`, `Significant` and `Junk`, exposing only the categories that real content justifies.

`Recent` is a particularly strong default because it answers “where did the thing I just acquired go?” without forcing the player to understand the entire inventory taxonomy.

Item cards matter more than dense rows: selecting an object should expose its specific state, provenance/significance and contextual verbs.

Rarity words such as `Epic` should not become the sole significance language. Presentation may emphasize a common object strongly because its provenance/quest/history matters.

## Leaning: item actions use meaningful verbs

Prefer specific verbs such as `Light`, `Read`, `Eat`, `Equip`, `Open` and `Offer trade` over a universal `Use` button where the item's behavior is known. Main verbs should use real button affordances in Telegram; textual inline controls are better reserved for compact drill-down/navigation.

## Leaning: trade and discard obey canonical ownership

Gifts/trades transfer actual stacks/instances. Multi-resource trades commit atomically. Significant ownership changes can extend provenance.

Items should generally obey world rules and be discardable. Avoid solving task safety by making important objects magically undroppable everywhere. Recoverability should come from alternate solutions, reacquisition, trade/search or explicit world rules.

Ordinary dropped commodities may use temporary world loot bundles/decay so the persistent world does not become a landfill of trivial item instances.

## Leaning: currency is accounting, not backpack clutter

Preferred MVP direction is one integer-valued spendable balance formatted into denominations. Collectible/old physical coins may still exist as ordinary item instances, but routine currency transactions should be atomic ledger operations rather than stacks of coin objects.

## Open

- Exact container/slot rules and starting capacity.
- Equipment positions actually required by the first slice.
- Capability representation and whether numeric strength is hidden, qualitative, or partially exposed.
- Durability/condition mechanics and when they create value rather than maintenance friction.
- Item rarity vocabulary and player-facing visual grammar.
- Vendor pricing, market/auction shape and player trade UX.
- Junk classification (intrinsic, suggested, player-marked) and sell-all behavior.
