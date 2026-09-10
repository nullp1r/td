# `tdx` Formatting Composition Redesign — 2026-09-10

> **Status:** implemented in source; one user-found `Styled<T>` trait integration fix is incorporated; full compiler/rustfmt/Clippy validation pending on a Rust-equipped machine.

## Why this exists

Playtesting exposed three related API/UX defects:

1. the group shoal card rendered a duration as ordinary text, so `Moves in 12m` became stale unless the bot edited the message;
2. `table(["", ""])` fabricated an empty header row because the `tdx` constructor treated headers as mandatory even though Telegram does not;
3. mixed rich text required expressions such as `paragraph(concat([plain("A "), plain(bold(name)), plain("." )]))`, which made the wrapper harder to use than the generated API deserved.

The formatting layer was redesigned rather than adding game-local workarounds.

## Settled composition rule

Use Rust's native collection syntax as the composition language:

- tuple: fixed heterogeneous fragments;
- array: fixed homogeneous fragments;
- `Vec`: dynamic homogeneous fragments.

Examples:

```rust
line(("Weight: ", code(format_args!("{weight:.2} kg"))))

lines((
  bold("Inventory"),
  "",
  ("Coins: ", coins),
))

paragraph((
  "This shared shoal features ",
  bold(species_name),
  ". Everyone gets one free cast during its window.",
))
```

The function determines how the outer collection is interpreted:

- `line((a, b, c))` concatenates `a`, `b`, and `c` into one line;
- `lines((a, b, c))` renders `a`, `b`, and `c` as separate lines;
- an inner tuple inside `lines`, such as `("Coins: ", coins)`, remains one composed line.

Tuple implementations exist through arity 20. The arity list is generated from one shared internal macro helper because Telegram tables currently support up to 20 columns and existing multiline callers also need large fixed tuples.

## APIs deliberately removed

The redesign does **not** preserve compatibility aliases. The following composition mechanisms were removed:

- `Text + part`;
- `Text += part`;
- `Styled + part`;
- `plain(...)`;
- `concat(...)`;
- `empty()`;
- `Extend` / `FromIterator` composition for `Text`.

There is one declarative composition model instead of parallel syntaxes.

For genuinely dynamic ordinary-text construction, `Text::push(...)` remains the direct imperative API:

```rust
let mut text = line("Catches:");
for catch in catches {
  text.push(("\n", catch.name));
}
```

No public formatting macro, builder object, `.inline()` extension method, compatibility wrapper, or extra dependency was introduced.

## Ordinary text implementation

`Part` streams directly into a destination `Text` buffer. Tuples recursively stream their members into that same buffer.

`lines(...)` therefore allocates one destination `Text` and inserts separators while each line writes into it. It no longer needs each source line to have already become an independent `Text` first.

This preserves the important existing properties:

- one UTF-8 text buffer;
- UTF-16 entity offsets calculated against the final destination;
- moved or borrowed `Text` fragments have their existing entities rebased correctly;
- owned `String` can still become the initial `Text` without copying its allocation;
- `format_args!` and primitive scalar values write directly into the ordinary text buffer.

The blanket `Part for T: Display` implementation was removed because it conflicts with heterogeneous tuple implementations under Rust coherence rules. Useful leaf types are implemented explicitly; arbitrary formatted expressions should use `format_args!`.

## Rich text implementation

`IntoRichText` supports the same tuple/array/`Vec` model.

Leaf and styled values still convert directly to generated `RichText` nodes. Composition buffers are allocated only when an actual tuple/array/`Vec` needs a `richTexts` aggregate.

Nested composition nodes are flattened where possible:

```rust
paragraph(("A", ("B", bold("C")), "D"))
```

produces one top-level `richTexts` sequence rather than a chain of nested composition containers. A style remains a real node around its composed content, so this remains meaningful:

```rust
bold(("Important: ", italic("very"), " important"))
```

Rich text necessarily owns generated string nodes; ordinary text keeps the stronger streaming/no-intermediate-buffer behavior.

### `Styled<T>` trait correction found during playtesting

The first tuple-formatting draft implemented only `append_to` for `Styled<T>: IntoRichText`. The user compiled/playtested the tree and found that the trait also requires the scalar `into_rich_text` path. The corrected implementation is:

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

Keep both methods: scalar styled values must convert directly, while composed values can append them into a flattened `richTexts` destination.

## Tables

Telegram table headers are optional. `tdx` now models that directly:

```rust
table()
  .row(("🐟 Shoal", species_name))
  .row(("👥 Casts", participants))
```

A real header is opt-in:

```rust
table()
  .header(("Species", "Weight"))
  .row((species_name, weight))
```

Rows accept heterogeneous tuples and homogeneous arrays/`Vec`s. Ordinary values become default cells directly. Use `cell(...)` only when native cell metadata is needed, for example alignment or spans:

```rust
table().row((species_name, cell(weight).right()))
```

This removes the old `table(["", ""])` workaround and therefore removes the empty rendered header row it created.

## Relative timestamps

Do not periodically edit a message merely to keep a countdown string fresh when Telegram can render the time itself.

`relative_time(fallback, unix_time)` is a small free helper over the generated `dateTimeFormattingTypeRelative` date/time entity. Supporting Telegram clients update the relative display while the message is visible without bot traffic.

The shared-shoal view now carries both:

- `resets_in_ms` for human-readable fallback content;
- `ends_at_unix` for the native relative date entity.

The public card uses temporally robust wording (`during its window`) and labels the field `Rotation`, so an old card remains understandable after the timestamp crosses zero. The server remains authoritative when an expired action is clicked.

Periodic edit scheduling is deliberately **not** introduced at this stage. It would add rate-limit usage, timers, restart/lifecycle state, and message tracking without solving a requirement that Telegram already handles client-side.

## Source-size accounting

Per repository guidance, production line count was checked against the UX-audited predecessor before this rewrite:

| Area | Before | Current | Delta |
|---|---:|---:|---:|
| `tdx/src/format.rs` | 79 | 114 | +35 |
| `tdx/src/format/text.rs` | 241 | 322 | +81 |
| `tdx/src/format/rich.rs` | 98 | 166 | +68 |
| `tdx/src/format/style.rs` | 172 | 177 | +5 |
| `tdx/src/format/rich/table.rs` | 136 | 186 | +50 |
| **all `tdx/src/**/*.rs`** | **2250** | **2489** | **+239** |

The increase is primarily the typed tuple/collection contracts and their tests/docs-facing support. Tuple arity enumeration is centralized once and reused by ordinary text, rich text, and table rows. No compatibility layer or parallel builder abstraction was retained. Game production source stayed at 5,644 lines during the caller migration; the view timestamp field and test assertion offset presentation simplifications elsewhere.

The next compiler-equipped diagnostics pass should be used to challenge this draft further: any line that exists only to satisfy an avoidable compiler/lint issue should be removed rather than normalized as permanent API weight.
