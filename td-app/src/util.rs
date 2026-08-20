use td_types::{enums, types};

/// Extracts the primary active username from an optional `Usernames` structure.
pub fn primary_username(usernames: Option<&types::usernames>) -> Option<&str> {
  match usernames {
    Some(u) if let [name, ..] = &*u.active_usernames => Some(name),
    _ => None,
  }
}

/// Extracts a user ID from a `MessageSender` if it was sent by an individual user.
pub const fn extract_user_id(sender: &enums::MessageSender) -> Option<i64> {
  match sender {
    enums::MessageSender::messageSenderUser(u) => Some(u.user_id),
    _ => None,
  }
}

/// Extracts the target replied message ID from a `MessageReplyTo` structure.
pub const fn reply_message_id(reply_to: Option<&enums::MessageReplyTo>) -> Option<i64> {
  match reply_to {
    Some(enums::MessageReplyTo::messageReplyToMessage(r)) => Some(r.message_id),
    _ => None,
  }
}

/// Extracts the plain text slice from a `MessageContent` if it is a text message.
pub fn message_text(content: &enums::MessageContent) -> Option<&str> {
  match content {
    enums::MessageContent::messageText(m) => Some(&m.text.text),
    _ => None,
  }
}
