# Social and MMO design

## Social play is not a bolt-on

Rustwater should be better because other people inhabit it. “Everyone individually DMs the bot” is not sufficient MMO design.

Telegram provides several useful scopes:

- the public conversation;
- shared chat-local state;
- private per-user ephemeral interaction inside that shared context;
- persistent personal DM play;
- classic inline sharing into arbitrary chats;
- Guest Mode invocation in chats where the bot is not a member;
- future game-level locations/factions/global state independent of Telegram chat boundaries.

Use each scope intentionally.

## Public state vs private result

The preferred Telegram-native pattern is:

**one public shared state + private per-player interpretation/result**.

Public history should contain things worth the group's attention: the event, collective progress, a world first, a major threshold or rare shared consequence. Routine individual bookkeeping should not emit one public message per participant.

This is both a social-design choice and a rate-limit constraint.

## Current shared shoal

Implemented group `/fish` is a clue-reading activity, not a free loot button:

1. a public card exposes the chat's current 20-minute shoal and an environmental clue;
2. **Read the water** opens a per-user ephemeral layer;
3. the player privately chooses **Let it drift** or **Hold steady**;
4. each social species has a learnable correct read;
5. a correct read awards the stronger of two deterministic specimen rolls; a wrong read can still land the weaker specimen;
6. the resulting catch is a normal persistent Rustwater specimen with discovery, XP, records and inline-sharing eligibility.

The current shoal species are Echo Herring and Rumor Carp. Group activity does not consume/wear the player's private rod, bait or coins; catch provenance uses the group's Old Harbor/basic-bait context so there is still one authoritative catch ledger.

One claim per `(chat_id, cycle, character_id)` is enforced transactionally in SQLite. Public card edits remain sparse; personal reads/results are ephemeral with toast fallback when the richer surface fails.

## Chat familiarity progression

A chat's standing is derived from its historical `group_event_claims`; there is no separate arbitrary social-XP currency.

| Historical catches in chat | Standing | What the chat learns |
|---:|---|---|
| 0–4 | Ripple | raw clue only |
| 5–14 | Current | collective-memory hint |
| 15–39 | Tidebound | familiar shoal is identified before the read |
| 40+ | Harbor Chorus | mastered approach is explicitly remembered |

This implements the wider Rustwater progression principle **knowledge becomes capability**. Other players matter because repeated activity changes what the shared context knows, not merely because a counter increases.

The lightweight personal title **Shoalbound** still unlocks after participating in a group shoal; it is identity/recognition, not the chat progression itself.

## Classic inline sharing

Classic inline mode is implemented for catches. The player's recent catches are available through `@botusername`, species-name text filters the recent set, and `catch:<item_id>` addresses one exact owned historical catch for one-tap **Share this catch** actions.

Sharing reads immutable catch/provenance history rather than current inventory state. A catch remains shareable after sale, contract turn-in or crafting consumption because those operations remove usability/ownership state without deleting the catch ledger row.

Inline mode still requires the deployed bot to have inline mode enabled via BotFather `/setinline`.

## Future social mechanics

Possible directions include contests/record races, cooperative exceptional creatures, collective choices, communal infrastructure, faction/location projects, group expeditions, environmental traces left by players, seasonal/global events and richer shareable art/trophies.

Player interaction through the environment is particularly valuable: players do not always need direct PvP or synchronized party combat to feel one another's presence.

## Chat scope is not world scope

A Telegram group is a useful social context but should not automatically become the canonical owner of a physical game location. Long-term state may exist at player, chat, world-location, region, faction and global levels.

Keep those identities distinct so one group's activity does not accidentally define reality for every player.
