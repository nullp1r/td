# Glossary

> **Status:** living reference

**Rustwater** — current MVP starting region/world-thread name. Not confirmed as final game title.

**MVP-0** — earliest serious vertical slice: personal persistent fishing + world/discovery/economy proving ground.

**Shoal** — current prototype group-chat fishing event that rotates per chat/cycle.

**Ephemeral message** — Telegram group message visible only to a specific user and the bot; key to private group results.

**Rich Message** — Telegram structured message format introduced in 2026, supporting rich blocks, tables, media, formulas, and later in-message button rows.

**In-message button** — button embedded inside Rich Message content (`inlineButton` / `inputPageBlockButtonRow` in current TDLib schema), distinct from conventional reply markup below a message.

**Provenance** — persistent historical identity/context of an individual catch even after it leaves usable inventory.

**Capability progression** — unlocking qualitatively new actions/environments/systems, as opposed to only larger numeric stats.

**Explicit discovery** — game formally records/notifies a newly discovered species/location/etc.

**Implicit discovery** — player learns a hidden relationship through observation without a numeric reveal.

**World first** — first global discovery/catch event persisted in authoritative game history.

**Encounter step** — compact optimistic/idempotency value used to reject stale/double Telegram callbacks for an active encounter.

**Semantic Back** — navigation to a screen's conceptual parent, not a global browser-history stack.

**`tdx`** — user's Rust convenience/application layer over generated TDLib APIs; intentionally evolving.
