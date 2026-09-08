//! Local markup parsing through synchronous `TDLib` execution; no session is needed.

use td_types::enums::{FormattedText, TextParseMode};
use td_types::{fns, types};

use crate::client;

/// Applies `TDLib`’s `parseMarkdown` rules to plain input.
///
/// This is the native Markdown convenience parser, not `MarkdownV2` or a
/// general-purpose Markdown renderer. Returns a generated value; use `.text()`
/// to append more content through the text builder.
pub fn markdown(input: impl Into<String>) -> client::Result<types::formattedText> {
  let req = fns::parseMarkdown { text: types::formattedText { text: input.into(), ..Default::default() } };
  let FormattedText::formattedText(ft) = client::execute(&req)?;
  Ok(ft)
}

/// Parses Telegram HTML into text and entity spans.
///
/// Unsupported or malformed markup is returned as a native parsing error.
pub fn html(input: impl Into<String>) -> client::Result<types::formattedText> {
  let req = fns::parseTextEntities { text: input.into(), parse_mode: TextParseMode::textParseModeHTML };
  let FormattedText::formattedText(ft) = client::execute(&req)?;
  Ok(ft)
}
