//! Text and layout blocks. Nest blocks explicitly with arrays or iterators.

use td_types::enums::InputPageBlock;
use td_types::types;

use super::IntoRichText;

/// Places inline content in a paragraph.
pub fn paragraph(text: impl IntoRichText) -> InputPageBlock {
  let text = text.into_rich_text();
  types::inputPageBlockParagraph { text }.into()
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
