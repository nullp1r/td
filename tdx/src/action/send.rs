//! Outgoing message requests.

use td_types::enums::InputMessageContent;
use td_types::{fns, types};

use crate::compose::reply;
use crate::target::{ChatTarget, MessageTarget};

/// Constructs a message request addressed to a chat.
pub fn message(chat_id: i64, content: impl Into<InputMessageContent>) -> fns::sendMessage {
  fns::sendMessage { chat_id, input_message_content: content.into(), ..Default::default() }
}

/// Constructs a message addressed to a chat and optional topic, without a reply reference.
pub fn respond(target: &impl ChatTarget, content: impl Into<InputMessageContent>) -> fns::sendMessage {
  fns::sendMessage {
    chat_id: target.chat_id(), //.
    topic_id: target.topic_id(),
    input_message_content: content.into(),
    ..Default::default()
  }
}

/// Constructs a callback-triggered ephemeral message visible only to the user who pressed the button.
pub fn ephemeral(target: &types::updateNewCallbackQuery, content: impl Into<InputMessageContent>) -> fns::sendEphemeralMessage {
  fns::sendEphemeralMessage {
    chat_id: target.chat_id,
    receiver_user_id: target.sender_user_id,
    callback_query_id: target.id,
    input_message_content: content.into(),
    ..Default::default()
  }
}

/// Constructs an ephemeral reply to an incoming ephemeral bot command or message.
pub fn ephemeral_reply(target: &impl MessageTarget, receiver_user_id: i64, content: impl Into<InputMessageContent>) -> fns::sendEphemeralMessage {
  let (chat_id, message_id) = target.message_target();
  fns::sendEphemeralMessage {
    chat_id,
    topic_id: target.topic_id(),
    receiver_user_id,
    reply_to: Some(reply::same_chat(message_id, None)),
    input_message_content: content.into(),
    ..Default::default()
  }
}

/// Constructs a reply to a message or coordinates in the same chat and topic.
pub fn reply(target: &impl MessageTarget, content: impl Into<InputMessageContent>) -> fns::sendMessage {
  let (chat_id, message_id) = target.message_target();
  fns::sendMessage {
    chat_id,
    topic_id: target.topic_id(),
    reply_to: Some(reply::same_chat(message_id, None)),
    input_message_content: content.into(),
    ..Default::default()
  }
}

/// Constructs a reply with an explicit quote in the same chat and topic.
pub fn reply_quote(
  target: &impl MessageTarget, //.
  quote: impl Into<types::inputTextQuote>,
  content: impl Into<InputMessageContent>,
) -> fns::sendMessage {
  let (chat_id, message_id) = target.message_target();
  fns::sendMessage {
    chat_id,
    topic_id: target.topic_id(),
    reply_to: Some(reply::same_chat(message_id, Some(quote.into()))),
    input_message_content: content.into(),
    ..Default::default()
  }
}

/// Constructs a `sendMessageAlbum` request for media items.
pub fn album(chat_id: i64, contents: impl IntoIterator<Item = InputMessageContent>) -> fns::sendMessageAlbum {
  fns::sendMessageAlbum { chat_id, input_message_contents: contents.into_iter().collect(), ..Default::default() }
}
