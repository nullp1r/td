//! Extension trait for borrowed message inspection.

use td_types::enums::MessageReplyTo;
use td_types::types;

use super::content::ContentExt as _;
use super::sender::MessageSenderExt as _;

/// Extension trait for inspecting message contents and metadata.
pub trait MessageExt {
  /// Returns the sender's user ID or chat ID.
  fn sender_id(&self) -> i64;

  /// Returns the ID of the replied-to message, if replying to a message.
  fn replied_message_id(&self) -> Option<i64>;

  /// Reports whether this message replies to another message.
  fn is_reply(&self) -> bool {
    self.replied_message_id().is_some()
  }

  /// Borrows formatted text or caption from the message.
  fn text(&self) -> Option<&types::formattedText>;

  /// Borrows the caption specifically from a media message.
  fn caption(&self) -> Option<&types::formattedText>;

  /// Borrows the primary media file from the message.
  fn primary_file(&self) -> Option<&types::file>;

  /// Extracts the primary media file ID from the message.
  fn primary_file_id(&self) -> Option<i32>;

  /// Reports whether the caption is shown above media.
  fn show_caption_above_media(&self) -> bool;
}

impl MessageExt for types::message {
  fn sender_id(&self) -> i64 {
    self.sender_id.id()
  }

  fn replied_message_id(&self) -> Option<i64> {
    match &self.reply_to {
      Some(MessageReplyTo::messageReplyToMessage(reply)) if reply.message_id != 0 => Some(reply.message_id),
      _ => None,
    }
  }

  fn text(&self) -> Option<&types::formattedText> {
    self.content.text()
  }

  fn caption(&self) -> Option<&types::formattedText> {
    self.content.caption()
  }

  fn primary_file(&self) -> Option<&types::file> {
    self.content.primary_file()
  }

  fn primary_file_id(&self) -> Option<i32> {
    self.content.primary_file_id()
  }

  fn show_caption_above_media(&self) -> bool {
    self.content.show_caption_above_media()
  }
}
