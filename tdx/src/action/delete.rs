//! Message deletion requests.

use td_types::fns;

use crate::target::MessageTarget;

/// Deletes a specific message.
pub fn message(target: &impl MessageTarget, revoke: bool) -> fns::deleteMessages {
  let (chat_id, message_id) = target.message_target();
  messages(chat_id, [message_id], revoke)
}

/// Deletes multiple messages in a chat.
pub fn messages(chat_id: i64, message_ids: impl IntoIterator<Item = i64>, revoke: bool) -> fns::deleteMessages {
  fns::deleteMessages { chat_id, message_ids: message_ids.into_iter().collect(), revoke }
}
