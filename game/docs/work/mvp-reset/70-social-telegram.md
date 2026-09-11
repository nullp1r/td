# Social and Telegram-native MMO surfaces

## Locked social model

Keep these concepts distinct:

- **Telegram chat** — accidental/external social context controlled outside the game;
- **party** — temporary consensual gameplay group;
- **guild** — persistent consensual in-game organization.

A chat is a window onto the canonical world, not a shard and not an automatic guild. Old chat-level Ripple/Current/Tidebound progression is rejected.

Telegram group chats are a critical MVP acquisition surface. A person should be able to encounter an interesting canonical object/profile/event in an unrelated chat, interact with it, and enter the game with that acquisition context preserved.

## Locked inline direction

Inline mode is first-class and should eventually support more than catch sharing. Candidate canonical shareables include:

- items/unique objects;
- character profiles/records;
- party/expedition recruitment;
- discoveries/world events;
- trade offers;
- locations/knowledge where privacy/state allows.

A shared Telegram card is a presentation/reference to canonical game state, not the authoritative item itself.

Inline sharing is not only static broadcasting: where Telegram semantics allow it, cards should be able to launch/join/inspect real interactions such as party recruitment, object inspection or trade.

A message is also historical presentation. If the underlying canonical object later changes owner/state, an old card need not retroactively rewrite every rendered line, but interacting with it must resolve current canonical truth and must never duplicate stale ownership/rewards.

## Locked: settings have explicit scope

Per-player/account preferences and per-chat/group preferences are both required concepts. Do not let a random Telegram group inherit a character's private settings or vice versa.

Group-facing notification/activity policy may also require admin/permission semantics because Telegram chat membership and administration are external to the game.

## Rejected group mechanic

The current “one shared shoal/fish per chat cooldown” mechanic does not represent the target MMO. Do not preserve it during the reset merely because it is implemented.

Group interactions should center on sharing, coordination, offers, consequences and spontaneous world events rather than forcing every chat into a fishing activity loop.

## Leaning: social presence through contextual traces

Telegram does not render an MMO crowd, so presence should appear through several forms:

- nearby/recent characters;
- discoveries with discoverers;
- records with holders;
- ownership/provenance on objects;
- trades/offers;
- party recruitment;
- meaningful recent actions reflected in local state.

Avoid fake online dots. “Here recently” or a domain-event-derived consequence often fits asynchronous Telegram better.

## Leaning: profiles are in-world character surfaces

Public world presence should primarily use character identity, with Telegram identity shown only where policy/UX requires it. Profiles should eventually expose enough identity, location/presence, records, equipment/achievements and shareable state to make other players feel real without becoming a privacy leak.

Exact public fields/privacy controls remain open.

## Leaning: trade/party cards are interactive social objects

Trade and party recruitment should be possible through Telegram-native rich cards. Canonical acceptance still validates current world/ownership/eligibility state at interaction time; old messages must not create stale duplicated rewards or ownership.

## Open platform R&D

Before locking some social UX, run empirical probes for:

- callbacks on inline-created messages in arbitrary chats when the bot is not a member;
- reliable identity of the interacting user/character;
- private/ephemeral response options from shared messages;
- editing/updating inline shared messages after canonical state changes;
- multi-participant callback behavior and contention;
- deep-link/DM handoff that preserves acquisition context.

Do not infer platform guarantees from conventional bot behavior when Rich Message/inline behavior can be tested directly.

## Open notifications

World conditions/events should be able to generate proactive notifications when useful.

There is an unresolved policy tension from the reset discussion:

- one idea was default-on alerts with opt-out and occasional re-engagement reminders even after opt-out;
- the counterproposal is conventional explicit levels such as `On`, `Important only`, `Off`, with `Off` genuinely silent and missed activity shown only after voluntary return.

The user did not explicitly settle that disagreement. Do not silently encode “remind after Off” as a requirement, and do not call strict silence permanently locked yet.

Group notifications may require admin controls, rate/frequency policy and quiet-hours behavior. This is also open.
