# Fishing and encounter design

Fishing is the first deep activity and the current mechanical spine, but its architecture and design language should be able to evolve into other kinds of encounters later.

This document separates the **implemented vertical slice** from the **long-term encounter model**.

## Design intent

Ordinary catches should usually be lightweight. Interesting catches can add another decision. Exceptional encounters may become short, expressive multi-step sequences.

A useful rhythm is:

- most catches: one meaningful reaction;
- some catches: an additional struggle/decision;
- rare or important encounters: richer state and several meaningful actions.

The game must not become a latency benchmark. Timing is allowed to matter; millisecond precision is not.

## Conceptual fishing flow

The durable conceptual pipeline is:

1. validate character, location, equipment and selected usable bait;
2. consume/prepare the cast atomically;
3. snapshot the relevant environment;
4. determine eligible encounter outcomes;
5. weight eligible outcomes by location, bait, conditions and future modifiers;
6. persist authoritative encounter state and a durable timer;
7. present the actionable state to the player;
8. start any player-reaction deadline **after successful presentation**;
9. accept actions with ownership/phase/step validation;
10. resolve the outcome idempotently;
11. generate a stable individual specimen or other result;
12. persist catch/history/discovery/economy/progression consequences atomically.

The current code implements a compact subset of this flow in `src/app/angling.rs` and pure deterministic mechanics in `src/fishing.rs`.

## Encounter selection: eligibility vs preference

Long term, keep **hard eligibility** separate from **soft weighting**.

Hard eligibility answers whether an outcome is possible at all. Examples:

- location/biome;
- time/weather/environment;
- capability/equipment requirements;
- world/quest/relationship state;
- event state.

Soft preference changes probability among eligible outcomes. Examples:

- bait attraction;
- preparation/freshness;
- environmental affinity;
- equipment characteristics;
- knowledge/profession effects;
- local/world events.

Do not expose the exact weighting formula by default. The player should learn useful relationships from clues, observations and experience.

As conditions become richer, prefer building an explicit encounter context (character capability, place/biome, environment, bait/equipment, effects and relevant world state) and evaluating eligibility/modifiers from that snapshot/reference. Avoid a design where every modifier independently reaches into mutable global state; delayed encounters must not silently change meaning because unrelated conditions advanced after selection.

### Current implementation

Current species selection:

- starts from the current location's weighted encounter list;
- rejects species whose required weather/day parts are not active;
- applies bait-specific basis-point multipliers where defined;
- falls back deterministically to a validated encounter if every computed weight is zero;
- uses a dedicated RNG domain so selection draws do not perturb bite/specimen generation.

`game/content/game.json` is authoritative for exact encounter weights and conditions.

## Possible result categories

Do not hardwire the long-term activity into “a cast always chooses a fish.” Peer outcomes can include:

- no bite;
- bait stolen or altered;
- junk;
- environmental interaction;
- ordinary specimen;
- treasure/relic;
- clue;
- special encounter;
- something that no longer resembles literal fishing at all.

The Rusted Key is the current proof that a cast can produce a world-progression object instead of a fish.

## Waiting and bite timing

### Current implementation

Starting a cast consumes one unit of the selected bait in the same transaction that creates the encounter and its first durable timer. Each species has a deterministic seeded wait within its `bite_min_ms..=bite_max_ms`. Bait Fishing Power shortens that wait, capped so power cannot erase the waiting phase entirely.

The bite timer is stored in SQLite. An in-memory scheduler only wakes the application to inspect due durable timers.

Long term, bait/item mechanics may earn different consumption semantics (on cast, bite, hook, catch or conditional failure), but do not complicate the current rule until a concrete bait/encounter benefits from it.

### Reaction timing invariant

The player must never lose reaction time while Telegram is still delivering/rendering the actionable state.

The sequence is intentionally:

1. durable bite becomes due;
2. bot edits/presents the Bite Rich Message with the action;
3. only after Telegram accepts that presentation does the application persist the reaction deadline.

This is one of the strongest game/transport invariants and must survive refactors.

If an action somehow reaches application logic while the persisted presentation timestamp is still absent, current reaction calculation treats the action as having zero elapsed reaction time. Impossible transport ordering should resolve conservatively in the player's favor rather than manufacture a late reaction.

## Reaction bands

Current reaction quality is categorical:

- `Excellent`
- `Good`
- `Late`
- `Missed`

Species define three boundaries. Timing modifies the ability to control/land the fish rather than asking for esports-level precision.

The first catch is deliberately forgiving so onboarding cannot be blocked by weak starter capability.

