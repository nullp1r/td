//! Ordinary Telegram messages: a UTF-8 buffer with UTF-16 entity ranges.

use std::fmt::{Arguments, Result as FmtResult, Write};
use std::ops::Deref;

use td_types::enums::{InputMessageContent, TextEntityType};
use td_types::types;

use crate::util::Utf16 as _;

/// Owned text and its entity spans.
///
/// Appends preserve insertion order and shift incoming entity offsets. Nested
/// entities are recorded inner-first; empty spans are omitted. Lengths and offsets
/// must fit in `i32`. Entity validity and permitted nesting are checked by `TDLib`.
/// Borrow the plain string through `Deref` or `AsRef<str>`.
///
/// Parts stream directly into this buffer. Tuples compose heterogeneous parts;
/// arrays and vectors compose homogeneous parts. A broken `Display`
/// implementation used by a supported scalar causes a panic because writing into
/// the buffer itself is infallible.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Text {
  buffer: String,
  entities: Vec<types::textEntity>,
  utf16_len: i32,
}

impl Text {
  /// Starts with no text or entities and allocates on demand.
  #[must_use]
  pub const fn new() -> Self {
    Self { buffer: String::new(), entities: Vec::new(), utf16_len: 0 }
  }

  /// Borrows spans in insertion order, measured in UTF-16 code units.
  pub fn entities(&self) -> &[types::textEntity] {
    &self.entities
  }

  /// Appends unstyled text and advances the UTF-16 position.
  pub fn push_str(&mut self, text: &str) {
    self.buffer.push_str(text);
    self.utf16_len += text.len_utf16();
  }

  /// Appends a scalar, styled span, tuple/collection, or another text buffer.
  pub fn push(&mut self, part: impl Part) {
    part.write_to(self);
  }

