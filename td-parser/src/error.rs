use std::{error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<'a> {
  UnterminatedGroup([char; 2]),
  Expected(&'static str),
  ExpectedTypeExpr,
  ExpectedCategory,
  ExpectedDefinitionKind,
  UnexpectedInput(&'a str),
}

impl error::Error for Error<'_> {}

impl fmt::Display for Error<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::UnterminatedGroup([open, close]) => write!(f, "unterminated '{open}{close}' group"),
      Self::Expected(pat) => write!(f, "expected '{pat}'"),
      Self::ExpectedTypeExpr => f.write_str("expected type expression"),
      Self::ExpectedCategory => f.write_str("expected category"),
      Self::ExpectedDefinitionKind => f.write_str("expected definition kind"),
      Self::UnexpectedInput(rest) => write!(f, "unexpected input: {}", rest.get(..20).unwrap_or(rest)),
    }
  }
}
