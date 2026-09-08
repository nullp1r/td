//! Accepted destinations for message actions.
//!
//! | Action target | Accepted values |
//! | --- | --- |
//! | [`ChatTarget`] | Chat ID, `(chat_id, Option<MessageTopic>)`, message reference |
//! | [`MessageTarget`] | `(chat_id, message_id)`, coordinates with an optional topic, message reference |
//!
//! Message references carry a snapshot's topic, keyboard and caption position.
//! Coordinate targets carry only the fields in their tuple. Consequently, text and
//! caption edits using coordinates remove the keyboard and use the default caption
//! position; edits using a message copy those values from its snapshot. Override
//! the returned request fields when a different result is intended.

use td_types::enums::{MessageTopic, ReplyMarkup};
use td_types::types;

use crate::ext::ContentExt as _;

/// Represents a chat destination with an optional topic.
pub trait ChatTarget {
  /// Returns the target chat ID.
  fn chat_id(&self) -> i64;

  /// Returns the optional topic for this chat.
  fn topic_id(&self) -> Option<MessageTopic> {
    None
  }
}

impl ChatTarget for i64 {
  fn chat_id(&self) -> i64 {
    *self
  }
}

impl ChatTarget for (i64, Option<MessageTopic>) {
  fn chat_id(&self) -> i64 {
    self.0
  }

  fn topic_id(&self) -> Option<MessageTopic> {
    self.1.clone()
  }
}

impl ChatTarget for types::message {
  fn chat_id(&self) -> i64 {
    self.chat_id
  }

  fn topic_id(&self) -> Option<MessageTopic> {
    self.topic_id.clone()
  }
}

/// Identifies a specific message within a chat for edits, replies, and deletions.
pub trait MessageTarget {
  /// Returns the target `(chat_id, message_id)` coordinates.
  fn message_target(&self) -> (i64, i64);

  /// Returns the optional topic for this message.
  fn topic_id(&self) -> Option<MessageTopic> {
    None
  }

  /// Returns existing reply markup if known from this target.
  fn reply_markup(&self) -> Option<ReplyMarkup> {
    None
  }

  /// Returns whether caption was shown above media if known from this target.
  fn show_caption_above_media(&self) -> bool {
    false
  }
}

impl MessageTarget for (i64, i64) {
  fn message_target(&self) -> (i64, i64) {
    *self
  }
}

impl MessageTarget for (i64, i64, Option<MessageTopic>) {
  fn message_target(&self) -> (i64, i64) {
    (self.0, self.1)
  }

  fn topic_id(&self) -> Option<MessageTopic> {
    self.2.clone()
  }
}

impl MessageTarget for types::message {
  fn message_target(&self) -> (i64, i64) {
    (self.chat_id, self.id)
  }

  fn topic_id(&self) -> Option<MessageTopic> {
    self.topic_id.clone()
  }

  fn reply_markup(&self) -> Option<ReplyMarkup> {
    self.reply_markup.clone()
  }

  fn show_caption_above_media(&self) -> bool {
    self.content.show_caption_above_media()
  }
}

impl ChatTarget for types::updateNewCallbackQuery {
  fn chat_id(&self) -> i64 {
    self.chat_id
  }
}

impl MessageTarget for types::updateNewCallbackQuery {
  fn message_target(&self) -> (i64, i64) {
    (self.chat_id, self.message_id)
  }
}

/// Identifies a callback query for answers, toasts, or acknowledgments.
pub trait CallbackQueryTarget {
  /// Returns the target callback query ID.
  fn callback_query_id(&self) -> i64;
}

impl CallbackQueryTarget for i64 {
  fn callback_query_id(&self) -> i64 {
    *self
  }
}

impl CallbackQueryTarget for types::updateNewCallbackQuery {
  fn callback_query_id(&self) -> i64 {
    self.id
  }
}
