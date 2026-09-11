# World, locations and travel

## Leaning: current location is the primary game surface

The game should normally open into the character's current place and current world state rather than an abstract Home dashboard. Global character surfaces such as inventory, journal/knowledge, profile and map remain available through navigation, but they do not replace the scene.

A useful location presentation answers:

1. Where am I?
2. What is it like right now?
3. What is happening?
4. What deserves attention?
5. Who is around or has been relevant here?
6. What can I do?
7. Where can I go?

## Leaning: location versus point of interest

Create a distinct **location** when moving there meaningfully changes spatial context/presence/travel. Represent a feature as a **point of interest** when it is an interactable inside the same local context.

Example working topology:

```text
Old Harbor region

Arrivals ─ Quay ─ Market Row ─ Harbor Square
             │
         North Pier ─ Breakwater
```

A rusted ladder, locked hatch or mooring post at North Pier is normally a POI rather than another movement node.

Exact location names/topology are provisional content, not locked canon.

Physical travel is not a radio-choice UI. The fact that a character currently occupies one location should be communicated as world state, not by rendering every destination as `● / ○`. See [`25-interface-grammar.md`](25-interface-grammar.md).

## Leaning: compose scenes from state

Do not store “the current prose description” as world truth. Compose presentation from:

- authored location definition/tags/features;
- canonical time;
- canonical regional/local conditions;
- world events/object state;
- NPC positions;
- direct/recent player presence and relevant traces;
- character knowledge;
- carried/equipped capabilities;
- personal encounter/discovery state.

Presentation variation uses its own deterministic RNG domain so adding flavor text cannot alter gameplay outcomes.

## Leaning: contextual affordances, not action menus

World verbs should appear because current state affords them. Examples:

```text
water + suitable tool             → Cast
loose ground + digging capability → Dig
locked target + key               → Unlock
locked target + leverage          → Pry
nearby NPC                        → Talk
market/service                    → Trade
route                             → Travel
```

Do not show every carried tool as a generic button. Interactions are target-oriented; inspecting a locked hatch may reveal the actions currently available for that hatch.

## Leaning: routes and travel are real state

Routes are first-class definitions/objects rather than destination strings. They may carry distance, duration, travel mode, availability, requirements, risk and current-condition effects.

Timed travel should record durable state such as:

```text
from
route
to
started_at
arrives_at
```

Short travel may be presented compactly, but correctness comes from timestamps. Route encounters/interruption can be added when demonstrated useful.

The eventual map is a replaceable visualization over coordinates, hierarchy and route data. Do not make the first Telegram map UI the authoritative geography representation.

## Leaning: presence is contextual, not surveillance

Useful presence forms include:

- direct nearby character/NPC;
- recently present/passed through;
- consequence of somebody's action;
- local record/discovery history;
- local trade/party opportunity.

Avoid a raw global activity feed and avoid false precise “online” surveillance. Contextual traces should be selected by location relevance, significance, visibility, decay and character knowledge.

## Leaning: starter hub remains useful

The first region should not become abandoned tutorial content. Long-term reasons to revisit may include transport, storage/market access, specialist NPCs/craftspeople, relationships, region-specific resources, world events, records and unresolved mysteries.

Separate **service availability** from **NPC availability**. A service that must remain accessible can have staffing/infrastructure semantics; a specific character such as Mara should not be forced to stand awake in one spot forever.

## Leaning: three availability layers

A starter region should remain playable at an awkward real-world hour or during inconvenient canonical weather without making schedules meaningless.

Design opportunities in three broad layers:

- **invariant** — movement, inspection, basic orientation, recent player traces, at least some safe interaction/service;
- **scheduled/conditional** — named NPCs, shops, transport, tide/weather/time-sensitive opportunities;
- **emergent** — parties, player offers, discoveries, temporary world events and consequences of recent actions.

This is not a requirement that every service be open 24/7. It is a requirement that the game not become a dead screen at 03:17 because all authored content assumed daytime.

## Open

- Exact starter-region graph, coordinates and travel times.
- Exact condition taxonomy beyond time/weather and likely tide/local environmental conditions.
- How player presence is privacy-filtered and how stealth/specialization may later alter visibility.
- Map rendering experiments in Telegram.
- Which services are invariant versus scheduled.
- Whether short travel should ever be instant for UX or always represented as a timed transition.
