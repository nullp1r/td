# MVP reset implementation plan

This is sequencing guidance, not a compatibility contract. Each phase should leave a runnable/testable game where practical, and every landed subsystem must migrate its durable truth into canonical `game/docs/`.

## Current gate: finish design convergence first

Do not begin the destructive schema/content reset merely because this implementation sequence exists. Before Phase 0, resolve enough of the remaining design frontiers to avoid encoding obvious contradictions:

- NPC/dialogue/knowledge/task semantics;
- Telegram social/multi-participant behavior that requires platform R&D;
- content authoring/localization choices sufficient for the first slice;
- progression/death/failure/economy structure;
- first entry point/starter-region slice after the above systems are coherent.

Concrete code experiments are still appropriate when they answer one of those questions; the restriction is against prematurely freezing the replacement production schema.

## Phase 0 — demolition and salvage

- [ ] Inventory current fishing-specific schema, content and modules by whether they are reusable mechanism or obsolete product assumption.
- [ ] Remove/replace the group shoal and fishing-centric progression/product assumptions.
- [ ] Reset pre-MVP migrations/schema/content identifiers where a clean model is simpler than compatibility.
- [ ] Preserve useful Telegram/Rich Message infrastructure and empirically verified interaction grammar.
- [ ] Preserve transactional mutation, durable timer, duplicate/stale callback and reaction-timing invariants.
- [ ] Preserve useful PRNG primitives while replacing hand-authored gameplay domain-separator constants with semantic derivation.
- [ ] Keep the pre-existing unrelated `tdx` working-tree state outside this reset's changes unless a demonstrated game requirement needs it.

**Exit condition:** old product assumptions no longer constrain the replacement schema/API design; the project still builds enough of a minimal game shell to continue incrementally.

## Phase 1 — world kernel

- [ ] Account persistence separated from character persistence.
- [ ] Multiple-character-capable ownership model, even if MVP UI exposes one.
- [ ] Character lifecycle including draft/creating/active and atomic world entry.
- [ ] Regions/locations/coordinates/routes/entry points.
- [ ] Canonical timestamp/world-condition foundations.
- [ ] Meaningful durable domain-event store/query paths without full event sourcing.
- [ ] Minimal current-location scene replacing the old location selector mentality.

**Exit condition:** a character can be created, enter the canonical world once, persist at a real location and move through a small route graph while seeing canonical conditions.

## Phase 2 — items and capabilities

- [ ] Item type definitions support stack policy.
- [ ] Unique item instances support owner/location/current state.
- [ ] Provenance/history for significant item events.
- [ ] Inventory/container model selected and implemented.
- [ ] Equipment positions sufficient for the first slice.
- [ ] Capability aggregation and composable interaction requirements.
- [ ] Currency/accounting primitive sufficient for buy/sell/trade.
- [ ] Item cards and inventory UX emphasizing real button verbs and recent/significant items.

**Exit condition:** acquiring a tool/object changes what a character can do/understand, and meaningful objects retain identity through ownership change.

## Phase 3 — actions and encounters

- [ ] Contextual affordance discovery.
- [ ] Idempotent action execution boundary.
- [ ] Persistent multi-step encounter state with explicit scope/revision/timing.
- [ ] Transactional contention for canonical shared targets.
- [ ] Semantic named RNG derivation with independent presentation streams.
- [ ] At least three materially different non-menu interactions proving the kernel is not fishing-specific.
- [ ] Exploration/opportunity state that prevents infinite identical fresh lottery pulls.

**Exit condition:** the same kernel supports several distinct world verbs and at least one personal, one cooperative/party-capable and one shared-object interaction model.

## Phase 4 — inhabited world

Do not start this phase until the open NPC/dialogue/knowledge design in [`60-npcs-dialogue-knowledge.md`](60-npcs-dialogue-knowledge.md) is sufficiently resolved.

- [ ] NPC canonical state/presence model.
- [ ] Service availability separated from named NPC availability.
- [ ] Dialogue representation and Telegram renderer.
- [ ] Character knowledge representation and knowledge-gated presentation/actions.
- [ ] Relationship/reputation subset required by the first region.
- [ ] Task/quest/objective model with alternate solutions where the world supports them.
- [ ] Mara implemented as one inhabitant, not tutorial infrastructure.
- [ ] Several additional named/lightweight NPCs.

**Exit condition:** the region feels inhabited even when Mara is absent and an NPC interaction can alter knowledge/opportunity without relying on a hardcoded linear tutorial quest.

## Phase 5 — Telegram MMO layer

- [ ] Character profile/presence surfaces.
- [ ] Contextual player traces from real domain events.
- [ ] Direct gift/trade with atomic ownership/currency transfer.
- [ ] Party formation/join/leave plus at least one cooperative objective.
- [ ] Inline share surfaces for at least item/profile/party or similarly representative objects.
- [ ] Preserve acquisition context across shared-card → DM → character entry where platform behavior supports it.
- [ ] Run targeted inline/multi-user platform probes and record durable findings in Telegram docs.
- [ ] Notification preferences/events sufficient for dynamic world changes without violating `Off` semantics if that policy is locked.

**Exit condition:** a non-player can discover the game from another Telegram conversation, enter it, and participate in a canonical social interaction rather than merely receiving a marketing link.

## Phase 6 — starter-region content and MVP polish

- [ ] Finalize the first entry point and compact starter-region route graph.
- [ ] Populate roughly a handful of deep locations rather than many shallow destinations.
- [ ] Populate enough NPCs/services/world interactions for different first-session paths.
- [ ] Implement canonical time/weather plus the local conditions actually used by encounters/routes.
- [ ] Implement multiple outcome categories: ordinary items/junk, useful tools/materials, knowledge, traces, hazards and at least one larger mystery hook.
- [ ] Make fishing, if retained in the slice, one ordinary capability-driven encounter source rather than the navigation/progression spine.
- [ ] Validate first-session UX under materially different canonical conditions (day/night, weather, NPC availability).
- [ ] Validate newcomer experience when arriving from direct `/start` and from at least one social acquisition context.
- [ ] Validate inventory/junk ergonomics and item significance communication.
- [ ] Validate another-player inspection/trade/party flow with multiple real Telegram users.

## Cross-cutting definition of done

For every phase:

- [ ] relevant source tests/diagnostics pass or a failing checkpoint is explicitly captured;
- [ ] current executable behavior is reflected in `current-state.md`;
- [ ] settled target decisions migrate from this work area into durable subsystem docs;
- [ ] obsolete durable fishing-era statements are removed rather than retained as compatibility history;
- [ ] platform facts discovered through R&D include evidence/date in the existing Telegram documentation style;
- [ ] no new abstraction exists solely because the hypothetical infinite game might someday need it.

## Final cleanup

- [ ] All unfinished items above are either completed or deliberately moved to durable open questions.
- [ ] `game/docs/README.md` no longer indexes this temporary work area.
- [ ] This entire `game/docs/work/mvp-reset/` directory is deleted.
- [ ] `tools/check-docs.py` passes after deletion.
