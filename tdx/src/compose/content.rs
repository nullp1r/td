//! Uniform constructors for outgoing message content payloads.

use std::fmt::Display;

use td_types::enums::{InputFile, RichMessageSource};
use td_types::types;

use crate::format::Text;

/// Creates editable text content from a string or formatted text.
pub fn text(text: impl Into<Text>) -> types::inputMessageText {
  let text = text.into().into();
  types::inputMessageText { text, ..Default::default() }
}

/// Constructs a rich message from an HTML, Markdown or block source.
pub fn rich(source: impl Into<RichMessageSource>) -> types::inputMessageRichMessage {
  let source = source.into();
  let message = types::inputRichMessage { source, ..Default::default() };
  types::inputMessageRichMessage { message, clear_draft: false }
}

/// Constructs a rich HTML message.
pub fn html(html: impl Display) -> types::inputMessageRichMessage {
  rich(types::richMessageSourceHtml { text: html.to_string(), ..Default::default() })
}

/// Constructs a rich Markdown message.
pub fn markdown(md: impl Display) -> types::inputMessageRichMessage {
  rich(types::richMessageSourceMarkdown { text: md.to_string(), ..Default::default() })
}

/// Constructs a photo message with dimensions and an optional caption.
pub fn photo(
  photo: impl Into<InputFile>, //.
  [width, height]: [i32; 2],
  caption: Option<types::formattedText>,
) -> types::inputMessagePhoto {
  let photo = photo.into();
  let photo = types::inputPhoto { photo, width, height, ..Default::default() };
  types::inputMessagePhoto { photo, caption, ..Default::default() }
}

/// Constructs a document message with an optional thumbnail and caption.
///
/// Automatic content-type detection is disabled by default, so documents are
/// sent as files unless the returned payload is changed.
pub fn document(
  document: impl Into<InputFile>,
  thumbnail: Option<types::inputThumbnail>,
  caption: Option<types::formattedText>,
) -> types::inputMessageDocument {
  let document = document.into();
  let document = types::inputDocument { document, thumbnail, disable_content_type_detection: true };
  types::inputMessageDocument { document, caption }
}

/// Constructs a video message with an optional thumbnail and caption.
///
/// `duration` is in seconds and dimensions are `[width, height]` in pixels.
pub fn video(
  video: impl Into<InputFile>,
  thumbnail: Option<types::inputThumbnail>,
  duration: i32,
  [width, height]: [i32; 2],
  caption: Option<types::formattedText>,
) -> types::inputMessageVideo {
  let video = video.into();
  let video = types::inputVideo { video, thumbnail, duration, width, height, ..Default::default() };
  types::inputMessageVideo { video, caption, ..Default::default() }
}

/// Constructs an animation (GIF / soundless video) message with an optional caption.
pub fn animation(
  animation: impl Into<InputFile>,
  thumbnail: Option<types::inputThumbnail>,
  duration: i32,
  [width, height]: [i32; 2],
  caption: Option<types::formattedText>,
) -> types::inputMessageAnimation {
  let animation = animation.into();
  let animation = types::inputAnimation { animation, thumbnail, duration, width, height, ..Default::default() };
  types::inputMessageAnimation { animation, caption, ..Default::default() }
}

/// Constructs an audio message with an optional caption.
pub fn audio(
  audio: impl Into<InputFile>,
  album_cover_thumbnail: Option<types::inputThumbnail>,
  duration: i32,
  title: impl Into<String>,
  performer: impl Into<String>,
  caption: Option<types::formattedText>,
) -> types::inputMessageAudio {
  let (audio, title, performer) = (audio.into(), title.into(), performer.into());
  let audio = types::inputAudio { audio, album_cover_thumbnail, duration, title, performer };
  types::inputMessageAudio { audio, caption }
}
