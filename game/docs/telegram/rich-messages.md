# Rich Message capabilities and compatibility

> **Platform facts verified against official Telegram documentation on 2026-09-10.**
> **Empirical compatibility verified on Telegram Desktop and Android on 2026-09-10.** iOS was not tested, and exact client build numbers were not recorded. Re-test client-sensitive behavior before relying on it broadly.

This document records the Rich Message capabilities that matter to Rustwater and the results of a focused live compatibility lab. It is a durable platform/UX reference, not a transcript of the experiment.

Official references:

- Bot API manual: <https://core.telegram.org/bots/api>
- Bot API changelog: <https://core.telegram.org/bots/api-changelog>
- formatted-date entities: <https://core.telegram.org/api/entities>

Rustwater talks to Telegram through TDLib/`tdx`, not the HTTP Bot API. The Bot API documentation is still useful because it documents the server-side Rich Message model and constraints; the matching generated TDLib types remain authoritative at the Rust boundary.

## Executive summary

For Rustwater, the most important confirmed result is that **compact Rich Message tables with RichText buttons inside table cells work well on both tested clients**. This makes Telegram-native list management practical for shops, inventory, equipment, crafting, selling and trading without immediately requiring a Mini App.

The reliable interaction grammar from current testing is:

- prose/headings/media for fiction and game state;
- inline RichText buttons for small contextual actions;
- Rich button rows for primary decisions;
- compact tables with an action/status cell for dense transactional lists;
- `●` / `○` as the preferred explicit selection-state glyphs;
- disabled buttons for unavailable or currently selected states;
- multi-select followed by one bulk confirmation action rather than repeated high-frequency edits.

Avoid treating Rich Message checkbox visuals as native controls: they render, but they are not clickable. Avoid nested full button rows inside list items for compact mobile UI, because Android currently renders them without the expected list indentation. Do not use `InputRichBlockThinking` in a final Rich Message; Telegram rejects it and documents it as draft-only.

## Platform model

### Rich Message size limits

Telegram currently documents these limits for one Rich Message:

| Resource | Limit |
|---|---:|
| UTF-8 text | 32,768 characters |
| Blocks, including nested structural blocks | 500 |
| Nesting depth | 16 levels |
| Media attachments | 50 |
| Table columns | 20 |
| Buttons in one Rich button row | 1–8 |
| Callback data | 1–64 bytes |

These are protocol ceilings, not UX targets. Rustwater should stay far below them on normal player screens.

### Rich buttons

A `RichMessageButton` can represent callback, URL, Web App, login, inline-query, copy-text, user and disabled actions depending on the surface. Bot API 10.3 added Rich Message buttons and `RichTextButton`, allowing buttons to appear as inline RichText rather than only in a separate footer keyboard.

Documented button styles are default, `primary`, `success`, `danger` and `link`. The documented `link` style renders without normal button borders and is documented as callback-only.

