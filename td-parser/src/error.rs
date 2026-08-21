use std::{error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<'a> {
  UnterminatedGroup([char; 2]),
  Expected(&'static str),
  ExpectedTypeExpr,
  ExpectedEnum,
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
      Self::ExpectedEnum => f.write_str("expected enum name"),
      Self::ExpectedDefinitionKind => f.write_str("expected definition kind"),
      Self::UnexpectedInput(rest) => {
        let [slice, ellipsis] = if let Some(s) = rest.get(..20) { [s, "…"] } else { [rest, ""] };
        write!(f, "unexpected input: {slice}{ellipsis}")
      }
    }
  }
}
