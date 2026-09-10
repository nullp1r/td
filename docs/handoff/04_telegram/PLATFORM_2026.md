# Telegram Platform Capabilities Relevant to the Game (2026)

> **Status:** verified platform context as of 2026-09-09; reverify against current TDLib schema when implementing.

The UX redesign was prompted partly by discovering that Telegram added exactly the interaction primitives this game needs.

## Rich Messages

Telegram Bot API 10.1 (2026-06-11) introduced Rich Messages with structured blocks, rich text, tables, formulas, media, etc.

Bot API 10.3 (2026-08-24) added Rich Message buttons/button rows and styles.

Official Bot API reference:

- https://core.telegram.org/bots/api

Current Rich Message button concepts include:

- in-message button rows;
- callback buttons;
- URL/Web App/etc. button types;
- styles such as default/primary/success/danger (and current APIs may also expose link style where valid).

## Ephemeral messages

Bot API 10.2 (2026-07-14) introduced ephemeral messages: bot messages in groups visible only to a specific user and the bot.

This is a foundational fit for the MMO because it permits private personalized interactions **without leaving the group context**. Telegram also exposes ephemeral bot commands (`botCommand.is_ephemeral` in the current TDLib schema), allowing a group command and its response flow to remain private to the invoking user.

Official feature explanation:

- https://core.telegram.org/bots/features#ephemeral-messages

## Current TDLib schema findings from the session

The current upstream `td_api.tl` was verified to contain:

- `inlineButton`;
- button styles including default/primary/danger/success/link;
- `inputPageBlockButtonRow`;
- `inputMessageRichMessage`;
- `sendEphemeralMessage`;
- `botCommand.is_ephemeral`;
- `editEphemeralMessage`;
- `editEphemeralMessageCaption`;
- `deleteEphemeralMessage`;
- `editCallbackQueryMessage`.

Current upstream schema:

- https://github.com/tdlib/td/blob/master/td/generate/scheme/td_api.tl

## Important schema-version warning

The diagnostics environment used a gitignored TDLib/schema snapshot that lagged some of these 2026 additions. The repository's `./td/fetch` flow already exists to fetch matching current TDLib + schema.

Before relying on these constructors:

1. run/update `./td/fetch`;
2. regenerate the `td` types as the repository expects;
3. verify the fetched `td_api.tl` includes the needed constructors;
4. compile the whole workspace.

The lost UX pass intended to harden `td/fetch` with explicit constructor verification but did not finish/package that change.
