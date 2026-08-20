//! # `td-client`
//!
//! Safe, lean, and zero-cost async runtime for `TDLib`.

use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::path::PathBuf;
use std::result::Result as StdResult;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use td_types::traits::Function;
use td_types::{enums, fns, types};

pub type Result<T, E = Error> = StdResult<T, E>;

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
  /// Returns the `TDLib` error code, if this is a `TDLib` error.
  #[must_use]
  pub const fn code(&self) -> Option<i32> {
    if let Self::Td(enums::Error::error(e)) = self { Some(e.code) } else { None }
  }

  /// Returns the `TDLib` error message, if this is a `TDLib` error.
  #[must_use]
  pub fn message(&self) -> Option<&str> {
    if let Self::Td(enums::Error::error(e)) = self { Some(&e.message) } else { None }
  }

  /// Checks if the error is a Telegram `FLOOD_WAIT` error, returning the wait duration.
  #[must_use]
  pub fn flood_wait(&self) -> Option<Duration> {
    let msg = self.message()?;
    let seconds_str = msg.strip_prefix("FLOOD_WAIT_")?;
    let (digits, _) = seconds_str.split_once([' ', ':', '\t', '\n']).unwrap_or((seconds_str, ""));
    let seconds: u64 = digits.trim().parse().ok()?;
    Some(Duration::from_secs(seconds))
  }

  /// Checks if this is an unauthorized (401) error.
  #[must_use]
  pub const fn is_unauthorized(&self) -> bool {
    matches!(self.code(), Some(401))
  }

  /// Checks if this is a not found (404) error.
  #[must_use]
  pub const fn is_not_found(&self) -> bool {
    matches!(self.code(), Some(404))
  }
}

// ---

#[derive(Debug, Clone)]
pub struct Config {
  pub api_id: i32,
  pub api_hash: String,
  pub database_dir: PathBuf,
  pub files_dir: PathBuf,
  pub use_test_dc: bool,
  pub system_language: String,
  pub device_model: String,
  pub application_version: String,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      api_id: 0,
      api_hash: String::new(),
      database_dir: "./tdlib_db".into(),
      files_dir: "./tdlib_files".into(),
      use_test_dc: false,
      system_language: "en".into(),
      device_model: "Server".into(),
      application_version: env!("CARGO_PKG_VERSION").into(),
    }
  }
}

// ---

#[expect(dead_code, reason = "config will be passed to TDLib initialization")]
pub struct Client {
  config: Config,
}

impl Client {
  #[must_use]
  pub fn new(config: Config) -> Self {
    Self { config }
  }

  #[expect(clippy::unused_async, reason = "stub async auth implementation")]
  pub async fn auth_bot(self, _token: impl Into<String>) -> Result<(ClientHandle, UpdateReceiver)> {
    Ok(self.start())
  }

  #[expect(clippy::unused_async, reason = "stub async auth implementation")]
  pub async fn auth_user<F, Fut>(self, _phone: impl Into<String>, _code_fn: F) -> Result<(ClientHandle, UpdateReceiver)>
  where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = String> + Send + 'static,
  {
    Ok(self.start())
  }

  #[must_use]
  pub fn start(self) -> (ClientHandle, UpdateReceiver) {
    let handle = ClientHandle { client_id: 1, extra: Arc::new(AtomicU64::new(1)) };
    (handle, UpdateReceiver { _client_id: 1 })
  }
}

// ---

#[derive(Debug, Clone)]
pub struct ClientHandle {
  client_id: i32,
  extra: Arc<AtomicU64>,
}

impl ClientHandle {
  #[must_use]
  pub const fn id(&self) -> i32 {
    self.client_id
  }

  #[tracing::instrument(name = "td::execute", skip(self, _fn), fields(client_id = self.client_id))]
  pub async fn execute<F: Function>(&self, _fn: &F) -> Result<F::Return> {
    let extra_id = self.extra.fetch_add(1, Ordering::Relaxed);
    tracing::debug!(extra = extra_id, "dispatching request to tdlib");
    Err(Error::Disconnected)
  }

  pub async fn send_text(&self, chat_id: i64, text: impl Into<String>) -> Result<enums::Message> {
    let req = fns::sendMessage { chat_id, input_message_content: make_input_text(text), ..Default::default() };
    self.execute(&req).await
  }

  pub async fn reply_text(&self, chat_id: i64, message_id: i64, text: impl Into<String>) -> Result<enums::Message> {
    let reply_to = Some(enums::InputMessageReplyTo::inputMessageReplyToMessage(types::inputMessageReplyToMessage { message_id, ..Default::default() }));
    let req = fns::sendMessage { chat_id, reply_to, input_message_content: make_input_text(text), ..Default::default() };
    self.execute(&req).await
  }
}

fn make_input_text(text: impl Into<String>) -> enums::InputMessageContent {
  enums::InputMessageContent::inputMessageText(types::inputMessageText {
    text: types::formattedText { text: text.into(), ..Default::default() },
    ..Default::default()
  })
}

// ---

pub struct UpdateReceiver {
  _client_id: i32,
}

impl UpdateReceiver {
  #[expect(clippy::unused_async, reason = "stub async receiver implementation")]
  pub async fn recv(&mut self) -> Option<enums::Update> {
    None
  }
}
