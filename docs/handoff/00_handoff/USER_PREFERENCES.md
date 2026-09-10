# User Preferences and Project Taste

> **Status:** high-value context; treat as durable unless the user explicitly changes it.

## Product taste

The strongest game references are **Terraria** and **World of Warcraft**. Skyrim, Fallout: New Vegas, League of Legends talent/trait systems, and Minecraft have also been used as comparison points. Cookie Clicker is useful mainly as an example of absurd/infinite depth, **not** as the primary gameplay model.

The user likes games where:

- there is always something else to discover;
- progression unlocks qualitatively new capabilities;
- collecting/knowledge can become a deep hobby;
- systems overlap in surprising ways;
- failure matters but does not casually erase enormous investment;
- different people can specialize and care about different parts of the world.

The user explicitly dislikes Minecraft-style catastrophic loss where perfect gear can disappear because of lava, server lag, or one stupid mistake. Terraria/WoW are closer to the desired frustration/loss balance.

## Scope philosophy

The user wants an **infinite game**, not a fishing game, but is very aware of scope creep. Fishing is the starting activity because it is already a strong vertical slice and can demonstrate:

- world conditions;
- items;
- equipment;
- procedural specimens;
- discovery;
- economy;
- social interaction;
- timing;
- long-term collection;
- capability progression.

Do not solve the entire future game before the MVP is fun.

## Playtime philosophy

The same persistent world should be interesting at radically different engagement lengths:

- ~10 seconds: one meaningful action;
- minutes: a fishing session or check-in;
- ~1 hour: a real progression step;
- a day: pursuit of a larger goal;
- a week/month/year: collection, social, specialization, world progression.

Short sessions are not a separate idle mode.

## Design uncertainty preference

The user is comfortable saying “not sure yet.” That is intentional, not missing documentation.

Known deliberately-open areas include:

- how far automation should go;
- exact stats model;
- exact talent/profession/specialization structure;
- whether multiple characters eventually matter;
- exact late-game traversal/navigation mechanics;
- detailed monetization/NFT integration.

When a mechanic is hard to judge without playing it, build the smallest coherent version and playtest rather than inventing an elaborate theory.

## UX taste

This became one of the strongest preferences in the session:

- Treat every user equally, including the developer. Developer status must not leak into normal player UI.
- `/help` must sound like game onboarding, not a release note, design spec, or technical explanation.
- Use Rich Message typography/structure confidently.
- Use emojis **sparingly but visibly and tastefully**. Pure walls of unaccented text look bland.
- Primary actions should be visually obvious.
- Telegram-native features should be used aggressively when they improve the game.
- Navigation should be obvious and ergonomic even while the game remains deep.
- Commands should earn their place as Telegram-level entry points, not duplicate every UI screen.
- Group chats should feel socially alive. “Go to the DM” is not a sufficient social design.

The user specifically called out the earlier `/game` group panel as intimidating and mysterious because it led with terms/stats such as “Rustwater,” world telemetry, species counts, etc. without first explaining what the player was supposed to do.

## Technical taste

- `tdx` is the user's own Rust TDLib wrapper and can be changed freely.
- Any `td`-family crate can be changed if that produces the cleanest result.
- `tdx` must never become a reason to give up on a Telegram feature.
- SQLite is the database choice.
- Lean crates are generally fine to add without asking.
- For non-lean/heavy dependencies, ask first.
- Industry-standard crates such as Tokio/tracing are acceptable when they materially improve DX/UX/correctness.
- Follow root `AGENTS.md` above all local convenience.
- The project deliberately tracks bleeding-edge Rust. The earlier pinned MSRV was 1.98.1, and the real diagnostics later ran on `1.100.0-nightly`.
- New/stabilizing Rust features are welcome when they produce concrete clarity/correctness/efficiency benefit.

## Communication preference for design work

Early in the session the user answered many broad questions. After that point they explicitly asked the assistant to **pick the best answers** where enough context already existed.

Future design work should therefore avoid repeatedly reopening settled broad questions. Present concrete options only when the choice has real consequences or playtesting cannot resolve it cheaply.
