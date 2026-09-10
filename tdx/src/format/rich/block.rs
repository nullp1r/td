//! Text and layout blocks. Nest blocks explicitly with arrays or iterators.

use td_types::enums::{ButtonStyle, InputPageBlock};
use td_types::types;

use crate::compose::markup::IntoCallbackData;

use super::IntoRichText;

/// Places inline content in a paragraph.
pub fn paragraph(text: impl IntoRichText) -> InputPageBlock {
  let text = text.into_rich_text();
  types::inputPageBlockParagraph { text }.into()
}

/// Creates a full-width row of native rich-message buttons.
pub fn button_row(buttons: impl IntoIterator<Item = types::inlineButton>) -> InputPageBlock {
  types::inputPageBlockButtonRow { buttons: buttons.into_iter().collect(), ..Default::default() }.into()
}

/// Constructs a default-style callback button inside a rich message.
pub fn callback_button(text: impl IntoRichText, data: impl IntoCallbackData) -> types::inlineButton {
  callback_button_with_style(text, data, ButtonStyle::buttonStyleDefault)
}

/// Constructs a dark-blue callback button inside a rich message.
pub fn primary_callback_button(text: impl IntoRichText, data: impl IntoCallbackData) -> types::inlineButton {
  callback_button_with_style(text, data, ButtonStyle::buttonStylePrimary)
}

/// Constructs a green callback button inside a rich message.
pub fn success_callback_button(text: impl IntoRichText, data: impl IntoCallbackData) -> types::inlineButton {
  callback_button_with_style(text, data, ButtonStyle::buttonStyleSuccess)
}

/// Constructs a red callback button inside a rich message.
pub fn danger_callback_button(text: impl IntoRichText, data: impl IntoCallbackData) -> types::inlineButton {
  callback_button_with_style(text, data, ButtonStyle::buttonStyleDanger)
}

fn callback_button_with_style(text: impl IntoRichText, data: impl IntoCallbackData, style: ButtonStyle) -> types::inlineButton {
  let text = Box::new(text.into_rich_text());
  let r#type = types::inlineKeyboardButtonTypeCallback { data: data.into_callback_data() }.into();
  types::inlineButton { text, style, r#type }
}

/// Places inline content in the document footer.
pub fn footer(footer: impl IntoRichText) -> InputPageBlock {
  let footer = footer.into_rich_text();
  types::inputPageBlockFooter { footer }.into()
}

/// Encloses blocks in a quotation without a credit line.
pub fn block_quote(blocks: impl IntoIterator<Item = impl Into<InputPageBlock>>) -> InputPageBlock {
  let blocks = blocks.into_iter().map(Into::into).collect();
  types::inputPageBlockBlockQuote { blocks, ..Default::default() }.into()
}

/// Places inline text in a collapsible quotation without a credit line.
pub fn block_quote_expandable(text: impl IntoRichText) -> InputPageBlock {
  let text = text.into_rich_text();
  types::inputPageBlockExpandableBlockQuote { text, ..Default::default() }.into()
}

/// Highlights inline text as a pull quote without a credit line.
pub fn pull_quote(text: impl IntoRichText) -> InputPageBlock {
  let text = text.into_rich_text();
  types::inputPageBlockPullQuote { text, ..Default::default() }.into()
}

/// Preserves whitespace in a code block; an empty string omits the language.
pub fn preformatted(text: impl IntoRichText, language: impl Into<String>) -> InputPageBlock {
  let text = text.into_rich_text();
  let language = language.into();
  types::inputPageBlockPreformatted { text, language }.into()
}

/// Creates a section heading with the supplied native size.
pub fn heading(text: impl IntoRichText, size: i32) -> InputPageBlock {
  let text = text.into_rich_text();
  types::inputPageBlockSectionHeading { text, size }.into()
}

/// Creates a bot thinking placeholder containing inline text.
pub fn thinking(text: impl IntoRichText) -> InputPageBlock {
  let text = text.into_rich_text();
  types::inputPageBlockThinking { text }.into()
}

/// Inserts a horizontal separator.
pub fn divider() -> InputPageBlock {
  InputPageBlock::inputPageBlockDivider
}

/// Groups blocks under an initially closed disclosure header.
pub fn details(
  header: impl IntoRichText, //.
  blocks: impl IntoIterator<Item = impl Into<InputPageBlock>>,
) -> InputPageBlock {
  let header = header.into_rich_text();
  let blocks = blocks.into_iter().map(Into::into).collect();
  types::inputPageBlockDetails { header, blocks, ..Default::default() }.into()
}

/// Names a position for in-document navigation.
pub fn anchor(name: impl Into<String>) -> InputPageBlock {
  let name = name.into();
  types::inputPageBlockAnchor { name }.into()
}

/// Displays a mathematical expression using native LaTeX rendering.
pub fn math(expression: impl Into<String>) -> InputPageBlock {
  let expression = expression.into();
  types::inputPageBlockMathematicalExpression { expression }.into()
}
