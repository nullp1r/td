//! Message edit requests. Execute with `Client::send`, not send tracking.
//!
//! A message object supplies its snapshot's keyboard and caption position. A tuple
//! supplies coordinates only: text/caption edits then remove the keyboard and use
//! the default caption position. Override returned fields to choose these explicitly.
//! A stale message snapshot can restore an older keyboard.

use std::fmt::Display;

use td_types::enums::{InputMessageContent, ReplyMarkup};
use td_types::fns;

use crate::compose::content;
use crate::format::Text;
use crate::target::MessageTarget;

/// Edits a message's text, preserving inline markup if known from the target.
pub fn text(target: &impl MessageTarget, text: impl Into<Text>) -> fns::editMessageText {
  let (chat_id, message_id) = target.message_target();
  fns::editMessageText {
    chat_id, //.
    message_id,
    reply_markup: target.reply_markup(),
    input_message_content: content::text(text).into(),
  }
}

/// Edits a message with rich message content, preserving inline markup if known from the target.
pub fn rich(target: &impl MessageTarget, content: impl Into<InputMessageContent>) -> fns::editMessageText {
  let (chat_id, message_id) = target.message_target();
  fns::editMessageText {
    chat_id, //.
    message_id,
    reply_markup: target.reply_markup(),
    input_message_content: content.into(),
  }
}

/// Edits a message with HTML-formatted rich content, preserving inline markup if known from the target.
pub fn html(target: &impl MessageTarget, html: impl Display) -> fns::editMessageText {
  rich(target, content::html(html))
}

/// Edits a media message's caption, preserving markup and caption position if known.
pub fn caption(target: &impl MessageTarget, caption: impl Into<Text>) -> fns::editMessageCaption {
  let (chat_id, message_id) = target.message_target();
  fns::editMessageCaption {
    chat_id,
    message_id,
    reply_markup: target.reply_markup(),
    caption: Some(caption.into().into()),
    show_caption_above_media: target.show_caption_above_media(),
  }
}

/// Edits a message's media content, preserving known inline markup.
pub fn media(target: &impl MessageTarget, content: impl Into<InputMessageContent>) -> fns::editMessageMedia {
  let (chat_id, message_id) = target.message_target();
  fns::editMessageMedia {
    chat_id, //.
    message_id,
    reply_markup: target.reply_markup(),
    input_message_content: content.into(),
  }
}

/// Edits or removes a message's inline keyboard markup.
pub fn markup(target: &impl MessageTarget, reply_markup: Option<ReplyMarkup>) -> fns::editMessageReplyMarkup {
  let (chat_id, message_id) = target.message_target();
  fns::editMessageReplyMarkup { chat_id, message_id, reply_markup }
}
