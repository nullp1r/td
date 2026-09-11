# Content, localization and live-world composition

## Locked boundary: definitions versus presentation text

Structured content and localized rich text are separate concerns.

Structured definitions should hold IDs/references, numeric/range data, tags, requirements, topology, encounter references, item properties and other machine-validated semantics. They reference localized text keys/documents rather than embedding final English throughout the definitions.

Dialogue, knowledge articles and other rich prose should render through an internal RichText/document representation. Telegram/TDLib wire types remain boundary types rather than authored/persisted game semantics.

## Open authoring formats

Current candidates/directions include:

- RON or another human-friendly structured definition format;
- other structured-source candidates such as KDL/TOML/Nickel where empirical authoring/tooling tests justify them;
- Fluent for localization;
- Markdown or a constrained rich-document source for long-form knowledge articles.

These are not locked. JSON is rejected as the preferred hand-authored game-content format. The first implementation should choose the smallest coherent toolchain that serves real content rather than inventing a generalized content framework.

## Implemented invariant: startup validation

Keep startup validation of content cross-references, ranges and impossible combinations. Invalid authored content is a startup/build-time error where practical, not a silent runtime fallback.

## Leaning: stable semantic content IDs

Human-readable stable IDs such as `old_harbor`, `mara_reed` and `black_glass_compass` are suitable authored references. Persistence may map these to compact internal/numeric IDs where beneficial. Pre-MVP IDs may be replaced during the reset; backward compatibility with current content identifiers is not a goal.

## Leaning: world conditions compose opportunities

Time, weather and local conditions should alter descriptions, routes, NPC behavior, encounter eligibility and available opportunities. Conditions are not merely a decorative global “weather” line.

A canonical event can inject opportunities into ordinary locations. Example: a storm changes route safety while active and may leave debris/damage/new search opportunities afterward. Notifications should describe the world phenomenon rather than expose raw loot-rate bonuses when possible.

## Leaning: authored fragments plus separate presentation RNG

Location/encounter prose can be assembled from compatible localized fragments to avoid identical repeated text. Presentation randomness must use a separate named RNG domain from gameplay state selection.

Variation should remain coherent with canonical facts. Never randomize prose in a way that contradicts actual weather, NPC position, object state or character knowledge.

## Leaning: meaningful history filters presentation

The canonical event/history store will grow much faster than the UI can display. Present contextualized history using relevance, significance, visibility, decay and knowledge rules rather than exposing a raw activity log.

The same durable event may support several views:

- local trace/history;
- item provenance;
- character profile/record;
- NPC reaction/rumor;
- notification;
- shared Telegram card.

## Open

- Exact structured-content syntax and compilation/loading pipeline.
- Localization key organization and grammatical-variable strategy.
- Rich-text AST/document schema and authoring source.
- Content hot reload versus restart-only loading during development.
- Versioning/migration strategy for persistent encounters/items that reference content changed by deployment.
- How world-condition generation is partitioned between deterministic derivation and persisted event/state rows.
