# Original UX Request and Precision Audit — 2026-09-09

> **Status:** canonical UX source + post-restoration audit
>
> Preserve this file verbatim enough that future handoffs do not reduce the user's product taste to a checklist and lose the reasoning behind it.

## Original user message

> there's one thing that bother me the most: UI and UX
>
> biggest issue is that im already showing this prototype to people and e.g. they get confused by that big message in /help, it seems the message is meant for me, the developer, not so much for the actual players, everyone should be treated equally already, even though im a developer, im a also a player and a user of the product, even though it's barely an MVP, but we should do all things right from the start
>
> also i try to use /game in group chats and all people just staring at it and afraid to even click the button because it's just some very odd message, like what is this and what is happening? rustwater? what is rustwater? what are those mysterious stats? even help command doesn't work in group chats, /help and /game doing the same mysterious thing
>
> then there's UI problem: yes, you used some rich styling but barely, spent a lot of hours designing and implementing tdx, it supports the entire rich formatting, you seem to be afraid to touch it, most of information is presented as plain text, no highlights, no accents, also complete lack of emojis, they're only in the buttom menu
>
> emojis should be used sparingly and with taste but they SHOULD be used, walls of text without any icons looks really bland and boring, like reading a book without pictures
>
> also you promised me in-message buttons, the new telegram feature, what happened to it? turned out to be impossible or unsupported? tbh not sure if tdx supports it, if it doesn't then just add the missing bits, add any missing bits, tdx should not be a reason you give up on some design or features, tdx is an evolving project, productive changes are welcome
>
> also navigation sometimes feels a bit unintuitive, home button is nice but perhaps we could also have back button, back and home, almost on all screens, maybe even some contextual 3rd button like in android prones but i can't think of one right now, just giving you some food for thoughts
>
> also most bot commands feel unnecessary, the entire game should be playable though the main interface with markup navigation (or in-message buttons, and maybe some other means), all other commands should earn their place serve some purpose, e.g. in group chats having /help and /game makes total sense, maybe also /ratings and stuff like that, btw /game is too generic name, but yeah commands is a pain point currently, they need an overhaul
>
> also you promised me some per-user messages in chats, is this still possible or what? the most stuff available in social setting - the better, dms are boring, but as we discussed earlier, everything should be designed with telegram rate limits and other constraints in minds, but ALWAYS keep an eye of all available telegram features we could use for our advantage, especially the recent features, telegram is always adding some nice UI/UX things
>
> ok i think those would be my thoughts for the next iteration, im going for a walk now, i hope i come back and see the next zip file ready to playtest, but take your time, im not in a hurry

## Precision audit after restoration

### 1. Player UI is for players, including the developer

**Implemented:** `/help` is concise onboarding, normal screens avoid MVP/build/schema/debug commentary, and player-facing application errors no longer mention loaded content or internal implementation state.

**Rule:** developer/admin/diagnostic information must live outside ordinary gameplay surfaces. The developer gets the same product UI as everyone else.

### 2. Group entry must explain itself before asking for trust

**Implemented:** `/game` stays removed. `/fish` leads with **Fishing in this chat**, names the shoal, states that everyone gets one free cast, shows only useful timing/participation state, and now reassures players directly on the public card that their rod, bait, and coins are untouched.

`/help` in groups explains what Rustwater is, what persists, what `/fish` does, what is private/public, and what a shared cast can and cannot change. The `/fish` card also exposes **How it works** directly, so help does not depend on command-menu discoverability.

### 3. Rich presentation is an interface primitive, not decoration

**Implemented:** player surfaces use Rich Message headings, bold/italic emphasis, block quotes, compact/striped tables, semantic button styles, and visible but restrained emoji anchors. Primary contextual actions live in Rich Message button rows where that improves hierarchy.

**Rule:** do not mechanically use every Rich Message primitive. Use the richer vocabulary confidently when it improves scanability, hierarchy, meaning, or interaction. Avoid both plain-text walls and decorative noise.

### 4. `tdx` must evolve with Telegram

**Implemented:** `tdx` gained Rich Message callback rows/styles, conventional styled callbacks, callback-triggered ephemeral sends, and now ephemeral-command reply ergonomics. `td/fetch` validates the schema surface required by the game before installing a new TDLib/schema pair.

**Rule:** if Telegram supports a useful product feature and `tdx` does not, improve `tdx`; never downgrade the product design merely to avoid touching the wrapper.

### 5. Navigation should be obvious: semantic parent + Home + context

**Implemented:** deeper stable panels use a semantic parent plus Home; top-level panels use Home plus useful sibling/context destinations; modal fishing states avoid a fake Back action that could invalidate or obscure the active encounter. Locations now exposes Conditions as a contextual destination, and misleading arrow labels are avoided when a destination is merely contextual rather than a parent.

**Why not a browser-history Back stack:** the current UI edits one Telegram message in place. A true history stack would require per-user navigation state, stale-history semantics, restart behavior, and invalidation. No playtest has yet demonstrated that this machinery is better than explicit semantic parents. Add it if real users still get lost.

### 6. Commands must earn their place

**Implemented:** private registration/routing is `/start` + `/help`. Group registration is `/fish` + `/help`; `/game` remains removed because the current group feature is specifically fishing and the generic name did not explain itself. Placeholder commands such as `/ratings` are not added until a real socially useful ratings surface exists.

The whole private game remains reachable through the native interface.

### 7. Keep personalized play inside the social context when possible

**Implemented:** group casts produce per-user Rich Message results inside the group rather than redirecting to DMs. The result is a real persistent catch. During this precision audit the social surface was extended with private **Journal** and **Records** actions from the ephemeral catch result, so a player can inspect personal progress without leaving the group.

Group `/help` now uses Telegram's ephemeral-command path: the help interaction stays private to the requesting player inside the group instead of adding public help-message clutter. The same private guide is available from an in-message **How it works** button on `/fish`, so the command is an alternate entry point rather than a UI dependency.

**Direction:** continue applying the public-shared-state + private-ephemeral-detail pattern to future cooperative encounters, choices, leaderboards, discoveries, contributions, and other social mechanics where it fits.

### 8. Rate limits and current Telegram capabilities are product constraints

**Implemented:** routine group catches create no new public messages; the shared shoal card only edits at sparse powers-of-two participation milestones or a world first; personalized views are user-triggered ephemerals; ephemeral failures fall back to callback toasts without rolling back already committed game state.

**Rule:** re-check Telegram/TDLib capabilities when working on interaction design. New platform primitives are potential game-design tools, not merely API trivia.

## What this audit changed beyond the first restoration

- preserved the exact original UX request in the repository;
- added explicit no-cost/no-gear-change reassurance to the public `/fish` card;
- made group `/help` an ephemeral command with an ephemeral Rich Message reply, while also exposing the same guide through an in-message **How it works** action;
- added ephemeral Journal/Records access from a group catch result;
- added `tdx::command::ephemeral_definition` and `send::ephemeral_reply` helpers plus tests;
- hardened `td/fetch`/diagnostics for `botCommand.is_ephemeral`;
- removed remaining developer-ish wording from player errors and progression/crafting copy;
- added a few missing visual anchors without turning every line into an emoji list;
- tightened navigation/context labels and added Conditions beside Locations.

## Remaining playtest question, not an implementation omission

A true history-based **Back** stack is intentionally not implemented yet. The user's original wording was exploratory (“perhaps”, “maybe”), and the current semantic-parent model solves the demonstrated cases with far less hidden state. If no-explanation testers still expect browser/Android-style history, that becomes demonstrated evidence to add it.
