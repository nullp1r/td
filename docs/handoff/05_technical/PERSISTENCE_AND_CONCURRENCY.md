# Persistence, Concurrency, and History

> **Status:** settled technical rules

## SQLite ownership

The MVP architecture uses SQLite with one controlled application access path/worker rather than introducing an ORM or connection-pool abstraction without a demonstrated need.

## Transactions

Meaningful mutations should be transactionally atomic:

- consume bait + create encounter/timer;
- claim objective reward;
- contract specimen consumption + reward;
- crafting specimen consumption + produced bait;
- group once-per-cycle participation + catch creation;
- catch recording + discovery/progression history.

## Provenance

Catch history is append/persistent truth even when the physical usable item leaves inventory.

Historical removal reasons evolved to distinguish cases such as:

- sale;
- contract;
- crafting/material use.

This enables records and future trading/trophy/external-asset systems.

## Global history

Global discoveries/world firsts are recorded transactionally rather than inferred from Telegram history.

## Group uniqueness

The v4 group shoal used a uniqueness boundary equivalent to `(chat_id, cycle, character_id)`, making “one participation per player per shoal” authoritative even though the public button is shared by everyone.

## Scaling

For the expected early scale, correctness/compactness of the SQLite state machine matters more than prematurely adding distributed infrastructure.

When scale eventually demands a different storage/partitioning strategy, preserve the same application invariants rather than leaking Telegram transport state into the model.
