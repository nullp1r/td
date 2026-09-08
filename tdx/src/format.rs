//! Compose entity text or native rich-message documents without parsing markup.
//!
//! [`Text`](crate::format::Text) maintains a UTF-8 buffer and UTF-16 entity ranges.
//! [`rich`](fn@crate::format::rich) collects page blocks into a generated message.
//! Both accept shared inline styles; rich layout uses generated `TDLib` values
//! directly, so native fields remain available without wrapper conversions.
//!
//! # Entity text
//!
//! Start with `line`, append with `+` or `+=`, and join parts with `lines`.
//! Display values and styles stream into one destination; existing owned text
//! reuses its buffer and borrowed text copies its entities with adjusted offsets.
//!
//! ```
//! use tdx::format::*;
//!
//! let text = lines([
//!   bold("Inventory") + " — " + italic("updated"),
//!   empty(),
//!   line("Weight: ") + code(format_args!("{:.2} kg", 1.5)),
//! ]);
//! assert_eq!(&*text, "Inventory — updated\n\nWeight: 1.50 kg");
//! let caption: Option<tdx::types::formattedText> = text.into();
//! ```
//!
//! Nest styles directly with `bold(italic("text"))`. Use `.text()` to resume
//! composition from a string or generated formatted text. `markdown` and `html`
//! invoke `TDLib`'s synchronous parsers when the source is markup instead.
//! Entity validity and permitted nesting are left to `TDLib`.
//!
//! # Rich documents
//!
//! `plain` converts inline content to the common rich-text enum, preserving its
//! styles; `concat` joins it without separators. Blocks, tables and list items
//! can then be mixed into a document. Use a vector to accumulate blocks in a loop.
//!
//! ```
//! use tdx::format::*;
//!
//! let stock = table(["Item", "Count"])
//!   .row([cell("Trout"), cell(code("3")).right()])
//!   .bordered();
//! let mut message = rich([
//!   paragraph(concat([plain("Hello, "), plain(bold("Alice"))])),
//!   stock.into(),
//!   bullet_list([list_item("Fresh today").checked(true)]),
//!   details("Notes", [block_quote([paragraph(italic("Keep chilled"))])]),
//! ]);
//! message.message.is_rtl = false;
//! ```
//!
//! `Cell`, `Table` and `ListItem` name generated types. Start with `cell`, `table`
//! and `list_item` for layout defaults; their `Default` implementations remain
//! the generated ones. Fluent methods come from `CellExt`, `TableExt`
//! and `ListItemExt`, all included by `use tdx::format::*` and the crate prelude.
//! Lists take explicit items; set an item's `blocks` field for nested content.
//!
//! # Rendering differences
//!
//! URL, language, command and emoji fallback strings remain borrowed until
//! rendering. Rich text requires owned tree nodes; ordinary text allocates only
//! the entity metadata it uses. Rich inline `pre` retains fixed-width styling
//! without its language, while `quote`, `quote_expandable` and `media_timestamp`
//! retain only their content. Use rich `preformatted`, `block_quote` and
//! `block_quote_expandable` blocks for document layout. A separate
//! `bot_command_target` applies only to rich text; ordinary command entities use
//! their visible text. Custom emoji fallback text is retained in both formats.
//!
//! Generated rich text and page blocks cover native features without a helper.

pub mod parse;
pub mod rich;
pub mod style;
pub mod text;

pub use parse::*;
pub use rich::*;
pub use style::*;
pub use text::*;
