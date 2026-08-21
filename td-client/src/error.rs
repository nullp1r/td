use std::{error, fmt, result};

use td_types::types;

pub type Result<T = ()> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
  /// Error returned by `TDLib` core.
  Td(types::error),
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
      Self::Td(types::error { code, message }) => write!(f, "TDLib {code}: {message}"),
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
}
