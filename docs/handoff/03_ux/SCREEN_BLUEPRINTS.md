# Screen Blueprints

> **Status:** design/reimplementation reference

These are not immutable pixel specs. They preserve the interaction hierarchy developed in the lost UX pass.

## Help

```text
# 🌊 Welcome to Rustwater

You’re an angler working the old harbor and the waters around it.
Fish, explore, keep interesting specimens, and follow whatever the shoreline turns up.

> 🎣 **Start here:** go Home and tap “Cast a line”. When something bites, react to what you see.

🎣 **Fish** — catch specimens and learn what lives where.
🧭 **Explore** — discover places, people and clues.
🎒 **Prepare** — choose gear, sell catches and make bait.
📖 **Record** — Journal and Records track discoveries and personal bests.
📋 **Progress** — Mara's board gives milestones and changing contracts.

[ 🏠 Back to Rustwater ]
```

Footer concept:

> Commands are entry points, not the game: `/start` opens your character and `/help` brings you here. Everything else is in the interface.

## Home

See `PRIVATE_CHAT_UX.md` for the full example.

## Bite

```text
# ⚡ Bite!

*The float darts sideways, pauses, then pulls steadily under.*

Watch the movement, then reel when it feels right.

[ 🎣 Reel ]

[ ✂️ Cut line ]
```

## New catch

```text
# 📖 New species!

**Harbor Mackerel**

📏 Length  31.8 cm
⚖️ Weight  1.24 kg
⚡ Timing  Good
🪙 Value   18

✨ **+18 XP** · Level 3

[ 🎣 Cast again ]

[ ← Journal ] [ 🏠 Home ]
```

## Group shoal

```text
# 🎣 Fishing in this chat

A shoal of **Echo Herring** is passing through.
Everyone gets one free cast before it moves on.

*Your rod, bait, and coins are untouched by this shared cast.*

🐟 Shoal    Echo Herring
👥 Casts    3 this shoal
⏳ Moves in 12m 40s

[ 🎣 Cast once ] [ ℹ️ How it works ]
```

## Ephemeral group result

```text
# 🎉 Your catch

**Echo Herring · 1.24 kg · 31.8 cm**

✨ XP      +12
⭐ Level   3
📖 New species — added to your Journal.
🎒 Added to your Inventory.

*Only you can see this result.*

[ 📖 Journal ] [ 🏆 Records ]
```

Group `/help` should itself be an ephemeral command/response: explain what Rustwater is,
what carries between private and group play, and why routine personal results stay private.

## Rules for future screens

- Name the thing/activity first.
- Explain unfamiliar world nouns when first encountered.
- Make the next meaningful action obvious.
- Distinguish achievement/discovery states visually from ordinary routine results.
- Keep numbers only when they inform decisions/status.
- Prefer semantic parent + Home.
