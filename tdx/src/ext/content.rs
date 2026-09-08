//! Extension trait for borrowed message content inspection.

use td_types::enums::MessageContent;
use td_types::types;

/// Extension trait for inspecting message contents.
pub trait ContentExt {
  /// Borrows formatted text or caption from the message content.
  ///
  /// Preserves empty text/captions. Rich-document blocks are not flattened.
  fn text(&self) -> Option<&types::formattedText>;

  /// Borrows the caption specifically from media content.
  fn caption(&self) -> Option<&types::formattedText>;

  /// Borrows the primary media file without manual struct traversal.
  fn primary_file(&self) -> Option<&types::file>;

  /// Extracts the primary media file ID without manual struct traversal.
  fn primary_file_id(&self) -> Option<i32>;

  /// Reports whether the caption is shown above media.
  fn show_caption_above_media(&self) -> bool;
}

impl ContentExt for MessageContent {
  fn text(&self) -> Option<&types::formattedText> {
    match self {
      Self::messageText(content) => Some(&content.text),
      _ => self.caption(),
    }
  }

  fn caption(&self) -> Option<&types::formattedText> {
    match self {
      Self::messageAnimation(content) => Some(&content.caption),
      Self::messageAudio(content) => Some(&content.caption),
      Self::messageDocument(content) => Some(&content.caption),
      Self::messagePaidMedia(content) => Some(&content.caption),
      Self::messagePhoto(content) => Some(&content.caption),
      Self::messageVideo(content) => Some(&content.caption),
      Self::messageVoiceNote(content) => Some(&content.caption),
      _ => None,
    }
  }

  fn primary_file(&self) -> Option<&types::file> {
    match self {
      Self::messageAnimation(content) => Some(&content.animation.animation),
      Self::messageAudio(content) => Some(&content.audio.audio),
      Self::messageDocument(content) => Some(&content.document.document),
      Self::messagePhoto(content) => content.photo.sizes.last().map(|size| &size.photo),
      Self::messageVideo(content) => Some(&content.video.video),
      Self::messageVoiceNote(content) => Some(&content.voice_note.voice),
      Self::messageVideoNote(content) => Some(&content.video_note.video),
      Self::messageSticker(content) => Some(&content.sticker.sticker),
      _ => None,
    }
  }

  fn primary_file_id(&self) -> Option<i32> {
    self.primary_file().map(|file| file.id)
  }

  fn show_caption_above_media(&self) -> bool {
    match self {
      Self::messageAnimation(media) => media.show_caption_above_media,
      Self::messagePaidMedia(media) => media.show_caption_above_media,
      Self::messagePhoto(media) => media.show_caption_above_media,
      Self::messageVideo(media) => media.show_caption_above_media,
      _ => false,
    }
  }
}
