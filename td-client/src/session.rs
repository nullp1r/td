//! Session ownership, authentication, and ordered update consumption.
//!
//! Construct with [`Session::bot`] for a bot token, or [`Session::open`] to handle
//! authorization yourself. Each owner exposes cloneable request-only [`Client`]s.
//! Keep receiving application updates after authentication, and finish with
//! [`Session::close`]. See the [crate guide](crate) for setup and cleanup.
//!
//! # Manual authorization
//!
//! Read [`Session::recv_auth`] until `TDLib` reports readiness. Match the generated
//! authorization state and send the corresponding request through [`Client::send`]:
//! for example, `setAuthenticationPhoneNumber`, `checkAuthenticationCode`, or
//! `checkAuthenticationPassword`. Other states may require registration, email,
//! device confirmation, or application-specific interaction. Consult the generated
//! state documentation rather than assuming a fixed phone/code/password sequence.
//!
//! Parameter setup has already happened when `open` returns, although an earlier
//! `authorizationStateWaitTdlibParameters` update may still be queued.
//! Non-auth updates encountered during authentication are buffered for later
//! [`Session::recv`] calls. The auth stream is not a replayable state snapshot.
//!
//! # Session storage
//!
//! Use a separate session directory for each account, and do not open one directory
//! concurrently from multiple clients or processes. Existing authorization can be
//! reused; changing a token does not switch the account in an authorized directory.
//! Protect session files as credentials and configure database encryption through
//! the generated parameters when required.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::mpsc;

use td_types::enums::{AuthorizationState, Update};
use td_types::fns;

use crate::client::Client;
use crate::connection::Connection;
use crate::error::{Error, Result};
use crate::runtime;

/// The unique owner of one native `TDLib` session.
///
/// Use [`client`](Self::client) to issue requests without borrowing the update
/// consumer. This type is intentionally not `Clone`; graceful closure consumes it.
///
/// # Lifecycle
///
/// Always drive [`close`](Self::close) to completion before process exit.
/// Dropping the owner revokes new client requests but performs no native close,
/// blocking wait, or join. In-flight native operations are not automatically undone.
#[must_use = "call close().await to close TDLib cleanly"]
pub struct Session {
  connection: Arc<Connection>,
  updates: mpsc::UnboundedReceiver<Update>,
  buffered: VecDeque<Update>,
  closed: bool,
}

impl Session {
  /// Creates a native client session and applies the supplied `TDLib` parameters.
  ///
  /// This does not complete authorization. Use [`recv_auth`](Self::recv_auth)
  /// and generated authentication requests, or construct with [`bot`](Self::bot).
  /// [`parameters`] supplies editable defaults for the generated parameter struct.
  ///
  /// # Errors
  ///
  /// Returns the parameter request's error. Before returning a failure, attempts
  /// graceful closure and preserves the original error if cleanup also fails.
  ///
  /// # Cancellation
  ///
  /// Drive construction to completion. Dropping this future after native creation
  /// abandons the owner without completing graceful closure.
  pub async fn open(params: fns::setTdlibParameters) -> Result<Self> {
    let (connection, updates) = Connection::create();
    let (buffered, closed) = Default::default();
    let session = Self { connection, updates, buffered, closed };
    if let Err(error) = session.connection.request(&params).await {
      // Preserve the initiating failure after attempting native cleanup.
      let _ = session.close().await;
      return Err(error);
    }
    Ok(session)
  }

  /// Creates a session and waits for bot authorization to become ready.
  ///
  /// Submits the token when `TDLib` requests authentication. A session that is
  /// already ready is reused without verifying it against `token`; use a fresh
  /// directory or explicitly log out when switching accounts.
  ///
  /// # Errors
  ///
  /// Returns construction or token-request errors, or [`Error::Auth`] for an
  /// authorization state this narrow helper does not handle. It attempts graceful
  /// closure on returned authorization failure, preserving the original error.
  /// For custom flows use [`open`](Self::open) and [`recv_auth`](Self::recv_auth).
  ///
  /// # Cancellation
  ///
  /// Dropping the future abandons construction/authentication and graceful cleanup.
  pub async fn bot(params: fns::setTdlibParameters, token: &str) -> Result<Self> {
    let mut session = Self::open(params).await?;
    if let Err(error) = session.authorize_bot(token).await {
      let _ = session.close().await;
      return Err(error);
    }
    Ok(session)
  }

