//! Extension trait for inspecting message sender identities.

use td_types::enums::MessageSender;

/// Extension trait for inspecting message sender identities.
pub trait MessageSenderExt {
  /// Returns the sender's ID, which is either a user ID or a chat ID.
  fn id(&self) -> i64;

  /// Returns the user ID if the sender is a user, or `None` if it is a chat.
  fn user_id(&self) -> Option<i64>;

  /// Returns the chat ID if the sender is a chat, or `None` if it is a user.
  fn chat_id(&self) -> Option<i64>;
}

impl MessageSenderExt for MessageSender {
  fn id(&self) -> i64 {
    match self {
      Self::messageSenderChat(c) => c.chat_id,
      Self::messageSenderUser(u) => u.user_id,
    }
  }

  fn user_id(&self) -> Option<i64> {
    match self {
      Self::messageSenderChat(_) => None,
      Self::messageSenderUser(u) => Some(u.user_id),
    }
  }

  fn chat_id(&self) -> Option<i64> {
    match self {
      Self::messageSenderChat(c) => Some(c.chat_id),
      Self::messageSenderUser(_) => None,
    }
  }
}
