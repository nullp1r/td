//! Ordinary Telegram messages: a UTF-8 buffer with UTF-16 entity ranges.

use std::fmt::{Display, Result as FmtResult, Write};
use std::ops::{Add, AddAssign, Deref};

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
/// Strings, display values and nested styles stream into this buffer. A broken
/// `Display` implementation that returns an error causes a panic; writing into
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

  /// Appends display content, a styled span, or another text buffer.
  pub fn push(&mut self, part: impl Part) {
    part.write_to(self);
  }

  /// Records a nonempty entity around the callback’s appended content.
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

impl<T: Part> Add<T> for Text {
  type Output = Text;

  fn add(mut self, rhs: T) -> Text {
    self.push(rhs);
    self
  }
}

impl<T: Part> AddAssign<T> for Text {
  fn add_assign(&mut self, rhs: T) {
    self.push(rhs);
  }
}

impl<T: Part> Extend<T> for Text {
  fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
    for item in iter {
      self.push(item);
    }
  }
}

impl<T: Part> FromIterator<T> for Text {
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let mut text = Self::default();
    text.extend(iter);
    text
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

/// Content appended by value, preserving entities when the source is [`Text`].
pub trait Part: Sized {
  /// Writes content and any entity spans at the destination’s current offset.
  fn write_to(self, text: &mut Text);

  /// Starts a buffer, reusing owned text when available.
  fn into_text(self) -> Text {
    let mut text = Default::default();
    self.write_to(&mut text);
    text
  }
}

impl<T: Display> Part for T {
  fn write_to(self, text: &mut Text) {
    write!(text, "{self}").expect("Display returned an error although the Text writer cannot fail");
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

/// Starts a text expression with one part; appends no newline.
pub fn line(part: impl Part) -> Text {
  part.into_text()
}

/// Supplies a blank entry for [`lines`], or an empty starting buffer.
#[must_use]
pub const fn empty() -> Text {
  Text::new()
}

/// Joins parts with exactly one newline between entries, writing directly into one buffer.
///
/// Empty entries are retained; an empty iterator produces empty text. The first
/// buffer is reused and subsequent entity offsets include the separators.
pub fn lines(items: impl IntoIterator<Item = impl Part>) -> Text {
  let mut iter = items.into_iter();
  let Some(mut first) = iter.next().map(line) else { return Default::default() };
  for next in iter {
    first.push_str("\n");
    first.push(next);
  }
  first
}
