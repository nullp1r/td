//! Parse errors that retain the relevant borrowed schema input.

use std::fmt::{self, Display};

/// A syntax error encountered while parsing a `TDLib` API schema.
///
/// Parsing stops at the first error. [`Self::UnexpectedInput`] borrows the
/// unconsumed suffix so callers can locate or report the failure without an
/// allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error<'a> {
  /// A generic or parameter group reached the end of input before its closing delimiter.
  #[error("unterminated '{}{}' group", .0[0], .0[1])]
  UnterminatedGroup([char; 2]),
  /// A fixed punctuation token was absent.
  #[error("expected '{0}'")]
  Expected(&'static str),
  /// A field type was absent or malformed.
  #[error("expected type expression")]
  ExpectedTypeExpr,
  /// The result category after `=` was absent.
  #[error("expected enum name")]
  ExpectedEnum,
  /// A `---...---` section marker named neither types nor functions.
  #[error("expected definition kind")]
  ExpectedDefinitionKind,
  /// Input at the start of the borrowed suffix did not begin a valid definition.
  #[error("unexpected input: {}", preview(.0))]
  UnexpectedInput(&'a str),
}

/// Formats a bounded preview without allocating an intermediate string.
fn preview(input: &str) -> impl Display + '_ {
  fmt::from_fn(move |f| {
    let [slice, ellipsis] = if let Some(slice) = input.get(..20) { [slice, "…"] } else { [input, ""] };
    write!(f, "{slice}{ellipsis}")
  })
}
