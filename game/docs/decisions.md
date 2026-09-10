# Durable decisions

This registry records decisions that matter beyond one implementation session. It is not chronological. Change an entry when the decision itself changes.

`Settled` means use it as the default. `Provisional` means the current direction is useful but intentionally easy to revisit. `Open` questions live in [`open-questions.md`](open-questions.md), not here.

## Product and world

| Status | Decision |
|---|---|
| Settled | The finished game is a broad sandbox MMO RPG; fishing is the first deep activity, not the player's identity or the ceiling of the world. |
| Settled | Discovery is core gameplay/progression, both as explicit recorded discoveries and implicit player-learned relationships. |
| Settled | Some collections may reveal blanks/counts as clues while other secrets hide their total entirely. This is contextual, not globally standardized. |
| Settled | The world has coherent spatial truth. A character normally occupies one primary physical location. |
| Settled | Travel may take time; later transport/capabilities can transform travel without deleting geography. |
| Provisional | Long-term navigation may mix directional traversal with destination/route-based travel depending on surface and context. |
| Settled | The world has historical truth: meaningful discoveries, firsts, transformations and social events can become persistent history. |
| Settled | The game clock can be accelerated and independent of real time; real-calendar events may coexist with it. |
| Settled | Player influence should tend toward frequent temporary effects, meaningful persistent construction, and rare permanent world transformation. |

## Character and progression

| Status | Decision |
|---|---|
| Settled | XP and level exist, but level should not automatically be a generic direct-power multiplier. |
| Settled | Qualitative capability progression is more important than pure numerical growth. |
| Settled | Long term, no one character should be able to master everything; specialization/opportunity cost matters. |
| Settled | Titles are useful non-power identity/status rewards. |
| Settled | Reputation, knowledge, infrastructure, wealth and social relationships are valid forms of progression/capability. |
| Settled | Failure should matter without casually deleting months of progression because of latency, mistakes or unclear UX. |

## Fishing, items and randomness

| Status | Decision |
|---|---|
| Settled | Reaction timing matters but uses forgiving bands; transport/client latency must not dominate valuable outcomes. |
| Settled | The reaction deadline begins only after the actionable Telegram state has been successfully presented. |
| Settled | Observations should vary and correlate with mechanics; they are information, not repeated flavor filler. |
| Settled | Long-term “bait” is a use/context rather than a permanently rigid taxonomy; unusual items may become bait. |
| Settled | Bait can combine attraction/power with positive or negative side effects; early UI may hide deeper properties. |
| Provisional | Bait preparation/combinations/crafting should deepen as content earns it, without overwhelming the early interface. |
| Settled | Individual procedural specimens are desirable and should have stable identity/provenance. |
| Settled | Procedural systems should create gameplay/collection consequences rather than only cosmetic rarity numbers. |
| Settled | Catch provenance/history survives sale, contract turn-in or crafting consumption. |
| Settled | Mandatory progression should not rely on extreme uncontrolled RNG; optional collector outcomes may be extremely rare. |

## Narrative and art

| Status | Decision |
|---|---|
| Settled | The finished game should contain many authored NPCs with lore, stateful dialogue and relationships rather than treating NPCs as quest vending machines. |
| Direction | Reputation and relationship progression should unlock meaningful world information/access/scenes/opportunities, not exist only as meters. |
| Direction | Romance is in scope for suitable clearly-adult characters; exact romanceable cast/rules/consequences remain open. |
| Settled | Mara Reed is the full identity of the existing Mara/Harbor Warden character and the current recurring visual face/mascot of Rustwater. She is not the player's canonical avatar. |
| Direction | Curated generative imagery is expected to become a major presentation/content layer; many substantial game screens will likely be image-backed where art adds meaningful place/character/atmosphere. |
| Direction | Unlockable images/gallery pieces can become progression rewards and collectible content. |
| Settled | AI generation is a production technique, not an aesthetic excuse: output must be curated and consistent with canonical character/world references. |

## Social and Telegram UX

| Status | Decision |
|---|---|
| Settled | Telegram is a first-class client; choose native surfaces by what they are good at rather than forcing everything through slash commands or a Mini App. |
| Settled | Group chats should contain real multiplayer/social gameplay; “go to the DM” is not sufficient MMO design. |
| Settled | Public shared state + private per-user ephemeral interaction is a powerful default pattern for group gameplay. |
| Settled | Telegram message/rate limits are product constraints. Routine participation must not flood groups. |
| Settled | Commands are entry points, not the private-game sitemap. |
| Settled | Player-facing UI should use Rich Message hierarchy, tasteful emoji and semantic controls rather than developer telemetry/walls of text. |
| Settled | Primary contextual actions belong inside Rich Messages when appropriate; conventional inline reply markup is mainly navigation/secondary action. |
| Settled | Semantic parent + Home is preferred over a generic persistent browser-history stack until real UX evidence requires history. |
| Settled | Classic inline mode is part of the intended social/share surface. |
| Direction | Telegram Guest Mode (mentioning the bot in chats where it is not a member) is a promising Rustwater surface and should be prototyped before committing detailed mechanics. |
| Provisional | Persistent reply keyboards in DMs are not the default navigation model; use only if a specific interaction benefits from an always-present action pad. |

## Architecture and engineering

| Status | Decision |
|---|---|
| Settled | SQLite is authoritative mutable game state in the current architecture. |
| Settled | Telegram/TDLib is input/presentation, not authoritative game truth. |
| Settled | Meaningful mutations are transactionally atomic. |
| Settled | Important delayed state lives durably in SQLite; in-memory timer/sleep state is never authoritative. |
| Settled | Callback IDs/steps are validated against authoritative state; stale/double actions must be harmless. |
| Direction | As multiple long-lived physical activities appear, mutually incompatible character activity should become an explicit coherent state model rather than a growing set of unrelated `is_*` booleans; do not generalize before a second real activity requires it. |
| Settled | `tdx` is part of the product stack and should evolve when Telegram exposes a useful feature. |
| Settled | Avoid generic repositories, services, DSLs, state-machine frameworks or other abstraction layers until repeated concrete consumers justify them. |
| Settled | Use modern/bleeding-edge Rust when it provides concrete clarity, correctness or efficiency; compatibility with stale compilers is not a goal. |
| Settled | Lean dependencies are acceptable when useful; notably heavy dependencies require explicit justification/approval. |
| Settled | Current source modules should convert persisted raw IDs into typed domain IDs near the query boundary. |
| Settled | Persisted numeric IDs/tags and callback wire values must remain explicit/stable where renumbering would reinterpret existing state/messages. |

## Development process

| Status | Decision |
|---|---|
| Settled | Documentation is part of the definition of done for behavior/design/invariant changes. Edit current truth; do not accumulate session-history files. |
| Settled | Project ZIPs/archives are transient transport between environments, not releases, milestones, or durable identifiers. |
| Settled | Diagnostics output is transient. The diagnostics script diagnoses the project; it must not encode archive-transfer policy. |
