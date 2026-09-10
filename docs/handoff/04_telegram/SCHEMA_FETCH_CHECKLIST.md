# TDLib Schema Fetch Checklist for 2026 UX Features

> **Status:** operational target

The lost UX pass discovered that the local gitignored TDLib/native/schema snapshot used during diagnostics could lag Telegram's newest features.

Before implementing/rebuilding Rich Message buttons or ephemeral group interactions:

## 1. Fetch current matching TDLib/schema

Use the repository's established `./td/fetch` flow rather than manually mixing a new schema with an old native library.

## 2. Verify required constructors

At minimum, the fetched `td_api.tl` should expose equivalents of:

```text
inlineButton
inputPageBlockButtonRow
inputMessageRichMessage
sendEphemeralMessage
editEphemeralMessage
deleteEphemeralMessage
```

Useful related constructors include button styles and `editCallbackQueryMessage`.

A simple schema guard in `td/fetch` was planned during the lost UX pass. It should fail loudly with a useful message if the fetched API is too old for the game's declared UI requirements.

## 3. Regenerate types

Run the repository's normal generation/build flow. Do not hand-edit generated TDLib types.

## 4. Compile `tdx` first, then game callers

Add focused tests at the `tdx` layer for the exact generated representation of:

- styled inline/Rich buttons;
- callback payloads;
- ephemeral request defaults.

Then compile the game integration.

## 5. Re-run the complete diagnostics matrix

These APIs are new and generated shapes can change quickly. Static assumptions are not an adequate final gate.
