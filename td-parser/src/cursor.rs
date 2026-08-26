//! Byte-slice cursor primitives shared by the parser.
//!
//! Each operation returns slices of the original UTF-8 input and advances only
//! at character boundaries. Failed optional matches leave the cursor unchanged.

use crate::error::Error;

/// A copyable view of the schema suffix not yet consumed.
#[derive(Clone, Copy)]
pub struct Cursor<'a> {
  /// Unconsumed input.
  pub rest: &'a str,
}

impl<'a> Cursor<'a> {
  /// Starts a cursor at `rest`.
  pub const fn new(rest: &'a str) -> Self {
    Self { rest }
  }

  /// Removes leading ASCII whitespace.
  pub fn skip_ws(&mut self) {
    self.rest = self.rest.trim_ascii_start();
  }

  /// Consumes adjacent `//` comment lines and their separating whitespace.
  ///
  /// The returned slice includes the original prefixes and spacing so later
  /// documentation parsing can distinguish `//@` tags and `//-` continuations.
  pub fn comments(&mut self) -> &'a str {
    let start = self.rest;
    loop {
      self.skip_ws();
      let Some(tail) = self.rest.strip_prefix("//") else { break };
      self.rest = tail.split_once('\n').map_or("", |(_, rest)| rest);
    }
    start.substr_range(self.rest).and_then(|r| start.get(..r.start)).unwrap_or("")
  }

  /// Consumes the longest prefix whose characters satisfy `pred`.
  pub fn take_while(&mut self, mut pred: impl FnMut(char) -> bool) -> &'a str {
    let len = self.rest.find(|c| !pred(c)).unwrap_or(self.rest.len());
    let (head, tail) = self.rest.split_at(len);
    self.rest = tail;
    head
  }

  /// Consumes a TL identifier.
  pub fn ident(&mut self) -> Option<&'a str> {
    let Some('a'..='z' | 'A'..='Z' | '_') = self.rest.chars().next() else { return None };
    Some(self.take_while(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_')))
  }

  /// Consumes hexadecimal digits.
  pub fn hex(&mut self) -> &'a str {
    self.take_while(|c| matches!(c, '0'..='9' | 'a'..='f' | 'A'..='F'))
  }

  /// Consumes `pat` or reports that it was required.
  pub fn expect(&mut self, pat: &'static str) -> Result<(), Error<'a>> {
    self.maybe(pat).then_some(()).ok_or(Error::Expected(pat))
  }

  /// Consumes `pat` when it is the next input prefix.
  pub fn maybe(&mut self, pat: &str) -> bool {
    let Some(tail) = self.rest.strip_prefix(pat) else { return false };
    self.rest = tail;
    true
  }

  /// Consumes a possibly nested delimiter group when `open` is next.
  ///
  /// Returns `false` without advancing if the group is absent. Nesting uses the
  /// same delimiter pair; strings and escaping are not part of TL type groups.
  pub fn maybe_balanced(&mut self, [open, close]: [char; 2]) -> Result<bool, Error<'a>> {
    let Some(rest) = self.rest.strip_prefix(open) else { return Ok(false) };
    let mut chars = rest.chars();
    let mut depth = 1usize;
    for c in &mut chars {
      match depth {
        1 if c == close => {
          self.rest = chars.as_str();
          return Ok(true);
        }
        _ if c == close => depth -= 1,
        _ if c == open => depth += 1,
        _ => {}
      }
    }
    Err(Error::UnterminatedGroup([open, close]))
  }
}
