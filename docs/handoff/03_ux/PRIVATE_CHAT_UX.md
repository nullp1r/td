# Private Chat UX Target

> **Status:** current target; substantial parts were implemented in lost UX tree

## Home

Home should immediately answer:

- Where am I?
- What are conditions like?
- What can I do now?
- What is my basic character/tackle state?

Conceptual hierarchy:

```text
# 🌊 Old Harbor

[short location/weather description]

> 🌅 Morning · 🌫 Fog

⭐ Level   3
✨ XP      184
🏷 Title   Naturalist
🪙 Coins   72
🎣 Rod     Reinforced Rod · 93%
🪱 Bait    Glow Larva ×4

[ 🎣 Cast a line ] [ 🔎 Explore ]
```

Secondary/navigation keyboard:

```text
[ 💬 Mara          ] [ 📋 Harbor Board ]
[ 🗺 Locations     ] [ 🎒 Inventory    ]
[ 📖 Journal       ] [ 🏆 Records      ]
[ 🧰 Prepare bait  ] [ 🏷 Titles       ]
[ 🏪 Tackle stall  ] [ 🌦 Conditions   ]
[ ℹ️ Help                                ]
```

The exact density can change after playtesting, but the hierarchy should remain obvious.

## Fishing wait

Do not create unnecessary history spam. The active panel can update in place.

## Bite

Target feel:

```text
# ⚡ Bite!

*The float darts sideways, pauses, then pulls steadily under.*

Watch the movement, then reel when it feels right.

[ 🎣 Reel ]

[ ✂️ Cut line ]
```

The observation is gameplay information, not flavor pasted below a generic instruction.

## Catch result

Meaningful things should visually dominate:

```text
# 📖 New species!

**Harbor Mackerel**

📏 Length   31.8 cm
⚖️ Weight   1.24 kg
⚡ Timing   Good
🪙 Value    18

✨ **+18 XP** · Level 3

[ 🎣 Cast again ]

[ ← Journal ] [ 🏠 Home ]
```

For an ordinary repeat catch, do not pretend every fish is a world-changing event; keep the same hierarchy but reduce celebratory emphasis.

## Struggle

A struggle should communicate:

- what the creature appears to be doing;
- why Pull/Give Line are meaningful choices;
- urgency without latency-hostile timers;
- clear destructive/escape action styling for cutting line.

## Exploration/relic

The clue/reward should feel like a world event, not a database mutation. Use a persistent message when the discovery is significant.

## Inventory/shop/crafting

These can remain structured/tabled, but each screen needs:

- a clear title;
- meaningful current context;
- obvious next actions;
- semantic Back/Home.

A future Mini App may eventually replace some high-density inventory/crafting views, but the native bot UI should remain fully usable.
