# Telegram platform capabilities

> **Platform facts verified against official Telegram documentation on 2026-09-10.** The latest Bot API release in the official changelog at verification time is 10.3 (2026-08-24). Re-verify before relying on details that may change.

Rustwater uses TDLib through the in-house `tdx` crate, but Bot API/MTProto documentation is still useful for understanding server-side bot capabilities and product semantics.

## Rich Messages

**Verified:** Bot API 10.1 (2026-06-11) introduced Rich Messages. Bot API 10.2 (2026-07-14) expanded outgoing Rich Message blocks/media, and Bot API 10.3 (2026-08-24) added buttons inside Rich Messages plus additional block capabilities.

Official changelog: <https://core.telegram.org/bots/api-changelog>

Current local TDLib schema contains `inputMessageRichMessage`, rich text/block types, media blocks and `inputPageBlockButtonRow`. `tdx` provides higher-level composition/helpers while leaving generated types accessible.

**Product implication:** Rich Messages are not decoration; they are the primary native game-panel primitive and are well suited to the planned image-backed screens.

## Ephemeral messages in groups

**Verified:** Bot API 10.2 introduced ephemeral messages/commands visible only to a specific user and the bot inside a group context.

Official changelog: <https://core.telegram.org/bots/api-changelog>

Current local schema contains ephemeral send/edit/delete support and `botCommand.is_ephemeral`. Rustwater already uses this for group help and personalized group-shoal results.

**Product implication:** preserve the public-shared-state + private-ephemeral-result pattern for social mechanics where it fits.

## Custom emoji — yes; simplest eligibility is bot-owner Premium

**Verified:** since Bot API 9.4 (2026-02-09), bots may use custom emoji in messages they directly send to private chats, groups and supergroups **if the owner of the bot has a Telegram Premium subscription**. This is the simplest relevant paywall for Rustwater: Premium belongs to the owner account, not to a separate “bot subscription.”

Current Bot API documentation also lists another eligibility path: bots that purchased additional usernames on Fragment may use custom-emoji entities/icons. Do not assume every surface has identical eligibility semantics; verify the actual direct/inline/Guest surface when implementing it.

Telegram's custom-emoji launch documentation states that all users, Premium or not, can see custom animated emoji in messages. Bot API 9.4 also added custom-emoji IDs and button styles to keyboard buttons; current button documentation uses the same owner-Premium/additional-username eligibility model.

Official changelog: <https://core.telegram.org/bots/api-changelog>

Current Bot API manual (formatting/button eligibility): <https://core.telegram.org/bots/api>

Telegram's custom emoji announcement states that all users, Premium or not, can **see** custom animated emoji in messages:
<https://telegram.org/blog/custom-emoji?setln=en>

Core representation details:
<https://core.telegram.org/api/custom-emoji>

Current local TDLib schema contains:

- `textEntityTypeCustomEmoji`;
- `richTextCustomEmoji`;
- `keyboardButton.icon_custom_emoji_id`;
- `inlineKeyboardButton.icon_custom_emoji_id`.

`tdx::format::custom_emoji` already exists for text/Rich Text composition.

### Rustwater use

A branded Rustwater custom-emoji pack is viable. Good candidates include location/faction icons, status/weather accents, title/rarity symbols and recurring action motifs. For normal direct game messages, keeping the bot owner's Premium subscription active is the straightforward operational requirement.

Do not make critical semantics depend solely on a custom emoji. Maintain readable text/standard-emoji fallback, and perform a credentialed TDLib bot integration test before shipping because eligibility is a server/account rule and may differ across direct, inline and Guest surfaces.

## Classic inline mode

**Verified:** an inline-enabled bot can be invoked by typing `@botusername query` in the input field of any chat; the user chooses a result that is then sent into that chat.

Official guide: <https://core.telegram.org/bots/inline>

Inline mode is enabled via BotFather (`/setinline`). The local TDLib schema includes `updateNewInlineQuery` and `answerInlineQuery`.

Rustwater considers classic inline mode a **committed future product surface**. See [`../ux/interaction-surfaces.md`](../ux/interaction-surfaces.md) for intended sharing use cases.

## Guest Mode — mention a bot that is not in the chat

**Verified:** Bot API 10.0 (2026-05-08) introduced Guest Mode. A guest-enabled bot can be mentioned by username from supported chats and post a result directly even if it is not a member.

Official protocol guide: <https://core.telegram.org/api/bots/guest-mode>

Important current behavior from the official guide:

- works in non-secret private chats, groups and supergroups;
- content-protected groups/supergroups are excluded;
- a user invokes the bot by mentioning its `@username`; Telegram's bot-features guide also allows replying to one of the guest bot's messages;
- up to three guest bots can be mentioned at once;
- bot receives the triggering message and referenced/replied-to messages, not arbitrary full-chat history or participant state;
- response uses an inline-result-shaped payload posted by the bot as a guest.

Current local TDLib schema exposes `userTypeBot.supports_guest_queries`, `updateNewGuestQuery` and `answerGuestQuery`.

Rustwater currently treats this as a **strong candidate**, not an implemented surface.

## Reply keyboards / custom keyboards

Current TDLib schema supports `replyMarkupShowKeyboard`, including persistent, resized, one-time/personal behavior and button fields such as custom emoji/style plus supported button actions.

Rustwater's current DM navigation does not use a persistent reply keyboard. Rich Message/inline navigation is preferred. A reply keyboard remains available if a future interaction has a genuine always-present action-pad use case.

## Private-chat topics

**Verified:** Bot API 9.3 (2025-12-31) added forum-topic mode for private bot chats; Bot API 9.4 expanded bot topic creation/controls.

Official changelog: <https://core.telegram.org/bots/api-changelog>

Possible Rustwater use: long-running NPC/story/activity threads. This is exploratory, because topics can also fragment a simple conversational interface.

## Inline/Mini App bridges

Telegram supports buttons that switch a user into inline mode, and Mini Apps can also transition to inline-query flows.

Useful references:

- <https://core.telegram.org/constructor/keyboardButtonSwitchInline>
- <https://core.telegram.org/api/bots/webapps>

This can make “Share catch/art/profile” a one-tap bridge from a private game panel into another chat.

## Schema freshness rule

Telegram/TDLib moves quickly. Before implementing a recently introduced feature:

1. verify current official Telegram behavior;
2. inspect/fetch the matching current `td_api.tl`;
3. regenerate the local TD crates as required;
4. add the thinnest useful `tdx` ergonomics;
5. test the actual bot/account capability with credentials where server-side eligibility matters;
6. update this document if platform assumptions changed.

Do not silently design against stale generated types.
