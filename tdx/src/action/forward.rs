//! Forwarding and copying requests.

use td_types::fns;

/// Forwards messages retaining their original attribution.
pub fn messages(
  from_chat_id: i64, //.
  message_ids: impl IntoIterator<Item = i64>,
  to_chat_id: i64,
) -> fns::forwardMessages {
  fns::forwardMessages {
    from_chat_id, //.
    message_ids: message_ids.into_iter().collect(),
    chat_id: to_chat_id,
    ..Default::default()
  }
}

/// Copies messages without forward attribution.
pub fn copy(from_chat_id: i64, message_ids: impl IntoIterator<Item = i64>, to_chat_id: i64) -> fns::forwardMessages {
  fns::forwardMessages { send_copy: true, ..messages(from_chat_id, message_ids, to_chat_id) }
}