Long term, timing may influence quality, opportunity, information or encounter state. Keep valuable gates broad enough that notifications, accessibility and network variance do not dominate.

## Imperfect information and observations

The player should act on in-world evidence, not hidden numbers alone.

Possible observations include:

- float movement;
- line direction/tension;
- pull strength;
- depth;
- rhythm/timing pattern;
- sounds;
- visual changes;
- weather/environment reactions;
- behavior learned from prior encounters.

Observation text/visuals should correlate with real mechanics. Experienced players should become better because they recognize patterns.

Future equipment, professions, relationships or knowledge can make observations more precise. For example, an expert may distinguish a behavior family that a new player sees only as “something heavy.”

Do not make prose the only canonical hidden state once encounter complexity grows. Structured behavior/observation concepts should drive presentation variants.

## Difficulty and control

### Current implementation

Current fish difficulty is a single content value. Rods provide base control; persistent rod condition retains most control even when worn. Reaction quality multiplies effective control. Strong fish can be impossible with weak gear even with excellent timing, except for the deliberately forgiving first catch.

### Long-term direction

Difficulty can become multidimensional when actual content needs it, e.g.:

- mass/strength;
- aggression;
- speed;
- unpredictability;
- endurance;
- depth;
- evasiveness;
- line stress;
- hook difficulty;
- environmental hazards.

Do not introduce these dimensions preemptively as generic stats. Add them when encounter behaviors/equipment choices make them legible and useful.

### Behavior archetypes

Species should eventually differ through learnable behavior, not merely rarity/difficulty numbers. Early design examples remain useful directional references:

- a perch-like creature: predictable, light, short/simple window;
- a pike-like predator: sudden burst, then a pause, punishing blind over-reeling;
- an eel-like creature: erratic direction changes where giving line can matter;
- a sturgeon-like giant: slow but extremely powerful, turning the encounter into an equipment/endurance check.

These are behavior sketches, not hardcoded canonical species rules. The durable point is that experienced players should recognize *how something behaves* and choose accordingly.

## Struggle phase

### Current implementation

Some difficult successful reels enter a short second phase. The player reads an observation and chooses:

- **Pull**;
- **Give Line**.

The seed determines the currently required response. Choosing the matching action succeeds as long as the original reaction was not missed; choosing the other action can still succeed if timing/control is sufficiently strong.

Callbacks carry the encounter step so stale/double actions cannot resolve the state twice.

### Long-term action vocabulary

Useful encounter verbs may include:

- Reel;
- Wait;
- Pull;
- Give Line;
- Cut Line;
- Use Ability.

Only show actions relevant to the current phase. Do not present a permanent wall of every possible verb.

Exceptional encounters should eventually have tradeoffs rather than one universally correct button. Tension, landing progress, fish position/behavior, consumables, equipment and environment can create choices—but only as complexity earns them.

## Specimen generation

A caught creature is an individual object, not only a species counter.

### Current implementation

Length and weight are deterministic from the encounter seed:

- a dedicated specimen RNG domain prevents unrelated random draws from changing existing outcomes;
- length averages three uniform integer draws, clustering results toward the middle of the species range without floating-point state;
- weight scales approximately with the cube of length relative to typical length;
- a small deterministic condition variation modifies weight;
- the catch stores its seed and `generator_version` in SQLite.

Sale value scales with specimen weight relative to the species typical weight, within bounded ratios.

### Long-term specimen identity

Possible future stable traits include:

- morphology;
- coloration;
- mutations;
- temperament/behavior;
- scars/marks;
- provenance/environment effects;
- rare properties;
- associated art variants.

Before adding procedural fields derived from old seeds, define generator-version semantics carefully. A software update must not silently reinterpret a treasured old specimen into a different object.

## Bait relationship

Bait is intentionally broader than one numeric bonus. See [`items-economy.md`](items-economy.md).

Current separation is useful:

- Fishing Power influences how long the player waits;
- species modifiers influence what is attracted.

Future bait may also affect aggression, specimen traits, junk probability, equipment risk, environmental reactions or exceptional encounters.

## Anti-frustration and fairness

- no millisecond-perfect high-value gates;
- Telegram/network latency does not consume reaction time before an action is visible;
- no bait soft lock;
- ordinary rod wear is recoverable and never destroys ownership;
- progression-critical rare outcomes can use anti-drought/knowledge/guaranteed paths;
- optional collector outcomes can be extremely rare;
- repeated/stale callbacks must be harmless;
- high-risk loss, if introduced, should be explicit rather than hidden in ordinary play.

The Rusted Key's current anti-drought behavior is a good example: chance creates surprise, but successful relevant play eventually guarantees progress.
