# Lost / Unpackaged UX Implementation

> **Status:** historical implementation report; reimplementation target
>
> This is the most important document for work that was **implemented in an assistant working tree but never packaged**, so the current repository may not contain it.

## Trigger

The user reported that the prototype was already being shown to people and that the UI was the biggest problem:

- `/help` read like a developer document rather than onboarding;
- `/game` in groups was unexplained and intimidating;
- group `/help` and `/game` effectively showed the same mysterious thing;
- Rich Message formatting existed but was barely used;
- emojis were almost absent outside button menus;
- promised in-message buttons had not appeared;
- navigation needed Back as well as Home;
- the private slash-command catalog duplicated the whole UI;
- group interaction still felt too DM-centric;
- Telegram's newest capabilities were not being exploited.

## Platform findings made during that pass

The assistant verified that the desired features were real:

- Rich Message in-body button rows;
- button styles;
- per-user ephemeral bot messages in group chats;
- Rich Message content inside those ephemeral responses.

The current platform verification has been refreshed in `04_telegram/PLATFORM_2026.md`.

## `tdx` changes reported as already implemented

The lost tree reportedly added:

### Rich Message buttons

Conceptual helpers:

```rust
button_row([
  success_callback_button("🎣 Cast a line", payload),
  callback_button("🔎 Explore", payload),
])
```

with default, primary, success, and danger styles.

### Conventional inline button styles

Conceptual helpers:

```rust
markup::primary(...)
markup::success(...)
markup::danger(...)
```

### Ephemeral send helper

A focused callback-query use-case helper conceptually like:

```rust
send::ephemeral(update, rich_message)
```

This was intended to wrap the awkward generated TDLib fields without hiding the generated API or inventing application policy.

## Group fishing redesign reported as implemented

- `/game` removed.
- Group commands reduced to `/fish` and `/help`.
- `/help` became actual group-fishing onboarding.
- `/fish` card became activity-first instead of telemetry-first.
- Shared primary action moved inside the Rich Message body.
- Each successful participant received a private ephemeral Rich Message result in the group.
- The catch still became a real persistent specimen on the normal character.
- Routine catches emitted zero extra public messages.
- Public shared card updates remained sparse/logarithmic.
- Ephemeral delivery had a callback-toast fallback.

## Private UX redesign reported as substantially implemented

- Home visual hierarchy rewritten.
- Primary Cast/Explore actions moved into the Rich Message body.
- Conventional inline markup used for navigation/secondary destinations.
- Bite presentation rewritten around observation + obvious Reel action.
- Catch result hierarchy improved, especially new species.
- Struggle/relic/escape/exploration surfaces received similar treatment.
- Semantic Back + Home model started across screens.
- Private command catalog reduced toward `/start` and `/help`.
- `/help` rewritten entirely for ordinary players.

## Work that remained unfinished before packaging

The assistant explicitly reported these remaining tasks:

- finish the same UX rewrite across the remaining progression/world panels;
- remove the now-unused old group “world status” application view;
- finish formatting the new `tdx` APIs;
- add/update `tdx` tests for Rich Message buttons;
- harden `td/fetch` to verify required new constructors in the fetched schema;
- finish README/docs changes;
- run static syntax/content/SQL checks;
- compile/format/test/Clippy against the refreshed TDLib schema;
- package a playtest ZIP.

## Reimplementation rule

Do not blindly copy helper names from this historical report. Re-read root `AGENTS.md`, inspect the latest generated TDLib API, and implement the smallest ergonomic surface needed by the current game callers.

What must be preserved is the **product behavior and architectural intent**, not necessarily every identifier from the lost tree.
