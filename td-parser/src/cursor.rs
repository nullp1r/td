use crate::error::Error;

#[derive(Clone, Copy)]
pub struct Cursor<'a> {
  pub rest: &'a str,
}

impl<'a> Cursor<'a> {
  pub const fn new(rest: &'a str) -> Self {
    Self { rest }
  }

  pub fn skip_ws(&mut self) {
    self.rest = self.rest.trim_ascii_start();
  }

  pub fn comments(&mut self) -> &'a str {
    let start = self.rest;
    loop {
      self.skip_ws();
      let Some(tail) = self.rest.strip_prefix("//") else { break };
      self.rest = tail.split_once('\n').map_or("", |(_, rest)| rest);
    }
    start.get(..start.len() - self.rest.len()).unwrap_or("")
  }

  pub fn take_while(&mut self, mut pred: impl FnMut(u8) -> bool) -> &'a str {
    let len = self.rest.as_bytes().iter().position(|&b| !pred(b)).unwrap_or(self.rest.len());
    let (head, tail) = self.rest.split_at_checked(len).unwrap_or((self.rest, ""));
    self.rest = tail;
    head
  }

  pub fn ident(&mut self) -> Option<&'a str> {
    matches!(self.rest.as_bytes().first()?, b'a'..=b'z' | b'A'..=b'Z' | b'_')
      .then(|| self.take_while(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')))
  }

  pub fn expect(&mut self, pat: &'static str) -> Result<(), Error<'a>> {
    self.maybe(pat).then_some(()).ok_or(Error::Expected(pat))
  }

  pub fn maybe(&mut self, pat: &str) -> bool {
    let Some(tail) = self.rest.strip_prefix(pat) else { return false };
    self.rest = tail;
    true
  }

  pub fn maybe_balanced(&mut self, [open, close]: [char; 2]) -> Result<bool, Error<'a>> {
    let Some(tail) = self.rest.strip_prefix(open) else { return Ok(false) };
    let mut chars = tail.chars();
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
