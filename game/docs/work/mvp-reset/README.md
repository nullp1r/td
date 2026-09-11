# MVP reset work plan

This directory is the temporary design and implementation control plane for the Rustwater MVP reset. It exists because the current source and durable docs still describe the fishing-centered vertical slice while the replacement product architecture is being designed and built.

Delete this directory when the reset is complete. Before deleting it, migrate every surviving product truth, invariant, open question, and architecture decision into the appropriate durable document under `game/docs/`.

## Authority during the reset

Three kinds of truth coexist during the reset:

1. **Source plus `current-state.md`** describe what executes now.
2. **Durable `game/docs/` documents** describe the established product and architecture outside this active reset.
3. **This directory** describes the intended reset target and its implementation sequence.

When this work plan intentionally disagrees with the current fishing implementation, do not add compatibility machinery merely to preserve the old behavior. The project explicitly allows wiping pre-MVP data, replacing migrations/schema/content IDs, and rewriting `game.json` or its successor.

Do not silently update durable docs to claim target behavior is already implemented. Migrate target decisions into durable docs when the corresponding decision is settled or implementation makes them current truth.

## Status vocabulary

- **Locked** — explicitly agreed; implementation should preserve it unless new evidence forces reopening.
- **Leaning** — strong current preference, but not yet explicitly locked.
- **Open** — intentionally unresolved; design/R&D/playtesting still required.
- **Rejected** — considered and deliberately not the target.
- **Implemented** — already present and intentionally reusable in the current codebase.

A heading or checklist item without one of these labels is organizational, not a decision.

## Fresh-session continuation

A new session should be able to continue from the repository/handoff without reconstructing the conversation that created this plan.

Read this file first, then [`00-north-star.md`](00-north-star.md) and [`10-foundations.md`](10-foundations.md), then the work package for the active design frontier. Preserve the status vocabulary above: do not silently promote a **Leaning** idea to **Locked**, and do not treat current fishing-era source/docs as target architecture when this work area explicitly supersedes them.

The reset is still in **design convergence, not implementation**. Before the starter slice is locked, the unresolved design dependencies include [`60-npcs-dialogue-knowledge.md`](60-npcs-dialogue-knowledge.md), Telegram social semantics, content authoring, and progression/failure/economy.

Important carried-forward tensions are documented rather than silently resolved. In particular:

- Mara's early first-meeting role versus canonical NPC presence is explicitly reopened in [`20-identity-onboarding.md`](20-identity-onboarding.md) / [`60-npcs-dialogue-knowledge.md`](60-npcs-dialogue-knowledge.md);
- strict notification `Off` versus occasional re-engagement reminders is unresolved in [`70-social-telegram.md`](70-social-telegram.md);
- exact arrival transport/fiction remains open even though explicit world-boundary entry is a strong direction;
- earlier broad `● / ○` and Home-dashboard habits are corrected for the reset in [`25-interface-grammar.md`](25-interface-grammar.md).

## Work packages

- [`00-north-star.md`](00-north-star.md) — product thesis, emotional target, MVP proof.
- [`10-foundations.md`](10-foundations.md) — shared-world, persistence, geography, history, content boundaries.
- [`20-identity-onboarding.md`](20-identity-onboarding.md) — accounts, characters, world entry, adaptive onboarding.
- [`25-interface-grammar.md`](25-interface-grammar.md) — reset-specific Telegram UI grammar and corrections.
- [`30-world-locations.md`](30-world-locations.md) — location/POI model, routes, presence, starter-region shape.
- [`40-items-capabilities.md`](40-items-capabilities.md) — item identity, inventory, provenance, requirements and trade.
- [`50-actions-encounters.md`](50-actions-encounters.md) — affordances, action lifecycle, scoped encounters, RNG and repetition.
- [`60-npcs-dialogue-knowledge.md`](60-npcs-dialogue-knowledge.md) — inhabited-world rules and the next design frontier.
- [`70-social-telegram.md`](70-social-telegram.md) — group acquisition, inline sharing, parties, profiles, traces and notifications.
- [`80-content-live-world.md`](80-content-live-world.md) — structured content/localization boundary and dynamic world composition.
- [`85-progression-failure-economy.md`](85-progression-failure-economy.md) — established constraints and unresolved progression/failure/economy design.
- [`90-implementation.md`](90-implementation.md) — demolition/salvage boundaries, phased rebuild, definition of done.

## Master checklist

### Design convergence

- [x] **Locked** — establish the finished-product north star: Telegram-native persistent sandbox MMO RPG; fishing is one ordinary activity.
- [x] **Locked** — establish one canonical world with explicit global/local/party/personal state scopes.
- [x] **Locked** — separate accounts from characters and keep multiple characters possible.
- [x] **Locked** — establish hybrid stackable commodities plus unique item instances with provenance.
- [x] **Locked** — establish geography as hierarchy + coordinates + explicit route graph.
- [x] **Locked** — establish real timestamps plus durable meaningful domain events without full event sourcing.
- [x] **Locked** — separate structured game definitions from localized rich text.
- [x] **Locked** — preserve Rich Message/state-event UI principles while correcting overgeneralized `● / ○` and text-link action usage.
- [x] **Locked** — treat activity gear/task objects as ordinary items and keep mundane junk/bulk ergonomics in scope.
- [x] **Locked** — retain meaningful-but-recoverable failure philosophy while leaving exact death/progression systems open.
- [ ] Resolve NPC, dialogue, knowledge, quest/task and service semantics.
- [ ] Resolve Telegram social acquisition and multi-participant interaction semantics through targeted R&D where platform behavior is uncertain.
- [ ] Resolve content authoring formats sufficiently to implement the first slice.
- [ ] Resolve structural progression, death/failure, economy and character-specialization semantics.
- [ ] Lock the first entry point and first starter-region content slice after the systemic design is stable.

### Implementation

- [ ] Replace fishing-centric schema/content assumptions instead of building compatibility layers around them.
- [ ] Build the account/character/world/location/time/event kernel.
- [ ] Build hybrid items, ownership, inventory/provenance and capability resolution.
- [ ] Build action/affordance/encounter infrastructure with semantic RNG domains and idempotent mutation boundaries.
- [ ] Build NPC state, dialogue, knowledge and task systems.
- [ ] Build player presence, profiles, trade, parties and inline/group social surfaces.
- [ ] Build the first small persistent starter region with enough systemic depth to imply a much larger world.
- [ ] Replace obsolete durable documentation as each subsystem becomes current truth.
- [ ] Delete this directory after all surviving decisions have migrated to canonical docs.

## Reset completion test

The reset is not complete because the schema was rewritten. It is complete when a real player can enter as a character, encounter a living shared location, meet inhabitants, discover multiple activities, acquire meaningful objects, understand why at least some of them matter, observe another player's presence/history, interact economically or socially, experience changing world conditions, encounter a surprising systemic outcome, share something into another Telegram conversation, and participate in a small cooperative objective.

The first slice should prefer a few deep locations that imply a large world over a large number of shallow locations.
