# Future AI Developer Bootstrap

> **Status:** operational guidance

If you are an AI coding agent continuing this project:

1. Read repository root `AGENTS.md` first. Its principles are intentional and strict.
2. Read `README.md`, `00_handoff/CURRENT_STATE.md`, `00_handoff/USER_PREFERENCES.md`, and `01_product/DECISIONS.md`.
3. Do not infer the exact code state from these documents; inspect the checkout.
4. Run `05_technical/DIAGNOSTICS.md` before large changes and again after integration.
5. Do not re-ask broad design questions already settled here unless new evidence makes the choice consequential.
6. When mechanics are genuinely uncertain and cheap to change, implement a coherent prototype and let playtesting answer them.
7. Do not add generic internal frameworks without a demonstrated consumer.
8. Do not treat `tdx` as immutable. If Telegram supports a valuable capability and `tdx` does not expose it ergonomically, improve `tdx`.
9. Do not use technical/MVP/developer language in ordinary player UI.
10. A successful iteration is one the user can actually playtest, not only one that compiles.

## Particularly important regression traps

Do not regress to any of these:

- Cast/Catch-only navigation loop.
- Huge private command catalog acting as a second menu.
- `/help` written like developer documentation.
- Group cards full of unexplained telemetry.
- Group gameplay that only says “open the bot in DM.”
- Strict reaction windows where Telegram latency determines valuable outcomes.
- catastrophic equipment loss from ordinary mistakes/lag.
- overengineering uncertain future systems before there is a player need.
- refusing a product idea because `tdx` lacks a convenience helper.

## When making UI decisions

Ask:

- What is the player trying to do **right now**?
- Is the primary action visually obvious?
- Is this information useful to the player or only interesting to the developer?
- Can Rich Message structure communicate hierarchy better than plain prose?
- Should this interaction be public, private-ephemeral within a group, or DM-only?
- Can it be accomplished without unnecessary Telegram message traffic?
