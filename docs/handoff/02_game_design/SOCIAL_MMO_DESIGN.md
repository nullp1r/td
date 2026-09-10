# Social and MMO Design

> **Status:** settled direction, early implementation

## Social play is not a bolt-on

The user explicitly wants the game to be more interesting in social contexts than “everyone opens a DM with the bot.” DMs are useful, but too much DM-only interaction makes the MMO feel lonely.

## Natural Telegram division

### Public group state

Good for:

- shared events;
- visible discoveries;
- group milestones;
- public competition/cooperation;
- rare announcements;
- chat-local environmental state.

### Per-user ephemeral state inside a group

Good for:

- personalized catch results;
- private decisions;
- inventories/equipment previews;
- hints;
- one-player consequences triggered by a shared event.

### DM

Good for:

- long personal sessions;
- private progression;
- inventory/crafting when no social context is useful.

## Prototype group shoal

The v4 prototype established a useful concrete model:

- deterministic per-chat shoal cycle;
- 20-minute real-time cycle in that implementation;
- social-only species such as Echo Herring/Rumor Carp;
- one participation per character per chat/cycle;
- persistent real specimen + XP/discovery/history;
- server-side transactional uniqueness;
- sparse public updates to respect rate limits.

The old UX used a public telemetry-heavy card and callback toast result. The target UX is documented in `03_ux/GROUP_CHAT_UX.md`.

## Future social directions

- fishing contests;
- reaction races for rare public catches;
- cooperative encounters;
- location/chat environmental effects;
- players leaving traces/resources/structures for one another;
- per-chat discoveries;
- region/global seasonal events;
- leaderboards/ratings only where socially meaningful.

Player interaction through the environment was specifically identified during design as an exciting direction worth exploring.
