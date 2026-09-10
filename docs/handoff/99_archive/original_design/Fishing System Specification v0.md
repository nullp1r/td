# Fishing System Specification v0

## 1. Purpose

The fishing system is the first fully developed activity in the game.

It should prove the broader activity architecture without attempting to become a universal engine prematurely.

The system must support:

- quick ordinary catches;
- multi-step encounters;
- reaction timing;
- bait;
- equipment;
- environmental conditions;
- hidden and explicit modifiers;
- species-specific behavior;
- discovery;
- procedural catch variation;
- extensibility toward much stranger future forms of “fishing”.

---

# 2. High-Level Flow

A fishing attempt follows this conceptual pipeline:

**validate → prepare → cast → wait → generate encounter → reveal observations → player reacts → resolve encounter → generate catch → persist result → update discoveries/progression**

Not every stage must correspond to a Telegram message.

Most stages are internal game state transitions.

---

# 3. Fishing Session

A fishing session represents the player being actively prepared to fish at a location.

Conceptually:

```text
FishingSession {
    character_id
    location_id
    equipment_snapshot
    bait_selection
    environment_snapshot_or_reference
    state
}
```

A session may persist across multiple casts.

The session ends when:

- the player leaves;
- starts traveling;
- changes to an incompatible activity;
- manually exits;
- the session expires if expiration is required.

For MVP-0, session persistence can remain simple.

---

# 4. Cast State Machine

Suggested first state machine:

```text
Ready
  ↓ Cast
WaitingForBite
  ↓
EncounterActive
  ↓
Resolving
  ↓
Caught | Escaped | NoCatch
  ↓
Ready
```

Exceptional encounters may contain substates:

```text
EncounterActive
    ├── InitialBite
    ├── Hooked
    ├── Struggling
    ├── Opportunity
    └── FinalAttempt
```

Do not force every species through every state.

---

# 5. Cast Validation

Before starting a cast, validate:

- player is at a fishable location;
- player is not traveling;
- player has usable equipment;
- player has compatible bait if bait is required;
- player has no conflicting active action;
- current location permits fishing;
- any special equipment requirements are satisfied.

Failure should return a player-readable reason.

Examples:

> You don't have any usable bait.

> The water here is frozen solid.

> Your current rod cannot be used in lava.

These are capability restrictions, not generic errors.

---

# 6. Bait Consumption

Do not necessarily consume bait immediately on cast.

Bait can be consumed at configurable stages:

- on cast;
- on bite;
- on successful hook;
- on catch;
- probabilistically on failure.

Different bait may behave differently later.

For MVP-0, a simple rule is acceptable:

**bait is consumed when a valid bite occurs.**

This avoids punishing players for empty casts while still creating resource pressure.

The exact rule should remain configurable.

---

# 7. Waiting for a Bite

After casting, determine:

```text
bite_delay
```

The delay may depend on:

- location;
- fishing activity;
- equipment;
- bait;
- weather;
- game time;
- species ecology;
- buffs/debuffs.

For MVP-0, delay should usually stay short enough for active Telegram play.

Example practical range:

```text
2–12 seconds
```

Rare special events may differ.

The delay should not be interpreted as a hard long-term rule.

---

# 8. Encounter Generation

When the bite resolves, the game selects an encounter.

The important distinction is:

**select an encounter/species first, then generate its individual specimen.**

Do not generate a random fish first and attempt to retrofit conditions afterward.

Conceptually:

```text
eligible encounters
    ↓
apply conditions
    ↓
apply attraction/modifiers
    ↓
weighted selection
    ↓
generate individual encounter instance
```

---

# 9. Eligibility

Each encounter has requirements.

Example:

```text
StormEel {
    location_tags: [coast, deep_water]
    required_weather: [rain, storm]
    allowed_time: [evening, night]
}
```

Requirements should support:

- location;
- biome;
- depth;
- game time;
- weather;
- season;
- world state;
- equipment capability;
- bait properties;
- discoveries;
- quest/world-event state.

MVP-0 only needs a subset.

The system should distinguish between:

### Hard requirements

If false, the encounter cannot occur.

### Soft preferences

If present, chance changes but does not necessarily become zero.

This distinction is essential.

---

# 10. Weighted Selection

Each eligible encounter has a base weight.

Example:

```text
HarborPerch: 100
SilverMinnow: 80
MudCrab: 30
GlassMinnow: 2
```

Then modifiers alter effective weight.

