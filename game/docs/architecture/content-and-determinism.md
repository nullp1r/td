# Content, IDs and determinism

## Content definitions

`game/content/game.json` is the current source for immutable bait, rod, location and species definitions.

`Content::prepare` sorts collections for predictable binary-search lookup, then startup validation checks cross-references and domain constraints before gameplay begins.

Prefer simple data-driven content where repeated entries clearly benefit. Do not create a generic condition/quest/content DSL ahead of concrete requirements.

## Typed IDs

Domain IDs are small newtypes rather than interchangeable integers. Keep them typed through application/game logic and unwrap only at persistence/protocol boundaries.

Persisted IDs can become compatibility contracts. Adding new definitions is generally safer than renumbering existing ones.

Known examples where IDs/history matter:

- species/bait/rod/location IDs;
- recipe IDs in `craft_consumptions`;
- objective IDs in `objective_claims`;
- title IDs on characters;
- callback protocol tags;
- discovery kind/subject IDs;
- timer/encounter phase/kind tags.

Document a new persisted tag where it is interpreted.

## Deterministic world clock

`world.rs` derives environment from wall-clock milliseconds. No continuously advancing “world tick” needs persistence for the current model.

Current parameters:

- game day = 2 real hours;
- weather slot = 15 real minutes;
- deterministic Clear/Rain/Fog selection;
- fixed day-part boundaries in fictional minutes.

The same timestamp yields the same current environment.

## Deterministic RNG

`rng.rs` provides a small SplitMix64-based deterministic generator.

Fishing deliberately separates random domains using different seed XOR constants for:

- species selection;
- bite timing;
- specimen generation.

This prevents adding/removing a draw in one subsystem from silently changing another subsystem's existing deterministic results.

When adding a new deterministic mechanic, consider whether it deserves an independent RNG domain rather than consuming from a shared sequence.

## Specimen generator versioning

Every persisted catch records `seed` and `generator_version`.

That is a compatibility affordance. If future procedural traits/art/morphology derive from seeds, do not assume a new algorithm can reinterpret all historical items without consequence.

Options later may include:

- preserve old generator semantics by version;
- materialize new traits for existing items during an explicit migration;
- only apply new traits to newly generated versions.

Decide deliberately before a beloved old specimen can visually/mechanically mutate because code changed.

## Content growth

The current content layer can scale much further without a generalized framework. Optimize only when real size/access patterns demand it.

Useful principles:

- validate once at startup;
- keep runtime representation compact/read-only;
- make lookup predictable;
- preserve stable IDs/provenance;
- encode player-visible clues separately from hidden raw formulas;
- let content patterns earn new abstraction.
