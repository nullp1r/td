# Commands and Navigation

> **Status:** current target

## Command philosophy

Slash commands are Telegram-level entry points, shortcuts, or public chat actions. They should not be a duplicate sitemap.

### Private

Advertise approximately:

- `/start` — open/create character and enter the main interface;
- `/help` — player onboarding/explanation.

Other old commands (`/inventory`, `/journal`, `/shop`, `/tasks`, `/talk`, `/conditions`, `/locations`, etc.) should be removed from registration/routing as normal navigation once the interface can reach those screens reliably.

Debug/admin commands, if needed, must be isolated from ordinary players.

### Group

Advertise:

- `/fish` — shared chat activity;
- `/help` — group-specific explanation; prefer an ephemeral group command/response so help does not add public clutter.

Potential later commands such as `/ratings` can exist if invoking them as a command is socially useful.

## Navigation hierarchy

### Primary contextual action

Use in-message Rich Message button where appropriate.

### Secondary/contextual actions

Use nearby Rich Message buttons or first reply-markup row if they need to remain accessible independently of rich-content rendering.

### Parent + Home

Most deeper screens end with:

```text
[ ← Parent ] [ 🏠 Home ]
```

The parent is semantic, not browser history.

## Why no global Back stack yet

A global stack would require per-user UI-navigation state, stale-history behavior, restart semantics, and extra invalidation logic. There is no demonstrated need yet because each current screen has an obvious conceptual parent.

If playtests later show semantic Back is insufficient, add history then with a real consumer.
