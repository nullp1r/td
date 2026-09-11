# Telegram interface grammar during the reset

This file records product-level UI corrections from the reset discussion. The existing Telegram/Rich Message documents remain the evidence base for what the platform accepts; this file states how the replacement game should use those primitives.

## Locked: scene and state before menus

Normal play should lead with the character's situation: current place, current conditions, what is happening, what is unusual, who is relevant and what the character can do. Avoid returning to a sitemap-style dashboard merely because it is easy to implement.

Preserve the useful history rule:

> **panels for state; messages for events**

Mutable scene/inventory/travel/encounter state usually belongs in an editable panel. Discoveries, significant trades, records, world events and other moments worth remembering may deserve durable chat history.

## Locked: primary verbs must look actionable

Core contextual verbs such as `Equip`, `Light`, `Read`, `Trade`, `Travel`, `Pry`, `Join` and similar actions should use real button affordances when practical. Do not make primary game verbs look like ordinary textual hyperlinks simply because RichText inline buttons are compact.

Inline RichText buttons remain useful for compact secondary drill-down such as `Details`, `View character`, `Inspect record` or similarly lightweight navigation.

A platform primitive being technically viable does not make it the default UX for every interaction.

## Locked correction: `● / ○` is not a universal game language

The Rich Message lab established that `● / ○` renders cleanly for selected/unselected state. The reset discussion explicitly rejected generalizing that result into a marker for every list.

Use `● / ○` narrowly when the UI is genuinely a radio/single-choice or selection control. Do not use it as a generic marker for:

- physical locations/travel destinations;
- arbitrary inventory rows;
- completion state;
- presence/activity;
- an equipped row when a separate `Equipped` label already communicates the same state.

Depending on composition, a selected radio row may use `●` with an intentionally empty/non-action cell rather than repeating both `●` and `Equipped`.

Use completion semantics such as `✓`/`✅` when the meaning is actually completed/successful, not selected.

## Leaning: restrained semantic emoji vocabulary

Emoji should clarify hierarchy and meaning rather than decorate every noun. Current useful semantics include:

- `✓` / `✅` — completed/successful;
- `✨` — unusual/noteworthy;
- `🏆` — record/exceptional achievement;
- `🌍` — world/global scope;
- `🗝` — clue/relic/key-like mystery;
- `📍` — location;
- `👥` — characters/people/social presence;
- `🪙` or denomination text — value/currency;
- `🌦` — conditions/weather;
- `⚠` — warning/risk.

Activity-specific emoji such as fishing symbols remain valid when the activity itself is actually relevant; they are no longer global product identity anchors.

Exact symbols remain polish-level choices and must keep text fallbacks where meaning would otherwise be ambiguous.

## Leaning: density follows information shape

Compact tables are useful for genuinely tabular transactional state, but should not become the default representation for scenes, travel or every inventory view. Rich button rows, prose, tables and inline controls should be selected based on the information shape.

Avoid interaction designs that require rapid repeated message edits. The Rich Message lab found edit latency noticeable, and known no-op transitions must not issue edits that can produce `MESSAGE_NOT_MODIFIED`.

## Leaning: current location replaces abstract Home

The reset's preferred target is that opening the game reconstructs the character's current scene rather than presenting an abstract Home dashboard. Global surfaces such as inventory, character/profile, journal/knowledge and map remain reachable through persistent navigation.

This intentionally supersedes the fishing-era tendency to make `Home` a universal destination, but remains **Leaning** until the first replacement location flow is playtested.

## Open

- Exact persistent navigation controls around the current scene.
- Whether some dense inventory/shop/trade screens should use table-cell buttons versus button rows after the new item model exists.
- Map presentation in Telegram and whether a Mini App eventually owns dense/spatial views.
- Custom emoji/iconography once the final game identity is clearer.
