//! Generated table values with fluent layout helpers.
//!
//! Start with [`table`] and add an optional header plus rows. Tuples compose
//! heterogeneous cells; arrays and vectors compose homogeneous cells. All native
//! fields remain editable, including invisible cells, spans and presentation flags.

use td_types::enums::{PageBlockHorizontalAlignment, PageBlockVerticalAlignment};
use td_types::types;

use super::IntoRichText;

/// A native table block. Use [`table`] for an empty text caption and no rows.
pub use types::inputPageBlockTable as Table;
/// A native table cell. Use [`cell`] for visible text and 1×1 spans.
pub use types::pageBlockTableCell as Cell;

/// Creates a visible 1×1 cell, aligned left and vertically centered.
pub fn cell(text: impl IntoRichText) -> Cell {
  let text = Some(text.into_rich_text());
  let [colspan, rowspan] = [1; 2];
  let align = PageBlockHorizontalAlignment::pageBlockHorizontalAlignmentLeft;
  let valign = PageBlockVerticalAlignment::pageBlockVerticalAlignmentMiddle;
  Cell { text, colspan, rowspan, align, valign, ..Default::default() }
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

/// Accepts an explicit native cell or inline content with default layout.
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

/// A table row composed from explicit cells or inline content.
///
/// Tuples support heterogeneous cells; arrays and vectors support homogeneous
/// cells. Telegram currently supports at most 20 columns, mirrored by tuple
/// implementations through arity 20.
pub trait IntoRow {
  /// Converts the row while preserving explicit cell layout.
  fn into_row(self) -> Vec<Cell>;
}

impl<T: IntoCell, const N: usize> IntoRow for [T; N] {
  fn into_row(self) -> Vec<Cell> {
    self.into_iter().map(IntoCell::into_cell).collect()
  }
}

impl<T: IntoCell> IntoRow for Vec<T> {
  fn into_row(self) -> Vec<Cell> {
    self.into_iter().map(IntoCell::into_cell).collect()
  }
}

macro_rules! row_tuple {
  ($($ty:ident $value:ident),+ $(,)?) => {
    impl<$($ty: IntoCell),+> IntoRow for ($($ty,)+) {
      fn into_row(self) -> Vec<Cell> {
        let ($($value,)+) = self;
        vec![$($value.into_cell()),+]
      }
    }
  };
}

tuple_impls!(row_tuple);

/// Starts a headerless table with an empty text caption.
#[must_use]
pub fn table() -> Table {
  Default::default()
}

fn push_nonempty_row(table: &mut Table, row: impl IntoRow) {
  let row = row.into_row();
  if !row.is_empty() {
    table.cells.push(row);
  }
}

fluent! {
  /// Fluent composition methods for native tables.
  TableExt for Table {
    /// Appends one emphasized header row. Empty input is ignored.
    fn header(mut self, cells: impl IntoRow) {
      let cells = cells.into_row().into_iter().map(CellExt::header).collect::<Vec<_>>();
      if !cells.is_empty() {
        self.cells.push(cells);
      }
      self
    }
    /// Appends one row, retaining explicit cell layout. Empty input is ignored.
    fn row(mut self, cells: impl IntoRow) {
      push_nonempty_row(&mut self, cells);
      self
    }
    /// Appends nonempty rows in iteration order.
    fn rows(mut self, rows: impl IntoIterator<Item = impl IntoRow>) {
      for row in rows {
        push_nonempty_row(&mut self, row);
      }
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
