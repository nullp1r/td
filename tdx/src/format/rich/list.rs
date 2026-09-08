//! Lists of explicit native items. Each item contains an ordered block sequence.

use td_types::enums::InputPageBlock;
use td_types::types;

use super::{IntoRichText, paragraph};

/// A native list entry; its blocks, checkbox and numbering fields are editable.
pub use types::inputPageBlockListItem as ListItem;

/// Creates an unnumbered item containing one paragraph.
/// For nested blocks, construct [`ListItem`] with its `blocks` field directly.
pub fn list_item(text: impl IntoRichText) -> ListItem {
  ListItem { blocks: vec![paragraph(text)], ..Default::default() }
}

fluent! {
  /// Checkbox control for native list items.
  ListItemExt for ListItem {
    /// Shows a checkbox with the supplied initial state.
    fn checked(mut self, checked: bool) {
      self.has_checkbox = true;
      self.is_checked = checked;
      self
    }
  }
}

/// Builds an unnumbered list, preserving each item's blocks and checkbox state.
pub fn bullet_list(items: impl IntoIterator<Item = ListItem>) -> InputPageBlock {
  ordered_list_styled("", items)
}

/// Numbers items in decimal, starting at one.
pub fn ordered_list(items: impl IntoIterator<Item = ListItem>) -> InputPageBlock {
  ordered_list_styled("1", items)
}

/// Applies native numbering: `1`, `a`, `A`, `i`, `I`, or empty for bullets.
/// Replaces each item's numbering fields; other schemes are left to `TDLib`.
pub fn ordered_list_styled(numbering_type: &str, items: impl IntoIterator<Item = ListItem>) -> InputPageBlock {
  let items = items
    .into_iter()
    .enumerate()
    .map(|(index, mut item)| {
      item.value = if numbering_type.is_empty() { 0 } else { (index + 1) as i32 };
      item.r#type = numbering_type.into();
      item
    })
    .collect();
  types::inputPageBlockList { items }.into()
}

/// Builds unnumbered paragraph items with checkboxes.
pub fn checklist(items: impl IntoIterator<Item = (impl IntoRichText, bool)>) -> InputPageBlock {
  bullet_list(items.into_iter().map(|(text, checked)| list_item(text).checked(checked)))
}
