# Social and MMO design

## Social play is not a bolt-on

Rustwater should be better because other people inhabit it. “Everyone individually DMs the bot” is not sufficient MMO design.

Telegram provides several useful social scopes:

- the public conversation;
- a shared chat-local event/state;
- private per-user ephemeral interaction inside that shared context;
- persistent personal DM play;
- classic inline sharing into arbitrary chats;
- Guest Mode invocation in chats where the bot is not a member;
- future game-level locations/factions/global state independent of Telegram chat boundaries.

Use each scope intentionally.

## Public state vs private result

A strong Telegram-native pattern is:

**one public shared state + private per-player interaction/result**.

Public history should contain things worth the group's attention: the event itself, a world first, a major milestone, a rare shared consequence. Routine individual bookkeeping should not emit one public message per participant.

This is simultaneously a social-design choice and a rate-limit constraint.

## Current shared shoal

Implemented group `/fish`:

- each Telegram group has a deterministic 20-minute shoal cycle;
- the cycle currently selects Echo Herring or Rumor Carp;
- each character can claim once per `(chat_id, cycle)`;
- the claim, catch creation, discovery/XP and participant state are transactionally authoritative;
- the shared cast does **not** consume or wear the player's private rod/bait/coins;
- catch provenance uses the group event's current basic Old Harbor context;
- individual result is a real persistent specimen;
- a public Rich Message contains **Cast once** and **How it works**;
- routine result is an ephemeral per-user Rich Message; toast is a fallback;
- public participant count edits occur sparsely at meaningful/logarithmic thresholds rather than every cast;
- native relative time lets the Telegram client keep the rotation label fresh without bot edit spam.

The social-only species deliberately reinforce the idea that some discoveries require other people or a conversational context.

## Future social mechanics

Possible directions include:

- chat-local encounters or environmental phenomena;
- fishing contests and record races;
- cooperative exceptional creatures/boss-like encounters;
- votes and collective choices;
- shared resource contributions;
- communal repairs/building/infrastructure;
- faction/location projects;
- reaction races for rare public opportunities;
- group expeditions;
- environmental traces/resources left for other players;
- per-chat, regional and global discoveries;
- seasonal/global events;
- socially meaningful rankings/records;
- sharing unlocked art, postcards, records and trophies through inline mode.

Player interaction through the environment is particularly valuable: players do not always need direct PvP or synchronized party combat to feel one another's presence.

## Chat scope is not world scope

A Telegram group is a useful social context but should not automatically become the canonical owner of a physical game location. Long-term state may exist at player, chat, world-location, region, faction and global levels.

Keep those identities distinct so one group's activity does not accidentally define reality for every player.

## Social identity

Titles already demonstrate lightweight public identity. Future identity may include:

- professions/specializations;
- reputations/factions;
- notable discoveries/world firsts;
- relationship/lore achievements;
- distinctive equipment/collections;
- profiles and shareable gallery pieces;
- guild/community identity if a meaningful game concept emerges.

Avoid turning every personal metric into a leaderboard. Competition should exist where it creates a social story.
