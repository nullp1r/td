# Group Chat UX Target

> **Status:** current target; core design implemented in lost UX tree but not packaged

## Why the old group UX failed

The old `/game` card showed a “world pulse” with terms/stats the group had no context for. Testers stared at the button instead of clicking because they did not know:

- what Rustwater was;
- whether the action would spend their inventory/coins;
- whether it would affect everyone;
- what the mysterious telemetry meant;
- whether `/help` would explain anything different.

This is a product failure, not a player problem.

## Commands

Target group commands:

- `/fish` — start/show the current shared fishing activity;
- `/help` — explain group fishing in player language; register it as an ephemeral command so the request and response can stay private to the asking player inside the group.

`/game` is too generic and should stay removed unless a future broader group hub earns that name.

## Group `/help`

It should explain:

- Rustwater is the starting world/region;
- the bot is a persistent RPG/fishing game;
- `/fish` creates a shared shoal event for this chat;
- everyone gets one cast per shoal/cycle;
- each player's result is private to them inside the group;
- the caught specimen belongs to their normal persistent character;
- pressing the shared button does not secretly spend their coins/change gear/use inventory in the current group-shoal design.

Do not show implementation details.

## Shared `/fish` card

Target:

```text
# 🎣 Fishing in this chat

This shared shoal features **Echo Herring**.
Everyone gets one free cast during its window.

*Your rod, bait, and coins are untouched by this shared cast.*

🐟 Shoal     Echo Herring
👥 Casts     3 this shoal
⏳ Rotation  in 12 minutes

[ 🎣 Cast once ]
```

Only show state that helps a player understand the activity. The rotation value should use Telegram's native relative date/time entity so the client updates it while the message is visible; do not run a periodic bot edit loop merely to refresh a countdown. Keep the surrounding wording valid if an old card is viewed after the rotation boundary.

## Private result inside the group

Pressing the public Rich Message button should create a private ephemeral result for that user when supported:

```text
# 🎉 Your catch

**Echo Herring · 1.24 kg · 31.8 cm**

✨ XP      +12
⭐ Level   3
📖 New species — added to your Journal.

*Only you can see this result.*
```

The specimen remains a real persistent catch. The result can expose read-only personal surfaces such as Journal/Records ephemerally so the player can stay in the group context.

Routine participation should therefore create **zero additional public messages**.

## Public update policy

Keep the public event card stable and edit it sparsely, for example when participation reaches powers of two (`1, 2, 4, 8, 16...`) or for genuinely noteworthy/world-first outcomes.

This keeps the event alive without fighting Telegram group rate limits.

## Fallback

If ephemeral delivery fails, a callback toast is an acceptable graceful fallback. It is not the desired primary social UX.

## Future group mechanics

The same public + ephemeral split can support:

- reaction races;
- cooperative boss/monster/fish encounters;
- votes/choices;
- chat-local environmental discoveries;
- competitive leaderboards;
- shared resource contributions;
- hidden personal outcomes from a public event.

## Help discoverability

Do not make group onboarding depend on the slash-command menu. The shared shoal card carries a **How it works** Rich Message button that opens the same per-user ephemeral guide as `/help`. Commands remain useful entry points, but the visible activity interface must stand on its own across Telegram clients.