Conceptually:

```text
effective_weight =
    base_weight
    × location_modifier
    × bait_modifier
    × weather_modifier
    × time_modifier
    × equipment_modifier
    × hidden_world_modifier
```

Do not expose this formula directly to players.

The precise math can change later.

MVP-0 should favor understandable multiplicative or additive modifiers rather than overly complex probability systems.

---

# 11. No-Catch Outcomes

Not every cast necessarily produces a valid creature.

Possible outcomes:

- no bite;
- bait stolen;
- junk;
- environmental interaction;
- fish;
- treasure;
- special encounter.

MVP-0 can use a simple weighted encounter table where non-fish outcomes are peers of fish encounters.

This avoids hardcoding the assumption that fishing returns fish.

---

# 12. Encounter Instance

After selecting the encounter archetype, generate an encounter instance.

Example:

```text
FishingEncounter {
    encounter_id
    character_id
    archetype_id
    generated_specimen_seed
    phase
    tension
    control
    escape_pressure
    started_at
    bite_at
    decision_deadline
    observations
    modifiers
}
```

Many fields may remain unused in simple encounters.

The structure should allow complexity without requiring it.

---

# 13. Encounter Difficulty

Encounter difficulty should emerge from several dimensions rather than one number.

Possible hidden properties:

- mass;
- strength;
- aggression;
- speed;
- unpredictability;
- endurance;
- depth preference;
- evasiveness;
- line stress;
- hook difficulty.

MVP-0 may compress these to approximately:

```text
power
speed
behavior
```

But the internal API should avoid assuming there will only ever be one scalar “difficulty”.

---

# 14. Player Capability

Fishing capability comes from multiple sources.

Conceptually:

```text
equipment
+ bait
+ talents
+ traits
+ temporary effects
+ environment
+ knowledge-driven choices
```

For MVP-0, most comes from equipment and bait.

Example equipment properties:

```text
control
line_strength
sensitivity
casting_range
reaction_assist
```

The player-facing UI does not need to expose every internal value.

---

# 15. Reaction Timing

When a reaction opportunity begins, record:

```text
opportunity_started_at
action_received_at
```

Calculate:

```text
reaction_duration
```

Map the result into broad bands.

Example:

```text
Excellent
Good
Late
Missed
```

These bands should be species/encounter-dependent.

A slow fish may have:

```text
Excellent: < 1.5 s
Good: < 3.5 s
Late: < 6 s
```

A fast species may be stricter.

Do not use sub-second windows as the primary game mechanic.

---

# 16. Latency Handling

Server-side timing should measure from when the interaction becomes valid in the system to when the server receives the player action.

Because Telegram/network latency is outside the player's control, the system should be conservative.

Possible later mitigation:

- generous windows;
- client-visible countdown-independent timing;
- reaction categories rather than exact rankings;
- special compensation for known interaction delivery latency;
- no millisecond leaderboard for normal fishing.

Group races may use stricter first-response rules if clearly presented as such.

---

# 17. Player Actions

MVP-0 should support a small action vocabulary.

Potential actions:

```text
Reel
Wait
Pull
GiveLine
CutLine
UseAbility
```

Do not expose all actions at once.

Available actions depend on encounter phase.

Ordinary catches might only offer:

```text
Reel
```

Intermediate encounters:

```text
Reel
Wait
```

Exceptional encounters:

```text
Pull
Give Line
Use Equipment
Cut Line
```

---

# 18. Action Resolution

Each action modifies encounter state.

Example:

```text
Pull:
    control += rod_control
    tension += target_strength

GiveLine:
    tension -= amount
    escape_pressure += amount

Wait:
    target_behavior advances
```

The important design rule is:

**actions should create tradeoffs.**

Avoid fake choices where one button is always mathematically correct.

---

# 19. Tension

Exceptional encounters may use a tension mechanic.

Example conceptual range:

```text
0 ------------------- 100
loose                  snapped
```

Possible outcomes:

- too little tension → fish may escape;
- too much tension → line may break;
- moderate tension → progress toward landing.

The exact numbers should stay hidden or partially abstracted unless exposing them improves UX.

Player presentation might use:

```text
Tension: Stable
Tension: High
Tension: Critical
```

or a visual bar.

---

# 20. Control / Progress

Exceptional encounters may also track landing progress.

Example:

```text
control_progress: 0–100
```

Successful decisions raise it.

Poor decisions reduce it or increase danger.

At threshold:

```text
landed = true
```

This creates combat-like structure without requiring health bars.

---

# 21. Species Behavior

Each species archetype can define behavior patterns.

Examples:

### Perch

- predictable;
- short bite window;
- low strength.

### Pike

- sudden burst;
- pauses after first pull;
- punishes immediate over-reeling.

### Eel

- erratic;
- changes direction;
- benefits from giving line.

### Sturgeon

- slow but extremely powerful;
- long encounter;
- equipment check.

Behavior should matter more than arbitrary rarity.

---

# 22. Observations

The player does not see hidden simulation values directly.

The system converts them into observations.

Example hidden state:

```text
strength = high
speed = low
depth = deep
element = electric
```

Possible observations:

> Something very heavy takes the line.

> It barely moves at first.

> The line angles sharply downward.

> Tiny sparks flicker across the water.

Observations can have multiple text variants.

---

# 23. Observation Model

Avoid storing only finished prose where possible.

Conceptually:

```text
Observation {
    kind
    intensity
    subject
    visibility
}
```

Example:

```text
kind = PullStrength
intensity = Heavy
```

Presentation selects from several phrases.

This allows:

- Rich Message rendering;
- localization later;
- different detail levels;
- contextual descriptions;
- future Mini App visualization.

---

# 24. Imperfect Information

Not every observation should be exact.

A player may observe:

> Something heavy.

even if the internal specimen weighs 31.7 kg.

Equipment, traits, knowledge, or research may improve observation accuracy later.

Example:

starter:

> Something large is pulling.

advanced sonar:

> Estimated mass: 25–40 kg.

expert scanner:

> Estimated mass: 31–34 kg.

This creates another capability progression axis.

---

# 25. Failure

Failure should resolve to a meaningful cause.

Possible causes:

- missed reaction;
- insufficient control;
- snapped line;
- hook failure;
- wrong action;
- target escaped;
- incompatible equipment;
- environmental hazard.

The game should report immediate causality.

Example:

> The creature surges downward. Your line reaches its limit and snaps.

It should not necessarily reveal the optimal solution.

---

# 26. Anti-Frustration Rules

For progression-critical encounters:

- avoid extreme unbounded RNG;
- allow knowledge to improve odds;
- allow equipment to meaningfully improve reliability;
- consider anti-drought systems later.

For optional collector content:

- extreme rarity is acceptable;
- procedural exceptional specimens may be effectively unique.

---

# 27. Successful Catch Generation

After landing a species, generate the individual catch.

Possible fields:

```text
Catch {
    item_id
    species_id
    owner_id
    size
    weight
    quality
    generation_seed
    caught_at
    location_id
    bait_item_id
    equipment_snapshot
    weather
    game_time
    encounter_id
}
```

Some metadata may later be compressed or moved to provenance records.

---

# 28. Size Generation

Each species defines a size distribution.

Avoid uniform random ranges.

Example:

```text
normal_size = 20–35 cm
large = 35–50 cm
exceptional = 50+ cm
```

Internally, use a skewed distribution so ordinary specimens cluster naturally while extreme specimens remain rare.

MVP-0 does not need biologically perfect modeling.

The distribution should support:

- common catches;
- notable personal records;
- rare extreme records.

---

# 29. Weight Generation

Weight should correlate with size rather than being independently random.

Conceptually:

```text
weight ≈ species-specific function(size) × variation
```

This prevents absurd combinations unless intentionally procedural.

Later, morphology and condition can alter weight.

---

# 30. Procedural Seed

Each non-trivial individual catch should have a stable generation seed.

This allows future deterministic derivation of additional traits without storing every field immediately.

Example future use:

```text
seed
→ coloration
→ mutation
→ temperament
→ morphology
```

Be careful not to make future updates retroactively reinterpret old items unpredictably.

Version procedural generation rules.

---

# 31. Item Identity

Valuable or unique catches should have stable IDs.

This is important for future:

- provenance;
- trading;
- records;
- ownership history;
- trophies;
- external asset representation.

Do not reduce every fish to:

```text
FishType × Quantity
```

unless the species is intentionally treated as a commodity.

---

# 32. Bait Model

Suggested internal bait properties:

```text
BaitProfile {
    fishing_power
    tags
    attraction_modifiers
    repel_modifiers
    side_effects
}
```

Example:

```text
GlowLarva:
    power = 12
    tags = [organic, luminous, insect]
```

Species may respond to bait tags.

Example:

