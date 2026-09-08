//! Rich messages built from generated inline trees and page blocks.

use std::fmt::Arguments;

use td_types::enums::{InputPageBlock, RichMessageSource, RichText};
use td_types::types;

macro_rules! fluent {
  ($(#[$doc:meta])* $trait:ident for $ty:ty {
    $($(#[$method_doc:meta])* fn $name:ident(mut $this:ident $(, $arg:ident: $arg_ty:ty)*) $body:block)*
  }) => {
    $(#[$doc])*
    pub trait $trait: Sized {
      $($(#[$method_doc])* #[must_use] fn $name($this $(, $arg: $arg_ty)*) -> Self;)*
    }
    impl $trait for $ty {
      $(fn $name(mut $this $(, $arg: $arg_ty)*) -> Self $body)*
    }
  };
}

pub mod block;
pub mod list;
pub mod media;
pub mod table;

pub use block::*;
pub use list::*;
pub use media::*;
pub use table::*;

/// Converts inline content into a generated rich-text tree.
///
/// Accepts strings, `format_args!`, styled content, generated [`RichText`], and
/// arrays or vectors of these. Use [`concat()`] with [`plain`] to mix element types.
pub trait IntoRichText {
  /// Consumes the inline content, retaining owned strings and generated nodes.
  fn into_rich_text(self) -> RichText;
}

impl IntoRichText for RichText {
  fn into_rich_text(self) -> RichText {
    self
  }
}

impl<T: IntoRichText, const N: usize> IntoRichText for [T; N] {
  fn into_rich_text(self) -> RichText {
    concat(self)
  }
}

impl<T: IntoRichText> IntoRichText for Vec<T> {
  fn into_rich_text(self) -> RichText {
    concat(self)
  }
}

impl IntoRichText for &str {
  fn into_rich_text(self) -> RichText {
    self.to_owned().into_rich_text()
  }
}

impl IntoRichText for String {
  fn into_rich_text(self) -> RichText {
    types::richTextPlain { text: self }.into()
  }
}

impl IntoRichText for Arguments<'_> {
  fn into_rich_text(self) -> RichText {
    self.to_string().into_rich_text()
  }
}

/// Converts inline content to the common [`RichText`] type, preserving styles.
pub fn plain(text: impl IntoRichText) -> RichText {
  text.into_rich_text()
}

/// Joins inline trees without separators.
///
/// Empty input becomes empty plain text; a single item is returned directly.
pub fn concat(items: impl IntoIterator<Item = impl IntoRichText>) -> RichText {
  let mut items = items.into_iter().map(IntoRichText::into_rich_text);
  let Some(first) = items.next() else { return plain("") };
  let Some(second) = items.next() else { return first };
  types::richTexts { texts: [first, second].into_iter().chain(items).collect() }.into()
}

/// Collects blocks into a native rich message, ready for editing or sending.
/// Use a `Vec<InputPageBlock>` when accumulating a document incrementally.
pub fn rich(blocks: impl IntoIterator<Item = impl Into<InputPageBlock>>) -> types::inputMessageRichMessage {
  let source = types::richMessageSourceBlocks { blocks: blocks.into_iter().map(Into::into).collect() };
  let message = types::inputRichMessage { source: RichMessageSource::from(source), ..Default::default() };
  types::inputMessageRichMessage { message, ..Default::default() }
}
