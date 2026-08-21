use std::time::Duration;
use std::{error, fmt, result};

use td_types::{enums, types};

pub type Result<T = ()> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
  /// Error returned by `TDLib` core.
  Td(enums::Error),
  /// JSON serialization or deserialization failure, preserving raw data if available.
  Json { source: serde_json::Error, raw: Option<Vec<u8>> },
  /// Client was destroyed or disconnected.
  Disconnected,
  /// Authentication flow failed.
  Auth(String),
}

impl error::Error for Error {
  fn source(&self) -> Option<&(dyn error::Error + 'static)> {
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
      Self::Json { source, raw: Some(raw) } => write!(f, "JSON error: {source} (payload: {})", String::from_utf8_lossy(raw)),
      Self::Json { source, raw: None } => write!(f, "JSON error: {source}"),
      Self::Disconnected => f.write_str("client disconnected"),
      Self::Auth(msg) => write!(f, "auth failed: {msg}"),
    }
  }
}

impl From<serde_json::Error> for Error {
  fn from(source: serde_json::Error) -> Self {
    Self::Json { source, raw: None }
  }
}

impl Error {
  pub(crate) fn json(source: serde_json::Error, raw: &[u8]) -> Self {
    Self::Json { source, raw: Some(raw.to_vec()) }
  }

  /// Returns the `TDLib` error code and message, if this is a `TDLib` error.
  pub fn td(&self) -> Option<(i32, &str)> {
    let Self::Td(enums::Error::error(e)) = self else { return None };
    Some((e.code, &e.message))
  }

  /// Checks if the error is a Telegram `FLOOD_WAIT` error, returning the wait duration.
  pub fn flood_wait(&self) -> Option<Duration> {
    let (_, msg) = self.td()?;
    msg.strip_prefix("FLOOD_WAIT_")?.parse().ok().map(Duration::from_secs)
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
