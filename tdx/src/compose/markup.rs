//! Inline keyboards and buttons without container boilerplate.

use std::fmt::{Arguments, Display};
use std::io::Write as _;

use td_types::enums::{ButtonStyle, ReplyMarkup, TargetChat};
use td_types::types;

/// Values convertible into callback button payload bytes.
pub trait IntoCallbackData {
  /// Converts the value into raw callback payload bytes.
  fn into_callback_data(self) -> Vec<u8>;
}

impl<T: AsRef<[u8]> + ?Sized> IntoCallbackData for &T {
  fn into_callback_data(self) -> Vec<u8> {
    self.as_ref().to_vec()
  }
}

impl IntoCallbackData for Vec<u8> {
  fn into_callback_data(self) -> Vec<u8> {
    self
  }
}

impl IntoCallbackData for String {
  fn into_callback_data(self) -> Vec<u8> {
    self.into_bytes()
  }
}

impl IntoCallbackData for Arguments<'_> {
  fn into_callback_data(self) -> Vec<u8> {
    let mut buf = Vec::default();
    let _ = buf.write_fmt(self);
    buf
  }
}

/// Constructs an inline keyboard from nested iterators, arrays, or vectors.
///
/// # Examples
///
/// ```
/// use tdx::compose::markup;
///
/// // Clean array literal syntax:
/// let keyboard = markup::inline([
///   [markup::url("Docs", "https://example.com")],
///   [markup::callback("Ping", b"ping")],
/// ]);
/// ```
pub fn inline<R, B>(rows: R) -> ReplyMarkup
where
  R: IntoIterator<Item = B>,
  B: IntoIterator<Item = types::inlineKeyboardButton>,
{
  let rows = rows.into_iter().map(|row| row.into_iter().collect()).collect();
  let markup = types::replyMarkupInlineKeyboard { rows, ..Default::default() };
  markup.into()
}

/// Constructs an inline keyboard where each button occupies its own row.
pub fn column<I>(buttons: I) -> ReplyMarkup
where
  I: IntoIterator<Item = types::inlineKeyboardButton>,
{
  inline(buttons.into_iter().map(|btn| [btn]))
}

/// Constructs an inline button that opens a URL.
pub fn url(text: impl Display, url: impl Into<String>) -> types::inlineKeyboardButton {
  let (text, url) = (text.to_string(), url.into());
  let style = ButtonStyle::buttonStyleDefault;
  let r#type = types::inlineKeyboardButtonTypeUrl { url }.into();
  types::inlineKeyboardButton { text, style, r#type, ..Default::default() }
}

/// Constructs an inline button that sends callback data.
pub fn callback(text: impl Display, data: impl IntoCallbackData) -> types::inlineKeyboardButton {
  callback_with_style(text, data, ButtonStyle::buttonStyleDefault)
}

/// Constructs a dark-blue callback button for a primary action.
pub fn primary(text: impl Display, data: impl IntoCallbackData) -> types::inlineKeyboardButton {
  callback_with_style(text, data, ButtonStyle::buttonStylePrimary)
}

/// Constructs a green callback button for a successful or affirmative action.
pub fn success(text: impl Display, data: impl IntoCallbackData) -> types::inlineKeyboardButton {
  callback_with_style(text, data, ButtonStyle::buttonStyleSuccess)
}

/// Constructs a red callback button for a destructive or abort action.
pub fn danger(text: impl Display, data: impl IntoCallbackData) -> types::inlineKeyboardButton {
  callback_with_style(text, data, ButtonStyle::buttonStyleDanger)
}

fn callback_with_style(text: impl Display, data: impl IntoCallbackData, style: ButtonStyle) -> types::inlineKeyboardButton {
  let (text, data) = (text.to_string(), data.into_callback_data());
  let r#type = types::inlineKeyboardButtonTypeCallback { data }.into();
  types::inlineKeyboardButton { text, style, r#type, ..Default::default() }
}

/// Constructs an inline button that prompts an inline query in the current chat.
pub fn switch_inline(text: impl Display, query: impl Into<String>) -> types::inlineKeyboardButton {
  let (text, query) = (text.to_string(), query.into());
  let style = ButtonStyle::buttonStyleDefault;
  let r#type = types::inlineKeyboardButtonTypeSwitchInline { query, target_chat: TargetChat::targetChatCurrent }.into();
  types::inlineKeyboardButton { text, style, r#type, ..Default::default() }
}

/// Constructs an inline button that prompts an inline query with a target chat.
pub fn switch_inline_target(
  text: impl Display, //.
  query: impl Into<String>,
  target_chat: impl Into<TargetChat>,
) -> types::inlineKeyboardButton {
  let (text, query, target_chat) = (text.to_string(), query.into(), target_chat.into());
  let style = ButtonStyle::buttonStyleDefault;
  let r#type = types::inlineKeyboardButtonTypeSwitchInline { query, target_chat }.into();
  types::inlineKeyboardButton { text, style, r#type, ..Default::default() }
}

/// Constructs an inline button that launches a Telegram Web App.
pub fn web_app(text: impl Display, url: impl Into<String>) -> types::inlineKeyboardButton {
  let (text, url) = (text.to_string(), url.into());
  let style = ButtonStyle::buttonStyleDefault;
  let r#type = types::inlineKeyboardButtonTypeWebApp { url }.into();
  types::inlineKeyboardButton { text, style, r#type, ..Default::default() }
}
