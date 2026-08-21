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

/// Extracts the underlying downloadable `file` from any media message content.
pub fn extract_media_file(content: &enums::MessageContent) -> Option<&types::file> {
  match content {
    enums::MessageContent::messageAnimation(m) => Some(&m.animation.animation),
    enums::MessageContent::messageAudio(m) => Some(&m.audio.audio),
    enums::MessageContent::messageDocument(m) => Some(&m.document.document),
    enums::MessageContent::messagePhoto(m) => m.photo.sizes.last().map(|s| &s.photo),
    enums::MessageContent::messageSticker(m) => Some(&m.sticker.sticker),
    enums::MessageContent::messageVideo(m) => Some(&m.video.video),
    enums::MessageContent::messageVideoNote(m) => Some(&m.video_note.video),
    enums::MessageContent::messageVoiceNote(m) => Some(&m.voice_note.voice),
    _ => None,
  }
}

/// Extracts the caption text from a media message if present.
pub fn message_caption(content: &enums::MessageContent) -> Option<&str> {
  let text = match content {
    enums::MessageContent::messageAnimation(m) => &m.caption.text,
    enums::MessageContent::messageAudio(m) => &m.caption.text,
    enums::MessageContent::messageDocument(m) => &m.caption.text,
    enums::MessageContent::messagePhoto(m) => &m.caption.text,
    enums::MessageContent::messageVideo(m) => &m.caption.text,
    enums::MessageContent::messageVoiceNote(m) => &m.caption.text,
    _ => return None,
  };

  if text.is_empty() { None } else { Some(text) }
}
