# Fishing — Current Model Summary

> **Status:** settled foundation; full historical spec archived separately

The detailed original design is preserved in `99_archive/original_design/Fishing System Specification v0.md`. This file is the shorter handoff summary.

## Intended rhythm

- Most ordinary catches: one meaningful reaction.
- Some stronger catches: an additional decision/struggle.
- Exceptional encounters: may become richer multi-step sequences.

## Timing

Timing matters, but the game must not become a latency benchmark.

- broad windows;
- late/poor timing changes outcome/quality;
- first catches can be forgiving;
- presentation success precedes start of reaction timing;
- Telegram/network delay is not counted as player reaction time.

## Imperfect information

Players should infer what is happening from observations such as:

- float/line movement;
- pull strength;
- depth;
- timing pattern;
- environment;
- textual/visual clues.

Observation text should have multiple variants and correlate with actual behavior rather than being decorative noise.

## Selection pipeline

Conceptually:

1. validate character/location/equipment/bait;
2. snapshot environment;
3. determine eligible encounter/species;
4. weight by location/bait/conditions;
5. persist authoritative encounter + timer;
6. present actionable state;
7. start reaction deadline after successful presentation;
8. resolve action idempotently;
9. generate deterministic specimen;
10. record history/discovery/economy consequences.

## Difficult catches

The prototype used control and a short struggle state with actions such as Pull / Give Line. Exact future encounter design can deepen as content demands it.

## Anti-frustration

- no millisecond-perfect valuable gates;
- no bait soft lock;
- no ordinary catastrophic equipment destruction;
- mandatory progression should avoid brutal uncontrolled RNG;
- stale/double callback actions must be harmless/idempotent.
