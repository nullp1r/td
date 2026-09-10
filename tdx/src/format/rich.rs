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

/// Inline content that can be converted into a generated rich-text tree.
///
/// Tuples concatenate heterogeneous parts; arrays and vectors concatenate
/// homogeneous parts. Nested compositions are flattened into one `richTexts`
/// node where possible. Styled values remain explicit tree nodes.
pub trait IntoRichText: Sized {
  /// Consumes the content, retaining owned strings and generated nodes.
  fn into_rich_text(self) -> RichText;

  /// Appends this value's top-level nodes to a composition buffer.
  #[doc(hidden)]
  fn append_to(self, texts: &mut Vec<RichText>) {
    append_node(texts, self.into_rich_text());
  }
}

fn append_node(texts: &mut Vec<RichText>, text: RichText) {
  match text {
    RichText::richTexts(group) => texts.extend(group.texts),
    text => texts.push(text),
  }
}

fn finish(texts: Vec<RichText>) -> RichText {
  match <[_; 1]>::try_from(texts) {
    Ok([text]) => text,
    Err(texts) if texts.is_empty() => types::richTextPlain { text: String::new() }.into(),
    Err(texts) => types::richTexts { texts }.into(),
  }
}

impl IntoRichText for RichText {
  fn into_rich_text(self) -> RichText {
    self
  }
}

impl IntoRichText for &str {
  fn into_rich_text(self) -> RichText {
    types::richTextPlain { text: self.into() }.into()
  }
}

impl IntoRichText for String {
  fn into_rich_text(self) -> RichText {
    types::richTextPlain { text: self }.into()
  }
}

impl IntoRichText for &String {
  fn into_rich_text(self) -> RichText {
    self.as_str().into_rich_text()
  }
}

impl IntoRichText for char {
  fn into_rich_text(self) -> RichText {
    self.to_string().into_rich_text()
  }
}

impl IntoRichText for Arguments<'_> {
  fn into_rich_text(self) -> RichText {
    self.to_string().into_rich_text()
  }
}

macro_rules! display_rich_parts(($($ty:ty),+) => {
  $(
    impl IntoRichText for $ty {
      fn into_rich_text(self) -> RichText {
        self.to_string().into_rich_text()
      }
    }
  )+
});

display_rich_parts!(bool, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

impl<T: IntoRichText, const N: usize> IntoRichText for [T; N] {
  fn into_rich_text(self) -> RichText {
    let mut texts = Vec::with_capacity(N);
    self.append_to(&mut texts);
    finish(texts)
  }

  fn append_to(self, texts: &mut Vec<RichText>) {
    for text in self {
      text.append_to(texts);
    }
  }
}

impl<T: IntoRichText> IntoRichText for Vec<T> {
  fn into_rich_text(self) -> RichText {
    let mut texts = Vec::with_capacity(self.len());
    self.append_to(&mut texts);
    finish(texts)
  }

  fn append_to(self, texts: &mut Vec<RichText>) {
    for text in self {
      text.append_to(texts);
    }
  }
}

macro_rules! rich_tuple(($($ty:ident $value:ident),+) => {
  impl<$($ty: IntoRichText),+> IntoRichText for ($($ty,)+) {
    fn into_rich_text(self) -> RichText {
      let mut texts = Vec::new();
      self.append_to(&mut texts);
      finish(texts)
    }

    fn append_to(self, texts: &mut Vec<RichText>) {
      let ($($value,)+) = self;
      $($value.append_to(texts);)+
    }
  }
});

tuple_impls!(rich_tuple);

/// Collects blocks into a native rich message, ready for editing or sending.
/// Use a `Vec<InputPageBlock>` when accumulating a document incrementally.
pub fn rich(blocks: impl IntoIterator<Item = impl Into<InputPageBlock>>) -> types::inputMessageRichMessage {
  let source = types::richMessageSourceBlocks { blocks: blocks.into_iter().map(Into::into).collect() };
  let message = types::inputRichMessage { source: RichMessageSource::from(source), ..Default::default() };
  types::inputMessageRichMessage { message, ..Default::default() }
}
