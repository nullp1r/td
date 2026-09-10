# Scaling Strategy

> **Status:** settled principles, future implementation open

## “Scalable” does not mean distributed now

The user's long-term vision is enormous, but the MVP should remain a compact system that preserves correct boundaries.

Scale first through:

- data-driven content;
- efficient compact read-only tables;
- transactional persistence;
- deterministic generation;
- clear IDs/provenance;
- idempotent operations;
- Telegram message coalescing;
- separation of public/private group presentation.

## Content scaling

Species/items/locations can grow substantially if startup validation and runtime lookup remain predictable. Specialized data structures are welcome when they clearly beat generic maps/object graphs.

## Social scaling

Telegram itself imposes visible message-rate constraints before application compute becomes the only issue.

Use:

- one public shared panel per event;
- logarithmic/coalesced edits;
- ephemeral personalized outputs;
- no public message per routine participant;
- persistent announcements only for noteworthy events.

## Storage scaling

SQLite is appropriate for MVP/testing and a meaningful amount of real use when access patterns are disciplined.

Do not pre-build a distributed persistence framework. If actual load demonstrates a storage bottleneck, migration should preserve the current application invariants and historical data model.

## Multi-surface scaling

DM, group, Mini App, and web should consume the same world/application operations. Avoid encoding gameplay rules in one presenter/client.
