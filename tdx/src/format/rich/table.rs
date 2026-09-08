//! Generated table values with fluent layout helpers.
//!
//! Start with [`cell`] and [`table`] for message-friendly defaults. All native
//! fields remain editable, including invisible cells, spans and presentation flags.

use td_types::enums::{PageBlockHorizontalAlignment, PageBlockVerticalAlignment};
use td_types::types;

use super::{IntoRichText, plain};

/// A native table block. Use [`table`] for headers and an empty text caption.
pub use types::inputPageBlockTable as Table;
/// A native table cell. Use [`cell`] for visible text and 1×1 spans.
pub use types::pageBlockTableCell as Cell;

/// Creates a visible 1×1 cell, aligned left and vertically centered.
pub fn cell(text: impl IntoRichText) -> Cell {
  Cell {
    text: Some(text.into_rich_text()), //.
    colspan: 1,
    rowspan: 1,
    align: PageBlockHorizontalAlignment::pageBlockHorizontalAlignmentLeft,
    valign: PageBlockVerticalAlignment::pageBlockVerticalAlignmentMiddle,
    ..Default::default()
  }
}

fluent! {
  /// Fluent layout methods for native cells.
  CellExt for Cell {
    /// Sets the column and row spans; geometry is validated by `TDLib`.
    fn span(mut self, colspan: i32, rowspan: i32) {
      self.colspan = colspan;
      self.rowspan = rowspan;
      self
    }
    /// Emphasizes this cell as a header.
    fn header(mut self) {
      self.is_header = true;
      self
    }
    /// Aligns content to the left.
    fn left(mut self) {
      self.align = PageBlockHorizontalAlignment::pageBlockHorizontalAlignmentLeft;
      self
    }
    /// Centers content horizontally.
    fn center(mut self) {
      self.align = PageBlockHorizontalAlignment::pageBlockHorizontalAlignmentCenter;
      self
    }
    /// Aligns content to the right.
    fn right(mut self) {
      self.align = PageBlockHorizontalAlignment::pageBlockHorizontalAlignmentRight;
      self
    }
    /// Aligns content to the top.
    fn top(mut self) {
      self.valign = PageBlockVerticalAlignment::pageBlockVerticalAlignmentTop;
      self
    }
    /// Centers content vertically.
    fn middle(mut self) {
      self.valign = PageBlockVerticalAlignment::pageBlockVerticalAlignmentMiddle;
      self
    }
    /// Aligns content to the bottom.
    fn bottom(mut self) {
      self.valign = PageBlockVerticalAlignment::pageBlockVerticalAlignmentBottom;
      self
    }
  }
}

/// Accepts explicit cells or inline text with default layout.
pub trait IntoCell {
  /// Converts the value without changing an explicit cell's layout.
  fn into_cell(self) -> Cell;
}

impl IntoCell for Cell {
  fn into_cell(self) -> Cell {
    self
  }
}

impl<T: IntoRichText> IntoCell for T {
  fn into_cell(self) -> Cell {
    cell(self)
  }
}

/// Starts a table with emphasized headers; empty input omits the header row.
/// Borders, stripes and compact padding are initially disabled.
pub fn table(headers: impl IntoIterator<Item = impl IntoCell>) -> Table {
  let headers: Vec<_> = headers.into_iter().map(|cell| cell.into_cell().header()).collect();
  let cells = if headers.is_empty() { Default::default() } else { vec![headers] };
  Table { cells, caption: plain(""), ..Default::default() }
}

fluent! {
  /// Fluent composition methods for native tables.
  TableExt for Table {
    /// Appends one row, retaining explicit cell layout.
    fn row(mut self, cells: impl IntoIterator<Item = impl IntoCell>) {
      self.cells.push(cells.into_iter().map(IntoCell::into_cell).collect());
      self
    }
    /// Appends rows in iteration order, including empty rows.
    fn rows(mut self, rows: impl IntoIterator<Item = impl IntoIterator<Item = impl IntoCell>>) {
      let rows = rows.into_iter().map(|row| row.into_iter().map(IntoCell::into_cell).collect());
      self.cells.extend(rows);
      self
    }
    /// Replaces the table caption.
    fn caption(mut self, caption: impl IntoRichText) {
      self.caption = caption.into_rich_text();
      self
    }
    /// Draws borders between cells.
    fn bordered(mut self) {
      self.is_bordered = true;
      self
    }
    /// Alternates row backgrounds.
    fn striped(mut self) {
      self.is_striped = true;
      self
    }
    /// Reduces cell padding.
    fn compact(mut self) {
      self.is_compact = true;
      self
    }
  }
}