```text
CaveEel:
    luminous × 4
    insect × 1.5
    metallic × 0.2
```

---

# 33. Bait Side Effects

Bait should eventually be able to modify more than encounter probability.

Examples:

- increases average size;
- attracts predators;
- shortens bite delay;
- makes fish more aggressive;
- increases junk chance;
- damages equipment;
- attracts rare non-fish encounters;
- affects specimen traits.

MVP-0 only needs a few examples.

---

# 34. Equipment Model

Suggested equipment properties:

```text
Rod {
    control
    power
    sensitivity
    range
    durability?
    tags
}
```

Other equipment slots may later include:

- line;
- hook;
- reel;
- bobber;
- accessories;
- fishing device modules.

Do not commit MVP-0 to all of these slots.

Start with rod plus perhaps one secondary slot if it creates useful choices.

---

# 35. Environmental Context

At encounter generation time, build a context.

Example:

```text
FishingContext {
    character
    location
    biome
    game_time
    weather
    world_state
    bait
    equipment
    active_effects
}
```

Species eligibility and modifiers consume this context.

This is preferable to every subsystem querying global state independently.

---

# 36. Catch Selection Pipeline

Concrete conceptual pipeline:

```text
1. Build FishingContext

2. Gather encounters available at location

3. Reject encounters failing hard requirements

4. Calculate encounter weights

5. Apply bait attraction/repulsion

6. Apply environmental modifiers

7. Apply equipment/capability modifiers

8. Apply world/event modifiers

9. Select encounter

10. Generate encounter instance

11. Resolve player interaction

12. If successful, generate individual catch

13. Apply discovery/progression/economy effects

14. Persist
```

Keep these stages observable in debug tooling.

---

# 37. Discovery Integration

On successful catch:

```text
if species not personally discovered:
    create discovery
    grant discovery XP
    emit first-discovery event
```

Potential future checks:

```text
if species never discovered globally:
    create world-history record

if trait combination novel:
    create special discovery

if hidden condition observed enough:
    advance knowledge
```

MVP-0 only needs personal species discovery plus selected special discoveries.

---

# 38. Global Firsts

Even if MVP-0 has only a few testers, architect first-discovery checks transactionally.

Potential events:

```text
FirstGlobalSpeciesCatch
FirstLocationDiscovery
RecordCatch
```

Avoid race conditions where two players can both become “first”.

This will matter later.

---

# 39. XP Integration

XP sources may include:

```text
base catch XP
× rarity modifier
+ first discovery bonus
+ exceptional specimen bonus
+ encounter difficulty bonus
```

Avoid tying XP directly to every hidden numeric property.

Level remains mostly informational for now.

---

# 40. Economy Integration

Successful catches may have a derived sale value.

Example factors:

```text
species base value
× size modifier
× rarity modifier
× quality modifier
```

Do not allow value formulas to become the main reason a catch is interesting.

Some catches should have more value as:

- bait;
- clues;
- crafting materials;
- collection pieces;
- quest items;
- trophies.

---

# 41. Rich Message Presentation

Fishing presentation should be generated from game state, not contain game rules itself.

Conceptual separation:

```text
FishingEncounter
    ↓
FishingEncounterView
    ↓
Telegram Rich Message renderer
```

The renderer decides:

- headings;
- tables;
- emphasis;
- buttons;
- progress/tension presentation;
- whether to edit an existing message;
- whether to create a new significant event message.

---

# 42. Message Traffic

The game should avoid one outbound message per state transition.

Prefer:

```text
one active interaction message
```

that changes throughout the encounter.

Persistent new messages should represent meaningful events.

Examples:

- new species;
- exceptional record;
- level-up;
- major clue;
- rare catch.

---

# 43. Concurrency

A player should normally have at most one active physical action that conflicts with fishing.

Examples:

```text
Fishing
Traveling
Combat
Exploring
```

This should eventually be represented as activity/state constraints rather than fishing-specific booleans.

Avoid:

```text
is_fishing
is_traveling
is_exploring
```

becoming dozens of unrelated flags.

---

# 44. Timers

Fishing requires delayed events.

Examples:

- bite occurs;
- reaction window expires;
- encounter phase timeout.

Timers should not depend on keeping an in-memory async task alive indefinitely.

Persist enough state to recover delayed activities after restart.

Possible architecture later:

```text
scheduled event
→ durable queue / database timer
→ game command
```

Exact implementation belongs in the architecture document.