  /// Returns a detached request client for this session.
  ///
  /// The returned [`Client`] holds no borrow of the owner, so it can be used
  /// concurrently with [`recv`](Self::recv). Cloning it does not create another
  /// native client or extend the owner's operational lifetime.
  pub fn client(&self) -> Client {
    Client(Arc::downgrade(&self.connection))
  }

  /// Returns the next non-authorization update, or `None` after closure.
  ///
  /// Updates buffered by [`recv_auth`](Self::recv_auth) are returned first,
  /// in their original order. Authorization transitions are consumed internally
  /// and never returned here; consume them through `recv_auth` when needed.
  ///
  /// Requests and message tracking progress without polling this method, but
  /// the unbounded application queue continues to grow until it is drained.
  ///
  /// # Cancellation safety
  ///
  /// Cancelling a pending receive does not lose an application update. It may
  /// already have consumed authorization transitions, which are excluded from
  /// this API. Do not alternate this method with `recv_auth` expecting an auth
  /// transition consumed here to be replayed there.
  pub async fn recv(&mut self) -> Option<Update> {
    loop {
      let update = match self.buffered.pop_front() {
        Some(update) => update,
        None if self.closed => return None,
        None => self.receive().await,
      };
      if let Update::updateAuthorizationState(_) = update {
        continue;
      }
      return Some(update);
    }
  }

  /// Returns the next authorization transition, buffering other updates.
  ///
  /// This is an event stream, not a query for the current authorization state.
  /// Once closure has been observed, subsequent calls return
  /// `authorizationStateClosed` immediately. Otherwise this can wait indefinitely,
  /// including when the client is already ready and no new transition occurs.
  ///
  /// Buffered application updates remain available through [`recv`](Self::recv).
  /// Cancelling a pending call preserves those buffered updates.
  pub async fn recv_auth(&mut self) -> AuthorizationState {
    if self.closed {
      return AuthorizationState::authorizationStateClosed;
    }
    loop {
      match self.receive().await {
        Update::updateAuthorizationState(update) => return update.authorization_state,
        update => self.buffered.push_back(update),
      }
    }
  }

  /// Consumes the session and attempts graceful native closure.
  ///
  /// Closes request admission, sends `TDLib`'s generated `close`, and waits for
  /// `authorizationStateClosed`. It then releases routing and waits for the
  /// receiver's safe idle/ownership transition when necessary. If closure was
  /// already consumed, it does not submit another close request.
  ///
  /// Application updates remaining at closure are discarded with the owner.
  /// Drain any updates your application needs before initiating closure.
  ///
  /// # Errors
  ///
  /// Returns a close-request error while still clearing local waiters and
  /// unregistering the client. An error is not proof that native shutdown finished.
  /// Requests racing with closure may complete, receive a `TDLib` error, or
  /// become [`Error::Disconnected`]; graceful close is not an application-task join.
  ///
  /// # Cancellation
  ///
  /// This operation has no built-in deadline and is not cancellation-safe.
  /// Dropping its future can interrupt close or unregistration. Keep it driven
  /// until it returns; ordinary `Drop` does not finish the protocol.
  pub async fn close(mut self) -> Result {
    let result = self.finish().await;
    self.connection.disconnect();
    runtime::unregister(self.connection.id).await;
    result
  }

  async fn finish(&mut self) -> Result {
    if self.closed {
      return Ok(());
    }
    self.connection.close().await?;
    while !self.closed {
      self.receive().await;
    }
    Ok(())
  }

