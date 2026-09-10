# Product and Game Design Decisions

> **Status:** decision registry
>
> `Settled` means use as the default. `Provisional` means implemented/targeted but easy to revisit. `Open` means deliberately undecided.

| ID | Status | Decision |
|---|---|---|
| D001 | Settled | The game is a broad sandbox MMO RPG; fishing is the first deep activity, not the identity of the player. |
| D002 | Settled | Discovery is core gameplay/progression, with both explicit tracked discoveries and implicit player-learned relationships. |
| D003 | Settled | Some completion views may reveal missing entries/counts as clues; other unknown content may remain entirely hidden. |
| D004 | Settled | The world has coherent spatial truth rather than being only unrelated menus. |
| D005 | Settled | A character normally occupies one primary physical location at a time. |
| D006 | Provisional | Navigation may eventually support both directional traversal and destination-based automatic travel. |
| D007 | Settled | Travel may take time; later progression can reduce/transform it rather than deleting geography. |
| D008 | Settled | Game clock is accelerated and independent of real-world time; real-world events may still exist on real-world schedules. |
| D009 | Settled | XP and levels exist, but initially level is investment/seniority/status rather than direct power. |
| D010 | Settled | Capability progression should be more important than pure numerical growth. |
| D011 | Settled | No character should eventually be able to master everything; specialization/opportunity cost is important. |
| D012 | Open | Exact future stats/traits/talent/profession model. User is skeptical of generic stat inflation and likes meaningful traits/talents. |
| D013 | Open | Multiple characters per account can be deferred until demonstrated need. |
| D014 | Settled | Failure should matter without casually destroying months of progress because of mistakes, lag, or bad UX. |
| D015 | Settled | Reaction timing matters, but uses broad forgiving windows; latency must not dominate valuable progression. |
| D016 | Settled | Observational text should vary; avoid repeating one boring line for every encounter. |
| D017 | Settled | Long-term item model should permit unusual things to become bait; rigid bait-only taxonomy is not the end state. |
| D018 | Settled | Bait can combine power/attraction with side effects; unusual positive/negative combinations are desirable. |
| D019 | Provisional | Bait combination/preparation/crafting should deepen over time, but early UI must not overwhelm players. |
| D020 | Settled | Procedural individual specimens are desirable; procedural systems should have gameplay meaning, not only rarity scores. |
| D021 | Settled | Individual catches preserve provenance/history even after sale/turn-in/crafting. |
| D022 | Settled | Player interaction through the environment is a promising MMO direction, not only direct PvP/chat interaction. |
| D023 | Settled | Telegram rate limits are a first-class design constraint, especially in groups. |
| D024 | Settled | Telegram is a first-class client; use each native surface where it is strongest. |
| D025 | Settled | Group chats should contain real multiplayer/social gameplay. DMs alone are insufficient and can feel boring. |
| D026 | Settled | Player-facing UI should use Rich Messages, hierarchy, accents, and tasteful emojis rather than walls of plain text. |
| D027 | Settled | Primary actions belong inside the content when Rich Message buttons fit; conventional reply markup should focus on navigation/secondary actions. |
| D028 | Settled | Commands are entry points, not the private-game navigation system. |
| D029 | Current target | Private advertised commands should be approximately `/start` and `/help`; normal game surfaces are reached through UI. |
| D030 | Current target | Group advertised commands should be approximately `/fish` and `/help`; later commands such as ratings may be added only if they have a clear chat-level purpose. |
| D031 | Settled | Semantic Back + Home is preferred over Home-only navigation; do not add a stateful browser-history stack until there is demonstrated need. |
| D032 | Settled | Per-user ephemeral responses inside groups should be exploited for private results/contextual interactions. |
| D033 | Settled | `tdx` can and should change when that unlocks a better Telegram-native design. |
| D034 | Settled | SQLite is authoritative mutable game state for the current architecture. |
| D035 | Settled | Telegram/TDLib is input/presentation, not source of game truth. |
| D036 | Settled | Important delayed game state is durable in SQLite; in-memory timers are not authoritative. |
| D037 | Settled | Reaction timing starts after the actionable Telegram presentation succeeds. |
| D038 | Settled | Avoid generic internal frameworks/DSLs until repeated concrete use demonstrates them. |
| D039 | Settled | Use bleeding-edge Rust when it creates real value; old-compiler compatibility is not a goal. |
| D040 | Settled | Lean dependencies are acceptable when useful; ask before bringing in notably heavy dependencies. |
| D041 | Open | How far automation should go. Passive nets/check-in mechanics sound promising; full idle automation is not decided. |
| D042 | Future | Monetization may use Telegram Stars/crypto/other systems, but should not distort the MVP architecture. |
| D043 | Future/Open | Telegram/NFT gift integration — catching, bait use, item representation — is interesting but requires future API/product/policy design. |
