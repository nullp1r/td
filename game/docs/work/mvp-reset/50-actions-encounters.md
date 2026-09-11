# Actions, encounters and randomness

## Leaning: small action kernel, bespoke mechanics above it

Standardize lifecycle concerns shared by world interactions:

```text
actor
location/context
target
affordance validation
capability/knowledge checks
scope
timing
RNG root/derivation
transaction/idempotency
state changes
meaningful domain events
presentation
```

Do not force fishing, lockpicking, travel, dialogue and trade into one universal skill-roll DSL. Use typed Rust resolvers for real mechanic families; make content data-driven only where stable repeated semantics have been demonstrated.

## Leaning: affordance is derived availability

An affordance is a contextual verb currently available to an actor/target pair. A locked hatch may expose `Inspect`, then `Unlock` and/or `Pry` depending on canonical hatch state and the character's capabilities/knowledge.

The target should not merely store a fixed list of buttons.

## Leaning: encounters are persistent only when interaction needs state

Atomic actions remain atomic. Multi-step interactions become persistent encounters when Telegram latency/retries, branching choices, multiple participants or elapsed time require durable state.

A persistent encounter likely needs at least:

```text
id
kind
scope
target
participants
state/revision
started_at
expires_at? / timing data
RNG root
```

Callbacks are duplicate/stale-capable. Mutation validates actor, current revision/state and current world constraints, resolves once transactionally, then renders committed state.

## Leaning: explicit encounter scope

Distinguish at least:

- personal;
- party;
- shared local/object;
- world.

A canonical environmental condition can feed personal encounters. A genuinely unique shared object uses canonical contention semantics so two simultaneous clicks cannot both independently consume/open it.

## Leaning: semantic RNG domains

Keep algorithm constants that belong to the PRNG algorithm. Remove hand-authored gameplay separator constants such as arbitrary hex `SPECIES_SEED`/`BITE_SEED` values.

Preferred conceptual API:

```text
root.derive("encounter")
root.derive("loot")
root.derive("quality")
root.derive("presentation")
```

Derivation hashes the root plus semantic domain/context into deterministic PRNG state. Presentation randomness must be independent so adding flavor variants never changes gameplay outcomes.

Canonical world phenomena may derive from world seed + stable context/time period. Individual committed uncertain interactions may store a fresh RNG root so retries/crash recovery reproduce the same result.

**Open:** exact hash primitive/API/versioning; BLAKE3 is only a candidate, not a locked dependency.

## Leaning: deterministic rules, uncertain consequences

Do not randomize whether an explicitly satisfied rule works. Correct key opens compatible lock; sufficient leverage permits the pry action. Randomness may govern natural uncertainty around outcome details such as wear, noise, contents, reaction, behavior or presentation.

## Leaning: no infinite identical lottery pulls

Repeated actions must consume/change relevant state or become gated by changing conditions/knowledge/opportunities. `Explore` should primarily reveal opportunities/POIs/traces rather than dispense a fresh loot roll every click.

Interesting/rare outcomes should often depend on eligibility conditions (place, weather, tide, recent world event, knowledge, tools) rather than microscopic independent probabilities that reward scripting the same action thousands of times.

## Leaning: repeated activities vary structurally, not only textually

A mechanic should not always traverse the same interaction topology with randomized flavor. Fishing is the clearest legacy example: avoid a universal `cast → wait → bite → reel → catch` pipeline.

A cast or comparable world action may resolve immediately, branch into a snag/behavior choice, reveal an environmental clue, create another opportunity, produce an object, or do nothing useful. Trivial encounters should remain trivial; only interactions that need state should become multi-step encounters.

The same principle applies to scavenging, travel, ruins and other activities: randomization should sometimes change what kind of situation occurred, not merely which noun appeared at the end.

## Leaning: outcomes are broader than loot

Actions may produce:

- items;
- knowledge;
- POIs/routes;
- NPC dialogue topics;
- injuries/effects/conditions;
- relationship/reputation change;
- party/social opportunities;
- canonical object/world changes;
- meaningful domain events;
- information with no immediate material reward.

Mundane “nothing useful” outcomes are valid when the interaction is not an invitation to spam the same roll indefinitely.

## Leaning: players can create opportunities for other players

Canonical player actions may alter shared objects, open routes, move/trade/drop significant objects, create traces, change merchant/world state or otherwise expose opportunities that another character later encounters.

This is a major source of MMO texture: players can become part of the world's content generation without a separate user-generated-content subsystem.

## Leaning: choices need semantic differences

Branching encounter choices should alter understandable variables or state. Avoid cosmetically different buttons that all call the same undifferentiated random roll.

Use stateful/randomized behavior (weighted transitions, cooldowns/shuffle bags where appropriate) when independent rolls would create obviously repetitive sequences.

## Leaning: actions and durable domain events are different layers

Not every player action deserves immortal history. `Inspect`, `open inventory` or an ordinary failed search can execute through the action infrastructure without producing a durable domain event.

Emit durable events when the consequence is meaningful history: discovery, significant acquisition, ownership transfer, record, task/world-state transition, party formation and similar facts that other systems may later need to remember/react to.

## Leaning: idempotency and timestamps are infrastructure rules

Economically or statefully meaningful commands must have idempotent execution semantics. Timed correctness is based on persisted timestamps; schedulers deliver reminders/notifications rather than owning completion truth.

Preserve the UI rule: **panels for state; messages for events**. Mutable location/encounter state can edit panels, while meaningful discoveries/trades/completions should leave durable conversational history where appropriate.

## Open

- Concrete Rust action/encounter type boundaries after the first non-fishing mechanic set is selected.
- How action identity/idempotency records are retained/compacted.
- Exact contention UX for shared objects.
- Which encounter behaviors warrant generic data definitions versus bespoke Rust resolvers.