  /// Records a nonempty entity around the callback's appended content.
  ///
  /// The callback must only append; replacing or clearing the buffer invalidates
  /// the starting offset. Nested calls are supported.
  pub fn entity(&mut self, kind: impl Into<TextEntityType>, write: impl FnOnce(&mut Self)) {
    let offset = self.utf16_len;
    write(self);
    let length = self.utf16_len - offset;
    if length > 0 {
      self.entities.push(types::textEntity { offset, length, r#type: kind.into() });
    }
  }

  fn append(&mut self, buffer: &str, utf16_len: i32, entities: impl Iterator<Item = types::textEntity>) {
    let offset = self.utf16_len;
    self.buffer.push_str(buffer);
    self.utf16_len += utf16_len;
    self.entities.extend(entities.map(|mut entity| {
      entity.offset += offset;
      entity
    }));
  }
}

impl Deref for Text {
  type Target = str;

  fn deref(&self) -> &str {
    &self.buffer
  }
}

impl Write for Text {
  fn write_str(&mut self, s: &str) -> FmtResult {
    self.push_str(s);
    Ok(())
  }

  fn write_char(&mut self, c: char) -> FmtResult {
    self.buffer.push(c);
    self.utf16_len += c.len_utf16() as i32;
    Ok(())
  }
}

impl AsRef<str> for Text {
  fn as_ref(&self) -> &str {
    &self.buffer
  }
}

impl From<Text> for types::formattedText {
  fn from(value: Text) -> Self {
    let Text { buffer: text, entities, .. } = value;
    Self { text, entities }
  }
}

impl From<Text> for Option<types::formattedText> {
  fn from(value: Text) -> Self {
    Some(value.into())
  }
}

impl From<&str> for Text {
  fn from(value: &str) -> Self {
    value.to_owned().into()
  }
}

impl From<String> for Text {
  fn from(value: String) -> Self {
    let utf16_len = value.len_utf16();
    Self { buffer: value, utf16_len, ..Default::default() }
  }
}

impl From<types::formattedText> for Text {
  fn from(value: types::formattedText) -> Self {
    let types::formattedText { text: buffer, entities } = value;
    let utf16_len = buffer.len_utf16();
    Self { buffer, entities, utf16_len }
  }
}

impl From<Text> for InputMessageContent {
  fn from(value: Text) -> Self {
    types::inputMessageText { text: value.into(), ..Default::default() }.into()
  }
}

/// Enables `.text()` on strings, styled parts and generated formatted text.
pub trait TextExt: Into<Text> {
  /// Consumes the value, retaining existing text and entity allocations where possible.
  fn text(self) -> Text {
    self.into()
  }
}

impl<T: Into<Text>> TextExt for T {}

/// Content that can stream into one ordinary Telegram text buffer.
///
/// Tuples concatenate heterogeneous parts without intermediate [`Text`] values.
/// Arrays and vectors do the same for homogeneous parts.
pub trait Part: Sized {
  /// Writes content and any entity spans at the destination's current offset.
  fn write_to(self, text: &mut Text);

  /// Starts a buffer, reusing owned text when available.
  fn into_text(self) -> Text {
    let mut text = Default::default();
    self.write_to(&mut text);
    text
  }
}

impl Part for Text {
  fn into_text(self) -> Text {
    self
  }

  fn write_to(self, text: &mut Text) {
    text.append(&self.buffer, self.utf16_len, self.entities.into_iter());
  }
}

impl Part for &Text {
  fn write_to(self, text: &mut Text) {
    text.append(&self.buffer, self.utf16_len, self.entities.iter().cloned());
  }
}

impl Part for &str {
  fn write_to(self, text: &mut Text) {
    text.push_str(self);
  }
}

impl Part for String {
  fn into_text(self) -> Text {
    self.into()
  }

  fn write_to(self, text: &mut Text) {
    text.push_str(&self);
  }
}

impl Part for &String {
  fn write_to(self, text: &mut Text) {
    text.push_str(self);
  }
}

impl Part for char {
  fn write_to(self, text: &mut Text) {
    text.write_char(self).expect("writing a char into Text cannot fail");
  }
}

impl Part for Arguments<'_> {
  fn write_to(self, text: &mut Text) {
    text.write_fmt(self).expect("formatting into Text cannot fail");
  }
}

macro_rules! display_parts(($($ty:ty),+) => {
  $(
    impl Part for $ty {
      fn write_to(self, text: &mut Text) {
        write!(text, "{self}").expect("Display returned an error although the Text writer cannot fail");
      }
    }
  )+
});

display_parts!(bool, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

impl<T: Part, const N: usize> Part for [T; N] {
  fn write_to(self, text: &mut Text) {
    for part in self {
      part.write_to(text);
    }
  }
}

impl<T: Part> Part for Vec<T> {
  fn write_to(self, text: &mut Text) {
    for part in self {
      part.write_to(text);
    }
  }
}

/// A fixed or dynamic sequence of lines that can stream into one [`Text`] buffer.
pub trait Lines: Sized {
  /// Writes each line with exactly one newline between adjacent entries.
  fn write_lines_to(self, text: &mut Text);

  /// Creates one buffer for the complete multiline result.
  fn into_lines(self) -> Text {
    let mut text = Text::new();
    self.write_lines_to(&mut text);
    text
  }
}

fn write_line(text: &mut Text, first: &mut bool, line: impl Part) {
  if *first {
    *first = false;
  } else {
    text.push_str("\n");
  }
  line.write_to(text);
}

impl<T: Part, const N: usize> Lines for [T; N] {
  fn write_lines_to(self, text: &mut Text) {
    let mut first = true;
    for line in self {
      write_line(text, &mut first, line);
    }
  }
}

impl<T: Part> Lines for Vec<T> {
  fn write_lines_to(self, text: &mut Text) {
    let mut first = true;
    for line in self {
      write_line(text, &mut first, line);
    }
  }
}

macro_rules! tuple_composition {
  ($($ty:ident $value:ident),+ $(,)?) => {
    impl<$($ty: Part),+> Part for ($($ty,)+) {
      fn write_to(self, text: &mut Text) {
        let ($($value,)+) = self;
        $($value.write_to(text);)+
      }
    }

    impl<$($ty: Part),+> Lines for ($($ty,)+) {
      fn write_lines_to(self, text: &mut Text) {
        let ($($value,)+) = self;
        let mut first = true;
        $(write_line(text, &mut first, $value);)+
      }
    }
  };
}

tuple_impls!(tuple_composition);

/// Composes one line directly into a single buffer; appends no newline.
pub fn line(part: impl Part) -> Text {
  part.into_text()
}

/// Joins fixed or dynamic lines with exactly one newline between entries.
///
/// Tuples permit heterogeneous line expressions such as
/// `lines((bold("Title"), ("Count: ", 3)))`; arrays and vectors cover homogeneous
/// collections. All lines stream directly into one destination buffer.
pub fn lines(items: impl Lines) -> Text {
  items.into_lines()
}