---

# 45. Idempotency

Telegram callbacks and network retries may produce duplicate actions.

Gameplay actions must be idempotent where appropriate.

Examples:

- pressing Reel twice must not catch two fish;
- repeated callback delivery must not consume bait twice;
- duplicate resolution events must not create duplicate items.

Encounter state transitions should validate expected state/version.

---

# 46. Optimistic Concurrency

Each active encounter should have a revision/version or equivalent concurrency guard.

Example:

```text
encounter_revision = 5
```

Action:

```text
Reel(expected_revision = 5)
```

If the encounter has already advanced to revision 6, reject or gracefully ignore the stale action.

This will matter with Telegram interactions and asynchronous timers.

---

# 47. Debug Information

Developer mode should expose the entire selection pipeline.

Example:

```text
Selected: Glass Minnow

Base weight: 2
Location: ×1
Night: ×3
Rain: ×1
Glow Larva: ×4
Equipment: ×1

Final weight: 24
```

Also expose:

- random roll;
- eligible encounters;
- rejected encounters and reasons;
- timing windows;
- specimen generation seed.

Do not expose this to ordinary players.

---

# 48. Deterministic Testing

Game logic should accept deterministic RNG sources.

This allows tests such as:

```text
given:
    Old Harbor
    rain
    Glow Larva
    specific RNG seed

expect:
    Glass Minnow encounter
```

Do not bury randomness directly inside handlers using global random calls.

---

# 49. Content Definitions

Species content should eventually resemble:

```text
Species {
    id
    name
    habitats
    base_weight
    requirements
    bait_preferences
    behavior
    specimen_generator
    economy
    discovery
}
```

Avoid creating a massive universal schema immediately.

Some exceptional species may require custom Rust behavior.

Data-driven content and custom behavior should coexist.

---

# 50. MVP-0 Fishing Content Minimum

To test the system meaningfully, MVP-0 should contain:

- 15–25 species;
- 5–8 bait types;
- 4–6 equipment choices;
- 3 fishing locations;
- 3–4 weather/time interactions;
- at least 3 behavior archetypes;
- at least 1 intentionally difficult early encounter;
- at least 1 non-fish progression catch;
- at least 1 rare collector-oriented catch.

This is enough diversity to reveal structural flaws.

---

# 51. What Not to Generalize Yet

Do not build:

- generic visual scripting;
- generic arbitrary-condition DSL;
- generic ECS framework unless independently justified;
- universal activity scripting;
- distributed simulation infrastructure;
- procedural ecosystem simulation;
- generic MMO combat engine.

Implement fishing concretely.

Extract abstractions only where actual repeated patterns appear.

---

# 52. Core Engineering Invariants

The first implementation should preserve these invariants:

1. Game logic is independent from Telegram rendering.
2. Randomness can be controlled in tests.
3. Delayed actions survive process restarts.
4. Player actions are safe against duplicate delivery.
5. Active encounter state has concurrency protection.
6. Catches can have persistent individual identity.
7. Species selection is explainable in developer tooling.
8. Content can be expanded without rewriting core handlers.
9. Fishing is an activity, not the definition of the entire game.
10. Rich Messages are presentation, not domain state.

---

# 53. First Implementation Order

A practical order:

### Phase A
- player;
- location;
- inventory;
- item definitions;
- persistence.

### Phase B
- fishing context;
- species definitions;
- weighted encounter selection;
- simple catch generation.

### Phase C
- cast state machine;
- delayed bite;
- reaction timing;
- success/failure.

### Phase D
- bait modifiers;
- equipment modifiers;
- weather/time.

### Phase E
- discovery;
- XP/levels;
- selling/shop.

### Phase F
- intermediate encounters;
- exceptional encounter;
- Rusted Key progression chain.

### Phase G
- telemetry;
- debug tooling;
- tuning based on friend testing.

At the end of Phase C, there should already be something playable.

At the end of Phase F, MVP-0 should resemble the intended vertical slice.

---

# 54. Central Design Rule

Fishing should not feel deep because every catch is complicated.

It should feel deep because a simple action sits on top of a world containing:

- hidden conditions;
- different behaviors;
- equipment interactions;
- bait interactions;
- geography;
- weather;
- timing;
- discovery;
- procedural variation;
- progression;
- mysteries.

The player should be able to understand:

**“Cast and reel.”**

while still spending years learning:

**“What exactly can happen when I cast here, with this, under these conditions?”**