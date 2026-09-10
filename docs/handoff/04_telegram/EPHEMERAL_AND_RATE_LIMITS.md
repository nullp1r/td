# Ephemeral Interaction and Telegram Rate Limits

> **Status:** settled architecture/UX direction

## Why ephemeral matters

Ephemeral group messages solve a central tension:

- the player wants to remain socially present in the group;
- personalized results should not spam everyone;
- DMs break the social context;
- a toast is too limited for rich RPG feedback.

Use ephemeral messages for personalized group results and contextual private views where available. Current Telegram also supports ephemeral bot commands: use them for group actions such as `/help` when the command/response should be private to the invoking player.

## Lost `tdx` helper

The UX pass originally added a focused callback-query helper conceptually like:

```rust
send::ephemeral(update, rich_message)
```

The restored implementation keeps that callback helper and now also has a focused ephemeral-reply helper for the demonstrated ephemeral-command use case. This still avoids a large speculative ephemeral-message framework.

Current TDLib's `sendEphemeralMessage` requires chat, receiver, callback query, replace behavior, reply/protection/sending/preview/markup/content fields; `tdx` should provide ergonomic defaults while keeping the generated request accessible when advanced behavior is needed.

## Group public update budget

For an active shared event:

- one public card;
- private ephemeral response per participant;
- edit public card sparsely;
- persistent public messages only for meaningful rare events.

The prototype used powers-of-two participation milestones (`1,2,4,8,16...`) as a simple logarithmic coalescing rule. That is a good starting point, not a law.

## Timing/latency

Telegram presentation latency must not be counted as player reaction time. Actionable deadlines start after successful presentation.

## Failure fallback

If ephemeral sending fails:

- acknowledge the callback;
- use a concise callback toast/alert if possible;
- log the TDLib error at the application policy layer;
- do not corrupt/rollback a catch that was already transactionally committed unless the application operation itself failed.

The exact commit/presentation sequencing should preserve the game's established authoritative-state semantics.