The documented button-label content is plain text plus custom-emoji and date-time RichText. Live testing found some server behavior broader than that documentation; see [Unsupported-by-documentation behavior](#unsupported-by-documentation-behavior).

### Buttons inside tables

Telegram documents table-cell contents as RichText and states that table cells may contain only inline formatting. `RichTextButton` is itself RichText. The combination is therefore structurally valid, and the live lab confirmed it on Desktop and Android.

Confirmed useful compositions include:

- one link-style callback in an action cell;
- a normal button in a cell;
- a disabled button in a cell;
- multiple inline buttons in one cell, including `− / +` controls;
- compact/striped tables with action cells.

This is the preferred basis for dense Rustwater management interfaces.

### Rich list checkboxes

`InputRichBlockListItem` supports `has_checkbox` and `is_checked`. These are presentation state. The Rich list item does not expose a checklist task identifier or checkbox callback action.

Live testing confirmed that the rendered Rich Message checkboxes are **not clickable**.

They can still be used as server-rendered state beside a separate callback button, but Rustwater should generally prefer an explicit selection marker such as `● / ○` rather than making a non-interactive checkbox look like a native control.

### Native Telegram checklists are a different feature

Telegram also has genuine interactive Checklist messages with persistent task IDs, completion metadata and permissions for other users to add or complete tasks. They are not the same thing as Rich Message list-item checkbox visuals.

Current Bot API constraints include:

- 1–30 tasks;
- 1–100 characters per task;
- 1–255 characters in the title;
- `sendChecklist` sends on behalf of a **connected business account** and requires `business_connection_id`.

Rustwater should not depend on native Checklist messages for normal inventory/shop/trade UX. Rich Message state plus callbacks works with the bot's existing architecture and was directly validated.

### Tables

Current table capabilities include:

- normal, bordered, striped and compact presentation;
- captions;
- header cells;
- horizontal and vertical alignment;
- `colspan` and `rowspan`;
- up to 20 columns.

The 20-column lab stress test was accepted on both tested clients. This establishes protocol/client acceptance, not desirability: practical Rustwater tables should normally remain narrow enough to read comfortably on phones.

### Details and quotations

Standard quotations, expandable quotations, pull quotations and `<details>` blocks rendered successfully on both tested clients.

Details are useful for secondary explanation that should be available without dominating the primary surface, for example item mechanics, recipe details or advanced stat explanations. Whether a particular screen benefits from disclosure remains a content-design decision rather than a platform limitation.

### Media

HTTPS-backed Rich Message photo media was successfully rendered and looked consistent on Desktop and Android after testing it through Rich HTML/media URL syntax.

Telegram documents Rich Message media blocks as HTTP/HTTPS URL-backed when specified through Rich HTML/Markdown. A TDLib “remote file” identifier must not be confused with an arbitrary HTTP URL.

The native map block also works, but it is a real Earth map. It is therefore not a substitute for an in-fiction Rustwater world map.

### Date-time entities

Formatted date/time entities can render a specific Unix timestamp in the user's locale/timezone, including client-updated relative time.

The timestamp must currently be between Unix time `0` and approximately **current time + 1098 days**. The initial lab used a timestamp outside this range and received `RICH_MESSAGE_DATE_INVALID`; after correcting the timestamp, date-time entities rendered successfully, including inside Rich button labels.

Relative date entities are particularly useful for Rustwater timers because the Telegram client updates the visible relative time without requiring periodic bot message edits.

### References and anchors

References are client-sensitive in current testing. After correcting the invalid date on the combined reference/anchor/math page, the reference interaction worked on Android but not on Desktop.

Do not make essential actions, navigation or required information depend on reference behavior. Anchors/references remain acceptable for optional enhancement when graceful degradation is harmless.

### Custom emoji

The lab's requested custom emoji rendered as the fallback `👍` in the current test setup rather than as the intended custom emoji. This should be treated as a deployment/account-eligibility result, not proof that non-Premium viewers cannot see custom emoji.

Telegram's current bot rules make direct bot custom-emoji usage eligibility depend on bot/account conditions such as the bot owner's Premium status (with another documented path for bots with additional Fragment usernames). Rustwater must keep critical semantics readable without custom emoji and should treat branded custom emoji as optional polish until credentialed production behavior is verified.

## Empirical compatibility matrix

`PASS` means the tested composition was accepted and behaved adequately on that client. `WEIRD` means accepted with a meaningful rendering/behavior caveat. `FAIL` means Telegram rejected the final Rich Message. iOS was not tested.

| # | Probe | Desktop | Android | Durable observation |
|---:|---|:---:|:---:|---|
| 1 | Headings + block layout | PASS | PASS | Heading hierarchy and ordinary block layout work. |
| 2 | Inline styles | PASS | PASS | Tested visual inline styles render. |
| 3 | Semantic inline entities | WEIRD | WEIRD | Only the first semantic row and direct `you` mention were clickable in the tested composition; requested custom emoji fell back to `👍`. |
| 4 | Time + refs + anchors + math | WEIRD | WEIRD | Corrected date renders; reference interaction works on Android but not Desktop. Do not depend on references. |
| 5 | Lists + checkboxes | PASS | PASS | Checkbox visuals render but are not clickable. |
| 6 | Nested list content | PASS | WEIRD | Nested action/button content loses expected list indentation on Android and can become full message width. |
| 7 | Quotes + details | PASS | PASS | Suitable platform primitives for secondary/expandable information. |
| 8 | Table styling | PASS | PASS | Default/bordered/striped/compact accepted. Caption behavior matched the configured table. |
| 9 | Table geometry + alignment | PASS | PASS | Alignment and spans accepted. |
| 10 | Rich button rows | PASS | PASS | Styles/alignment/callback delivery work. |
| 11 | Inline buttons in paragraphs | PASS | PASS | Compact contextual actions are viable. |
| 12 | Buttons inside table cells | PASS | PASS | Critical result: normal/link/disabled/multiple buttons all viable in compact table cells. |
| 13 | Buttons inside list items | PASS | WEIRD | Mobile nesting/indentation is unreliable for compact composition. |
| 14 | Mock multi-select checklist | PASS | PASS | Server-rendered selection + adjacent callback works smoothly despite checkbox itself being non-interactive. |
| 15 | Mock shop: table actions | PASS | PASS | Dense table actions work; repeated stepper edits expose edit latency and `MESSAGE_NOT_MODIFIED` edge cases. |
| 16 | Mock shop: compact inline rows | PASS | PASS | Technically sound but visually weaker; ballot-box glyph rendered as an unattractive text square. |
| 17 | Mock inventory radio list | PASS | PASS | Works; selected/disabled button causes a slight row-width/stretch difference on Android. |
| 18 | Mock trade multi-select | PASS | PASS | Selection grammar works for trade composition. |
| 19 | Button types | PASS | PASS | Tested callback/URL/copy/disabled/user/inline-query-related button types accepted. |
| 20 | Button label entities | PASS | PASS | Corrected date-time labels work; requested custom emoji falls back to `👍` in current setup. |
| 21 | Rich HTML parser | PASS | PASS | Viable authoring path. |
| 22 | Rich Markdown parser | PASS | PASS | Viable authoring path; selected HTML blocks can coexist where Telegram permits them. |
| 23 | Map block | PASS | PASS | Works, but represents the real-world Earth map. |
| 24 | RTL + automatic blocks | PASS | PASS | RTL visibly works; toggling automatic-block detection produced no obvious visible difference in this probe. |
| 25 | 8 buttons + 20-column table | PASS | PASS | Documented stress limits accepted; not a recommended normal layout. |
| 26 | Bold inside button label | PASS | PASS | Accepted despite current documentation describing a narrower allowed label set. Treat as undocumented behavior. |
| 27 | `link` style on URL button | PASS | PASS | Accepted despite current documentation saying `link` style is callback-only. Treat as undocumented behavior. |
| 28 | Thinking block in final message | FAIL | FAIL | Server rejects with `RICH_MESSAGE_BLOCK_UNSUPPORTED`; documented as draft-only. |
| 29 | HTTPS remote photo | PASS | PASS | Corrected URL-backed photo test renders consistently across tested clients. |
| 30 | Composite Rustwater economic UI | PASS | PASS | Practical composition works; selection glyph and no-op edit handling need product-side care. |

## Interaction constraints learned from the lab

### Message edits are the practical bottleneck

Interactive Rich Message state usually requires the bot to edit the message after a callback. In the live lab, repeated quantity-stepper and multi-select edits felt noticeably slower than local UI controls.

Do not design normal Rustwater management flows around many rapid taps. Prefer:

- selecting a small number of meaningful items and confirming once;
- `Buy 1`, `Buy 5`, `Buy max`-style coarse operations when quantities matter;
- bulk actions such as `Select all`, `Clear`, `Sell selected`;
- actions that immediately commit when an extra confirmation adds no safety or meaning.

This is a UX constraint observed in practice. It is intentionally not expressed as a guessed numeric Telegram rate limit.

### Never send a known no-op edit

The lab hit `MESSAGE_NOT_MODIFIED` when state was already at a boundary, for example pressing `+` after a quantity had reached its maximum or clearing an already-clear selection.

Rustwater presenters/callback handlers should avoid issuing an edit when the rendered next state is identical to the current state. The callback should still be acknowledged promptly.

This applies to:

- clamped quantity controls;
- `Clear` on an empty selection;
- choosing an already-equipped item;
- stale/duplicate callbacks that resolve to current state;
- any other transition whose rendered result is unchanged.

### Use explicit state markers only for real selection state

The following selection glyphs were compared directly:

| Selected | Unselected | Verdict |
|---|---|---|
| `●` | `○` | **Preferred for true radio/selection state** |
| `✓` | `·` | Acceptable lighter alternative |
| `✅` | `⬜️` | Avoid as fake checkbox UI |
| `☑️` | `◻️` | Avoid as fake checkbox UI |
| `◆` | `◇` | Avoid |
| `✓` | `—` | Avoid |

`● / ○` communicates selected/unselected state without pretending that a rendered Rich Message checkbox itself is clickable. The later product-design reset explicitly corrected an overgeneralization of this result: it is **not** the default marker for every list, physical location, equipped item or completion state.

Use it narrowly when the user is genuinely choosing one/more options. For completion use completion semantics (`✓`/`✅`) where appropriate; for physical world state and equipment, prefer direct labels/layout rather than redundant radio metaphors.

## Rustwater UI patterns supported by the evidence

These are evidence-backed patterns, not a requirement to convert every screen into a table.

### Dense transactional lists

Use a compact table when rows naturally have item/state/value/action structure:

```text
○ Glow Grub       ×4    5c    Add
● Rust Shrimp     ×1    8c    Remove
○ Glass Minnow    ×0   12c    Sold out
```

The action word can be an inline RichText button inside the final cell. `Sold out` can be a disabled button.

Good candidates:

- shops;
- inventory/equipment;
- selling;
- crafting ingredient selection;
- player/NPC trading;
- compact contract/objective actions when the content is genuinely tabular.

### Single-choice equipment/state

A compact table plus `● / ○`, or a disabled selected-state button, can approximate a genuine single-choice list.

Avoid redundantly communicating the same state twice. For example, use either a selection marker or an `Equipped` state treatment when one is sufficient rather than mechanically rendering both on every row.

Be aware that the disabled/selected button produced a slight row-size difference on Android. Prefer a stable text/action-cell width when visual jitter matters.

### Multi-select

Multi-select works well when the expected number of toggles is small:

```text
● Rumor Carp       31c   Remove
○ Glass Eel        17c   Add
● Moon Crab        23c   Remove

2 selected · 54c

[Clear]                 [Sell · 54c]
```

Do not require users to toggle dozens of rows one by one. Provide bulk operations for large collections.

### Contextual actions in prose

Inline RichText buttons inside paragraphs work on both tested clients and are appropriate when an action belongs directly to one sentence/object:

```text
A strange mark runs along the lure.  Inspect
```

This is preferable to adding every contextual action to a large bottom keyboard.

### Primary decisions

Rich button rows remain appropriate when a decision deserves clear visual weight, especially encounters and other moment-to-moment gameplay. Dense table-cell controls are a management primitive, not the universal visual language of the game.

### Fiction versus management

The lab solves the problem of dense Telegram-native management UI, but Rustwater should preserve a visual distinction:

- **fiction/gameplay:** image, prose, observations, headings, selective contextual buttons, prominent decisions;
- **management:** compact structured rows, selection state, value/status columns and colocated actions.

Do not turn exploration, fishing outcomes, discoveries or NPC scenes into spreadsheet-like interfaces merely because tables are capable.

## Unsupported-by-documentation behavior

Two deliberately risky probes were accepted by Telegram:

1. bold RichText nested inside a Rich Message button label;
2. `link` style applied to a URL Rich Message button.

Current Bot API documentation states that button labels may contain only plain text, custom emoji and date-time entities, and that `link` style is allowed only for callback buttons.

Therefore these two accepted behaviors are **undocumented compatibility observations**, not stable platform contracts. Do not use them for essential Rustwater UX without re-verifying after Telegram/TDLib updates and across the clients we support.

## Known non-options / weak options

- **Final-message Thinking block:** rejected; use only in `sendRichMessageDraft` if Rustwater ever has a real streaming-draft use case.
- **Rich Message list checkbox as the click target:** not interactive.
- **Nested full button rows inside list items:** visually unreliable on Android for compact list composition.
- **References for essential navigation:** client-inconsistent between tested Desktop and Android.
- **Custom emoji as semantic state:** current deployment fell back to ordinary emoji; essential meaning requires text/standard-glyph fallback.
- **Rapid quantity steppers:** technically possible but a poor fit for networked message-edit latency.
- **Native map for Rustwater geography:** the map block is an Earth map, not an in-world mapping primitive.

## Re-test triggers

Re-run focused credentialed tests when any of these changes materially:

- Bot API or TDLib gains a new Rich Message release;
- generated `td_api.tl` changes RichText/RichBlock/button types;
- Telegram Desktop/Android/iOS changes Rich Message rendering substantially;
- Rustwater begins depending on an undocumented behavior such as bold button labels or URL `link` styling;
- branded custom emoji becomes production-critical;
- iOS becomes part of the active compatibility target.

The highest-value regression probes are table-cell buttons, inline paragraph buttons, compact tables, disabled buttons, multi-select editing, relative date-time rendering, remote media, and any currently undocumented composition Rustwater chooses to ship.
