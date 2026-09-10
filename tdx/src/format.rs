//! Compose entity text or native rich-message documents without parsing markup.
//!
//! [`Text`](crate::format::Text) maintains a UTF-8 buffer and UTF-16 entity ranges.
//! [`rich`](fn@crate::format::rich) collects page blocks into a generated message.
//! Both accept shared inline styles; rich layout uses generated `TDLib` values
//! directly, so native fields remain available without wrapper conversions.
//!
//! # Composition
//!
//! Tuples are the common composition primitive: they concatenate heterogeneous
//! fragments in order. Arrays and vectors do the same for homogeneous fragments.
//! This keeps fixed expressions declarative while allowing ordinary text to stream
//! directly into one destination buffer.
//!
//! # Entity text
//!
//! ```
//! use tdx::format::*;
//!
//! let text = lines((
//!   (bold("Inventory"), " — ", italic("updated")),
//!   "",
//!   ("Weight: ", code(format_args!("{:.2} kg", 1.5))),
//! ));
//! assert_eq!(&*text, "Inventory — updated\n\nWeight: 1.50 kg");
//! let caption: Option<tdx::types::formattedText> = text.into();
//! ```
//!
//! `line(parts)` composes one line without a separator. `lines(lines)` interprets
//! the outer tuple/array/vector as separate lines and inserts exactly one newline
//! between entries. Nest styles directly, including composed style content such as
//! `bold(("total: ", code(3)))`. `markdown` and `html` invoke `TDLib`'s synchronous
//! parsers when the source is markup instead. Entity validity and permitted nesting
//! are left to `TDLib`.
//!
//! # Rich documents
//!
//! The same tuple rule applies to rich inline content, so mixed styling requires
//! no conversion wrappers:
//!
//! ```
//! use tdx::format::*;
//!
//! let stock = table()
//!   .header(("Item", "Count"))
//!   .row(("Trout", code(3)))
//!   .bordered();
//! let mut message = rich([
//!   paragraph(("Hello, ", bold("Alice"))),
//!   stock.into(),
//!   bullet_list([list_item("Fresh today").checked(true)]),
//!   details("Notes", [block_quote([paragraph(italic("Keep chilled"))])]),
//! ]);
//! message.message.is_rtl = false;
//! ```
//!
//! Tables are headerless by default; add `.header(...)` only when the data has a
//! real header row. `Cell`, `Table` and `ListItem` name generated types. Use
//! [`cell`] when a cell needs explicit alignment/span settings; ordinary row values
//! convert directly. Fluent methods come from `CellExt`, `TableExt` and
//! `ListItemExt`, all included by `use tdx::format::*` and the crate prelude.
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

macro_rules! tuple_impls {
  ($impl:ident) => {
    $impl!(T0 p0);
    $impl!(T0 p0, T1 p1);
    $impl!(T0 p0, T1 p1, T2 p2);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9, T10 p10);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9, T10 p10, T11 p11);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9, T10 p10, T11 p11, T12 p12);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9, T10 p10, T11 p11, T12 p12, T13 p13);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9, T10 p10, T11 p11, T12 p12, T13 p13, T14 p14);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9, T10 p10, T11 p11, T12 p12, T13 p13, T14 p14, T15 p15);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9, T10 p10, T11 p11, T12 p12, T13 p13, T14 p14, T15 p15, T16 p16);
    $impl!(T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9, T10 p10, T11 p11, T12 p12, T13 p13, T14 p14, T15 p15, T16 p16, T17 p17);
    $impl!(
      T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9, T10 p10, T11 p11, T12 p12, T13 p13, T14 p14, T15 p15, T16 p16, T17 p17, T18 p18
    );
    $impl!(
      T0 p0, T1 p1, T2 p2, T3 p3, T4 p4, T5 p5, T6 p6, T7 p7, T8 p8, T9 p9, T10 p10, T11 p11, T12 p12, T13 p13, T14 p14, T15 p15, T16 p16, T17 p17, T18 p18,
      T19 p19
    );
  };
}

pub mod parse;
pub mod rich;
pub mod style;
pub mod text;

pub use parse::*;
pub use rich::*;
pub use style::*;
pub use text::*;
