//! Constructors for message reply references and quotes.

use td_types::enums::InputMessageReplyTo;
use td_types::types;

/// Constructs a reply reference to a message in the same chat.
pub fn same_chat(message_id: i64, quote: Option<types::inputTextQuote>) -> InputMessageReplyTo {
  types::inputMessageReplyToMessage { message_id, quote, ..Default::default() }.into()
}

/// Constructs a reply reference to a message in another chat.
pub fn external(chat_id: i64, message_id: i64, quote: Option<types::inputTextQuote>) -> InputMessageReplyTo {
  types::inputMessageReplyToExternalMessage { chat_id, message_id, quote, ..Default::default() }.into()
}

/// Constructs a reply quote at a UTF-16 code unit offset in the source message.
///
/// Entity types are preserved as supplied. `TDLib` validates entity support during request execution.
pub fn quote(text: impl Into<types::formattedText>, position: i32) -> types::inputTextQuote {
  types::inputTextQuote { text: text.into(), position }
}
