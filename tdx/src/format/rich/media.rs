//! Rich media blocks with default dimensions and empty captions.
//!
//! These constructors do no I/O. Use generated payloads directly when additional
//! media metadata, captions or sizing are needed.

use td_types::enums::{InputFile, InputPageBlock};
use td_types::types;

/// Embeds an image file with default photo metadata.
pub fn photo(photo: impl Into<InputFile>) -> InputPageBlock {
  let photo = types::inputPhoto { photo: photo.into(), ..Default::default() };
  types::inputPageBlockPhoto { photo, ..Default::default() }.into()
}

/// Embeds a video with streaming enabled.
pub fn video(video: impl Into<InputFile>) -> InputPageBlock {
  let video = types::inputVideo { video: video.into(), supports_streaming: true, ..Default::default() };
  types::inputPageBlockVideo { video, ..Default::default() }.into()
}

/// Embeds an animation file.
pub fn animation(animation: impl Into<InputFile>) -> InputPageBlock {
  let animation = types::inputAnimation { animation: animation.into(), ..Default::default() };
  types::inputPageBlockAnimation { animation, ..Default::default() }.into()
}

/// Embeds an audio file without title or performer metadata.
pub fn audio(audio: impl Into<InputFile>) -> InputPageBlock {
  let audio = types::inputAudio { audio: audio.into(), ..Default::default() };
  types::inputPageBlockAudio { audio, ..Default::default() }.into()
}

/// Embeds a downloadable document.
pub fn document(document: impl Into<InputFile>) -> InputPageBlock {
  let document = types::inputDocument { document: document.into(), ..Default::default() };
  types::inputPageBlockDocument { document, ..Default::default() }.into()
}

/// Embeds a voice recording without duration or waveform metadata.
pub fn voice_note(voice_note: impl Into<InputFile>) -> InputPageBlock {
  let voice_note = types::inputVoiceNote { voice_note: voice_note.into(), ..Default::default() };
  types::inputPageBlockVoiceNote { voice_note, ..Default::default() }.into()
}

/// Arranges media blocks into a collage in their supplied order.
pub fn collage(blocks: impl IntoIterator<Item = impl Into<InputPageBlock>>) -> InputPageBlock {
  let blocks = blocks.into_iter().map(Into::into).collect();
  types::inputPageBlockCollage { blocks, ..Default::default() }.into()
}

/// Arranges media blocks into a slideshow in their supplied order.
pub fn slideshow(blocks: impl IntoIterator<Item = impl Into<InputPageBlock>>) -> InputPageBlock {
  let blocks = blocks.into_iter().map(Into::into).collect();
  types::inputPageBlockSlideshow { blocks, ..Default::default() }.into()
}

/// Embeds a map centered on latitude/longitude with the supplied zoom.
pub fn map([latitude, longitude]: [f64; 2], zoom: i32) -> InputPageBlock {
  let location = types::location { latitude, longitude, ..Default::default() };
  types::inputPageBlockMap { location, zoom, ..Default::default() }.into()
}
