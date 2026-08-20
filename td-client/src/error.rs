use std::error::Error as StdError;
use std::fmt;
use std::result;
use std::time::Duration;

use td_types::{enums, types};

pub type Result<T, E = Error> = result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
  /// Error returned by `TDLib` core.
  Td(enums::Error),
  /// JSON serialization or deserialization failure, preserving raw data if available.
  Json { source: serde_json::Error, raw: Option<String> },
  /// Client was destroyed or disconnected.
  Disconnected,
  /// Request timed out waiting for `TDLib` response.
  Timeout,
  /// Authentication flow failed.
  Auth(String),
  /// Low-level FFI or system error.
  Sys(String),
}

impl StdError for Error {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self {
      Self::Json { source, .. } => Some(source),
      _ => None,
    }
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Td(enums::Error::error(types::error { code, message })) => write!(f, "TDLib {code}: {message}"),
      Self::Json { source, raw: Some(raw) } => write!(f, "JSON error: {source} (payload: {raw})"),
      Self::Json { source, raw: None } => write!(f, "JSON error: {source}"),
      Self::Disconnected => f.write_str("client disconnected"),
      Self::Timeout => f.write_str("request timeout"),
      Self::Auth(msg) => write!(f, "auth failed: {msg}"),
      Self::Sys(msg) => write!(f, "system error: {msg}"),
    }
  }
}

impl From<serde_json::Error> for Error {
  fn from(source: serde_json::Error) -> Self {
    Self::Json { source, raw: None }
  }
}

impl Error {
  /// Returns the `TDLib` error code and message, if this is a `TDLib` error.
  pub fn td(&self) -> Option<(i32, &str)> {
    let Self::Td(enums::Error::error(e)) = self else { return None };
    Some((e.code, &e.message))
  }

  /// Checks if the error is a Telegram `FLOOD_WAIT` error, returning the wait duration.
  pub fn flood_wait(&self) -> Option<Duration> {
    let (_, msg) = self.td()?;
    let digits = msg.strip_prefix("FLOOD_WAIT_")?.split(|c: char| !c.is_ascii_digit()).next()?;
    digits.parse().ok().map(Duration::from_secs)
  }

  /// Checks if this is an unauthorized (401) error.
  pub fn is_unauthorized(&self) -> bool {
    matches!(self.td(), Some((401, _)))
  }

  /// Checks if this is a not found (404) error.
  pub fn is_not_found(&self) -> bool {
    matches!(self.td(), Some((404, _)))
  }
}