  async fn authorize_bot(&mut self, token: &str) -> Result {
    loop {
      match self.recv_auth().await {
        AuthorizationState::authorizationStateReady => return Ok(()),
        AuthorizationState::authorizationStateWaitTdlibParameters => {}
        AuthorizationState::authorizationStateWaitPhoneNumber => {
          self.connection.request(&fns::checkAuthenticationBotToken { token: token.into() }).await?;
        }
        state => return Err(Error::Auth(state)),
      }
    }
  }

  async fn receive(&mut self) -> Update {
    // The owner's Arc keeps the channel sender alive even after native closure.
    // Closed is an ordered authorization event, not channel EOF.
    let update = self.updates.recv().await.expect("Connection owns the update sender");
    if let Update::updateAuthorizationState(update) = &update
      && let AuthorizationState::authorizationStateClosed = update.authorization_state
    {
      self.closed = true;
    }
    update
  }
}

/// Builds editable `TDLib` parameters with local database and file directories.
///
/// Uses `directory/db` and `directory/files`; this function does not create them.
/// Enables file, chat-info, and message databases. Language defaults to `en`,
/// device model to `Server`, and application version to this crate's version.
/// Other fields retain their generated defaults, including the encryption key.
///
/// These are conveniences, not validated configuration or a security policy.
/// Disable databases you do not need and set application metadata/encryption
/// explicitly where appropriate. Paths are converted with lossy UTF-8 conversion.
///
/// # Examples
///
/// ```
/// let mut params = td_client::parameters(12345, "api hash", "session");
/// params.use_message_database = false;
/// params.device_model = "My application".into();
/// assert!(!params.use_message_database);
/// ```
pub fn parameters(api_id: i32, api_hash: impl Into<String>, directory: impl AsRef<Path>) -> fns::setTdlibParameters {
  let directory = directory.as_ref();
  fns::setTdlibParameters {
    api_id,
    api_hash: api_hash.into(),
    database_directory: directory.join("db").to_string_lossy().into_owned(),
    files_directory: directory.join("files").to_string_lossy().into_owned(),
    use_file_database: true,
    use_chat_info_database: true,
    use_message_database: true,
    system_language_code: "en".into(),
    device_model: "Server".into(),
    application_version: env!("CARGO_PKG_VERSION").into(),
    ..Default::default()
  }
}

#[cfg(test)]
mod tests {
  use std::assert_matches;
  use std::sync::Weak;
  use std::time::Duration;

  use td_types::fns;
  use tokio::time::timeout;

  use super::*;

  #[tokio::test]
  async fn authentication_buffers_application_updates_in_order() {
    let (connection, updates) = Connection::fixture();
    let mut session = Session { connection: Arc::clone(&connection), updates, buffered: VecDeque::new(), closed: false };
    connection.update(br#"{"@type":"updateOption","name":"first","value":{"@type":"optionValueEmpty"}}"#);
    connection.update(br#"{"@type":"updateOption","name":"second","value":{"@type":"optionValueEmpty"}}"#);
    let auth = br#"{"@type":"updateAuthorizationState","authorization_state":{"@type":"authorizationStateWaitPhoneNumber"}}"#;
    connection.update(auth);
    connection.update(br#"{"@type":"updateOption","name":"third","value":{"@type":"optionValueEmpty"}}"#);
    let exercise = async {
      let authorization = session.recv_auth().await;
      assert_matches!(authorization, AuthorizationState::authorizationStateWaitPhoneNumber);
      for expected in ["first", "second", "third"] {
        let update = session.recv().await;
        assert_matches!(update, Some(Update::updateOption(option)) if option.name == expected);
      }
    };
    timeout(Duration::from_secs(1), exercise).await.unwrap();
  }

  fn assert_send(_: impl Send) {}

  #[test]
  fn generic_operations_with_borrowed_callbacks_are_send() {
    let client = Client(Weak::new());
    let mut observe = |_, _| {};
    let request = fns::sendBotStartMessage::default();
    assert_send(client.track(&request, None, Some(&mut observe)));
    let request = fns::forwardMessages::default();
    assert_send(client.track_all(&request, None, Some(&mut observe)));
    let request = fns::downloadFile { synchronous: true, ..Default::default() };
    assert_send(client.download(&request, None, Some(&mut observe)));
  }
}
