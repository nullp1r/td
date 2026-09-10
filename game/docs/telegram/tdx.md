# `tdx` conventions for Rustwater

`tdx` is the in-house ergonomic layer over generated TDLib Rust types. Rustwater depends on it directly and may evolve it when a Telegram capability would improve the product.

`tdx` must not become a reason to give up on a useful Telegram-native interaction.

## Boundary rule

Good `tdx` helpers:

- solve a demonstrated caller problem;
- stay thin over generated TDLib requests/types;
- preserve access/editability of generated values;
- avoid game-specific policy;
- reduce repetitive/error-prone boilerplate;
- stay dependency-light;
- follow current TDLib rather than preserving stale wrapper compatibility forever.

Game rules remain in `game`; Telegram ergonomics live in `tdx`.

## Formatting composition model

The settled formatting mental model is intentionally small.

### Fixed heterogeneous composition → tuples

Use tuples when adjacent parts have different concrete types, e.g. styled + plain + formatted values.

### Homogeneous runtime composition → arrays / `Vec`

Use arrays/`Vec` when the parts share a type or are generated dynamically.

This model applies to ordinary entity text, Rich Text and table rows.

## Ordinary text

Use `line(...)` for one composable text expression and `lines(...)` for line-separated composition.

`lines(...)` should stream into one output buffer rather than building a `Vec<String>` and joining it.

For genuinely dynamic ordinary text, use `Text::push`/the `Text` buffer API directly.

Do **not** reintroduce legacy composition helpers/operators:

- `plain(...)`;
- `concat(...)`;
- `empty(...)`;
- composition `Add` / `AddAssign`.

Those syntaxes were deliberately removed to keep one composition rule.

## Rich Text

Rich Text follows the same tuple/collection model through `IntoRichText`.

A critical implementation detail for styled values is:

```rust
impl<T: IntoRichText> IntoRichText for Styled<'_, T> {
  fn into_rich_text(self) -> RichText {
    self.kind.into_rich_text(self.content)
  }

  fn append_to(self, texts: &mut Vec<RichText>) {
    texts.push(self.into_rich_text());
  }
}
```

Keep both methods. Scalar styled values must convert directly while composed values can append into a flattened Rich Text destination.

When building content for an async spawned handler, do not allow non-`Send` `fmt::Arguments` temporaries to survive across `.await`; materialize the Rich Message/request before awaiting the TDLib call where necessary.

## Tables

`table()` creates a **headerless** table by default.

Add a header only when there is a real header:

```rust
table().header(("Species", "Record"))
```

Rows accept tuples, arrays or vectors according to the same composition rule. Use explicit `cell(...)` only when alignment/span/cell metadata needs customization.

Never fake a headerless table with empty strings.

## Native relative time

Use `relative_time(fallback, unix_time)` for client-updated relative timestamps rather than periodically editing a message only to refresh “in N minutes.”

The fallback text must remain sensible if a client cannot render/update the entity.

## Custom emoji

`custom_emoji(fallback, custom_emoji_id)` composes a custom emoji entity/Rich Text node while retaining readable alternative text.

Bot eligibility to send custom emoji is a Telegram server/account rule; see [`platform.md`](platform.md).

## Rich media

`tdx` currently exposes Rich Message block helpers including:

- `photo`;
- `video`;
- `animation`;
- `audio`;
- `document`;
- `voice_note`;
- `collage`;
- `slideshow`;
- `map`;
- structured text/table/button blocks.

Generated TDLib types remain available directly when advanced metadata is required. Do not create game-local wrappers around every generated field.

## Adding a new Telegram feature

When current Telegram supports something but the local wrapper/schema does not:

1. verify upstream behavior/schema;
2. update `td/fetch` / generated `td-*` surface as needed;
3. add minimal `tdx` ergonomics for demonstrated use;
4. update Rustwater callers;
5. add/update focused `tdx` tests;
6. run the full diagnostics matrix;
7. update [`platform.md`](platform.md) or other affected docs.

Backward-compatibility shims for superseded internal APIs are not required unless a real external consumer exists.
