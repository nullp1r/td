//! A small typed runtime for `TDLib`'s client-ID JSON interface.
//!
//! `td-client` connects the generated functions and objects from `td-types` to
//! `TDLib`'s asynchronous native transport. It correlates concurrent requests,
//! routes any number of clients through `TDLib`'s single process-wide receiver,
//! preserves application-update order, tracks the operations whose direct
//! response is not their final result, and makes graceful shutdown explicit.
//!
//! The crate deliberately has no retry, deadline, logging, overflow, or task
//! policy. Those choices belong to the application. Its public model consists of
//! one owning [`Client`] and cloneable non-owning [`Sender`] values. Long-running
//! operations are ordinary futures with optional [`CancellationToken`]s.
//!
//! # Client ownership and shutdown
//!
//! A native client has exactly one Rust owner. [`Client`] receives ordered updates
//! and owns the right to call [`Client::shutdown`]; it is intentionally not
//! cloneable. [`Client::sender`] creates a [`Sender`] containing a weak reference,
//! so detached request tasks do not keep the client alive or acquire update and
//! shutdown authority.
//!
//! Dropping `Client` revokes its senders but performs no native cleanup. A normal
//! application always consumes the client with `shutdown`, which sends `TDLib`'s
//! `close` function, observes `authorizationStateClosed`, and waits for the
//! process-wide receiver to reach a safe ownership transition.
//!
//! ```ignore
//! use td_client::{Client, Result};
//! use td_types::{enums::User, fns, types};
//!
//! async fn run() -> Result {
//!   let parameters = td_client::params(123456, "api hash", ".td");
//!   let client = Client::bot(parameters, "bot token").await?;
//!   let sender = client.sender();
//!
//!   let User::user(types::user { id, .. }) = sender.send(&fns::getMe {}).await?;
//!   println!("authorized as {id}");
//!
//!   drop(sender);
//!   client.shutdown().await
//! }
//! ```
//!
//! If application work can fail, preserve that error separately from shutdown:
//!
//! ```ignore
//! let result = application(&mut client).await;
//! let shutdown = client.shutdown().await;
//! result?;
//! shutdown
//! ```
//!
//! # Direct requests
//!
//! [`Sender::send`] accepts any generated `Function` and returns its declared
//! response type. Serialization failures, `TDLib` `error` objects, malformed
//! responses, and disconnection are all returned to the caller. Requests accepted
//! concurrently are distinguished by `@extra`; a request racing shutdown is
//! either submitted before `close` or rejected.
//!
//! Most `TDLib` functions complete with this direct response. In particular,
//! message edits complete through `send`; `updateMessageEdited` is an application
//! update, not a request-completion signal.
//!
//! # Message sends
//!
//! Normal send functions returning `td_types::enums::Message` or
//! `td_types::enums::Messages` can first return temporary messages whose
//! `sending_state` is pending. [`Sender::send_message`] and
//! [`Sender::send_messages`] bind those temporary `(chat_id, message_id)` keys on
//! the receiver thread before awaiting authoritative success, failure, or non-cache
//! deletion. This does not require the application to poll [`Client::recv`], and
//! the original terminal update is still enqueued unchanged.
//!
//! The tracked methods rely on a caller invariant: use them only for actual
//! non-preview normal-message sends. Preview requests construct pending-looking
//! messages but emit no terminal send update; getters and edits have different
//! completion contracts. Send all of those through [`Sender::send`] instead.
//!
//! ```ignore
//! let content = types::inputMessageText {
//!   text: types::formattedText { text: "hello".into(), ..Default::default() },
//!   ..Default::default()
//! };
//! let request = fns::sendMessage {
//!   chat_id,
//!   input_message_content: content.into(),
//!   ..Default::default()
//! };
//! let message = sender.send_message(&request, None).await?;
//! ```
//!
//! Pass a borrowed cancellation token when the application needs cancellation.
//! Cancellation deletes only a still-pending temporary ID and awaits its terminal
//! result. [`Error::Cancelled`] means deletion won; an authoritative success is
//! returned normally and its final ID is never explicitly deleted. This is not
//! server-atomic: `TDLib` itself may delete a concurrently accepted message after
//! removing its pending record.
//!
//! # Files
//!
//! [`Sender::download`] requires `downloadFile.synchronous = true`. In `TDLib` this
//! is an asynchronous request promise whose response becomes ready only when the
//! requested full file or exact byte range succeeds, fails, is cancelled, or is
//! superseded by a request for another range. The method returns that final file
//! state and reports coalesced observations through a synchronous `Send` callback.
//! Cancellation awaits both `cancelDownloadFile` and the original response;
//! completion wins the race.
//!
//! [`Sender::upload`] binds the file ID from `preliminaryUploadFile`, reports
//! coalesced progress through the same callback shape, and waits until preliminary
//! staging first becomes inactive. `TDLib` explicitly does not complete the upload
//! until the file is sent in a message, so the returned file ID and byte counts are
//! neither standalone success nor failure. Cancellation awaits
//! `cancelPreliminaryUploadFile`.
//!
//! Progress callbacks run synchronously and should remain cheap. Dropping the
//! operation future abandons local observation without native cancellation;
//! cancellation cleanup progresses while that future is polled.
//!
//! # Updates and authorization
//!
//! [`Client::recv`] returns ordinary updates in `TDLib` transport order and consumes
//! authorization transitions internally. [`Client::recv_auth`] returns the next
//! authorization state while buffering intervening ordinary updates; later calls
//! to `recv` replay that buffer in order.
//!
//! The update queue is unbounded. `TDLib`'s synchronous native receiver cannot await
//! capacity, and silently dropping or inventing a spill policy would lose protocol
//! information. Applications should keep their receive loop moving and dispatch
//! slow work separately.
//!
//! # Synchronous functions
//!
//! [`execute`] exposes modern `td_execute` for the small set of functions `TDLib`
//! documents as synchronously executable. It is client-independent and uses the
//! same generated request/response typing and error mapping as `Sender::send`.
//! Because `TDLib` may invalidate a returned JSON buffer on the next `td_receive` or
//! `td_execute`, both calls share one process-wide mutex and parsing finishes while
//! that mutex is held. An execute call may consequently wait for the configured
//! receive timeout.
//!
//! [`set_receive_timeout`] changes only the next low-level receive wait. It is not
//! an operation timeout or retry policy.

use std::collections::{HashMap, VecDeque};
use std::ffi::CStr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock, Weak};
use std::time::Duration;
use std::{fmt, future, mem, pin::pin, result, thread};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use td_types::enums::{AuthorizationState, File, Message, MessageSendingState, Messages, Update};
use td_types::traits::Function;
use td_types::{fns, types};

pub use tokio_util::sync::CancellationToken;

/// A `td-client` operation result.
pub type Result<T = ()> = result::Result<T, Error>;

/// A failure at the typed `TDLib` boundary or in a long-running operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// `TDLib` returned an `error` object.
  #[error("TDLib: {} {}", .0.code, .0.message)]
  Td(types::error),
  /// A request could not be serialized or a response or update could not be decoded.
  ///
  /// The error is shared because one malformed process-wide envelope must fail
  /// every client whose route can no longer be determined.
  #[error("JSON: {0}")]
  Json(#[source] Arc<serde_json::Error>),
  /// The narrow [`Client::bot`] flow encountered an authorization state it cannot handle.
  #[error("unexpected auth state: {0:?}")]
  Auth(AuthorizationState),
  /// Caller-requested cancellation completed its native cleanup.
  #[error("operation cancelled")]
  Cancelled,
  /// `TDLib` reported the terminal failure of a tracked message send.
  #[error("message {} in chat {} failed: {} {}", .0.old_message_id, .0.message.chat_id, .0.error.code, .0.error.message)]
  MessageFailed(Box<types::updateMessageSendFailed>),
  /// A non-cache deletion removed a tracked temporary message before success.
  #[error("message {} in chat {} was deleted while being sent", .0.message_id, .0.chat_id)]
  MessageDeleted(MessageKey),
  /// A tracked response attempted to reuse an existing temporary-message key.
  #[error("message {} in chat {} is already being tracked", .0.message_id, .0.chat_id)]
  MessageCollision(MessageKey),
  /// `TDLib` returned a structurally impossible result.
  #[error("unexpected TDLib response: {0}")]
  UnexpectedResponse(&'static str),
  /// The owning client or a required response channel disappeared.
  #[error("client disconnected")]
  Disconnected,
}

impl From<serde_json::Error> for Error {
  fn from(error: serde_json::Error) -> Self {
    Self::Json(Arc::new(error))
  }
}

/// The chat and temporary message ID used to correlate a pending send.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageKey {
  /// Chat containing the pending message.
  pub chat_id: i64,
  /// Temporary message identifier returned by the direct send response.
  pub message_id: i64,
}

struct MessageOperation {
  key: MessageKey,
  result: oneshot::Receiver<Result<types::message>>,
}

impl MessageOperation {
  async fn finish(mut self, client: &ClientState, cancel: Option<&CancellationToken>) -> Result<types::message> {
    tokio::select! {
      biased;
      result = &mut self.result => result.map_err(|_| Error::Disconnected)?,
      () = cancelled(cancel) => self.cancel(client).await,
    }
  }

  async fn cancel(self, client: &ClientState) -> Result<types::message> {
    if client.message_pending(self.key) {
      let cancellation = client.delete_message(self.key).await;
      // Deletion can fail after a terminal update won the race. Preserve that
      // authoritative result; surface the deletion error only while still pending.
      if let Err(error) = cancellation
        && client.message_pending(self.key)
      {
        return Err(error);
      }
    }
    match self.result.await.map_err(|_| Error::Disconnected)? {
      Err(Error::MessageDeleted(_)) => Err(Error::Cancelled),
      Ok(message) => Ok(message),
      Err(error) => Err(error),
    }
  }
}

#[derive(Debug)]
enum FileState {
  Unknown,
  Known(FileProgress),
  Json(Arc<serde_json::Error>),
}

/// The copy-only transfer fields retained from a `TDLib` `file` object.
///
/// Paths, remote identifiers, and other owned metadata remain in the original
/// application update and are not duplicated here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileProgress {
  /// `TDLib` file identifier.
  pub file_id: i32,
  /// Current file size, or zero when unknown.
  pub size: i64,
  /// Expected file size, which `TDLib` may report approximately.
  pub expected_size: i64,
  /// Start offset of the currently downloaded range.
  pub download_offset: i64,
  /// Contiguous downloaded prefix size from `download_offset`.
  pub downloaded_prefix_size: i64,
  /// Total number of downloaded bytes.
  pub downloaded_size: i64,
  /// Current download activity.
  pub download: TransferState,
  /// Total number of uploaded bytes.
  pub uploaded_size: i64,
  /// Current upload activity.
  pub upload: TransferState,
}

/// A compact projection of `TDLib`'s active and completed transfer flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
  /// The transfer is neither active nor reported complete.
  Inactive,
  /// The transfer is currently active.
  Active,
  /// `TDLib` reports the transfer complete.
  Completed,
}

struct FileUpdates {
  id: i32,
  states: watch::Receiver<FileState>,
  unseen: bool,
}

impl FileUpdates {
  async fn next(&mut self) -> Result<FileProgress> {
    loop {
      if !mem::take(&mut self.unseen) && self.states.changed().await.is_err() {
        return match &*self.states.borrow() {
          FileState::Json(error) => Err(Error::Json(Arc::clone(error))),
          _ => Err(Error::Disconnected),
        };
      }
      match &*self.states.borrow_and_update() {
        FileState::Known(progress) => return Ok(*progress),
        FileState::Json(error) => return Err(Error::Json(Arc::clone(error))),
        FileState::Unknown => {}
      }
    }
  }
}

/// A cloneable, non-owning request capability for one [`Client`].
///
/// `Sender` cannot receive updates or initiate shutdown, and its weak reference
/// does not extend the owning client's lifetime.
#[derive(Clone)]
pub struct Sender(Weak<ClientState>);

impl Sender {
  /// Sends a generated function and returns its direct correlated response.
  ///
  /// Use this for ordinary functions, message edits, getters, and preview-only
  /// sends. Normal sends requiring a later authoritative terminal result use
  /// [`Self::send_message`] or [`Self::send_messages`] instead.
  pub async fn send<F: Function>(&self, request: &F) -> Result<F::Return> {
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    client.execute_request(request, false).await
  }

  /// Sends one actual non-preview normal message and awaits its terminal result.
  ///
  /// The request must return `enums::Message` and obey the tracked-request
  /// invariant described in this crate's module-level documentation. Cancellation
  /// deletes a still-pending temporary message and returns [`Error::Cancelled`]
  /// after `TDLib` reports its deletion; an authoritative success wins the race.
  pub async fn send_message<F: Function<Return = Message>>(&self, request: &F, cancel: Option<&CancellationToken>) -> Result<types::message> {
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    let operations = client.start_messages(request, false).await?;
    let [operation] = operations.try_into().map_err(|_| Error::UnexpectedResponse("expected one message"))?;
    operation.finish(&client, cancel).await
  }

  /// Sends a batch of actual non-preview normal messages.
  ///
  /// Registration is atomic: duplicate temporary keys or collisions with existing
  /// sends fail without registering only part of the batch. Terminal results stay
  /// in direct-response order and preserve independent send failures.
  pub async fn send_messages<F: Function<Return = Messages>>(&self, request: &F, cancel: Option<&CancellationToken>) -> Result<Vec<Result<types::message>>> {
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    let operations = client.start_messages(request, true).await?;
    let mut results = Vec::with_capacity(operations.len());
    for operation in operations {
      results.push(operation.finish(&client, cancel).await);
    }
    Ok(results)
  }

  /// Downloads an exact range while reporting coalesced progress.
  ///
  /// The request must set `synchronous` to `true`, selecting `TDLib`'s final
  /// exact-range response. Cancellation awaits `cancelDownloadFile`; completion
  /// wins if its response is already available. The `Send` progress callback keeps
  /// the operation future movable between executor threads.
  ///
  /// # Panics
  ///
  /// Panics if `request.synchronous` is `false`.
  pub async fn download(
    &self,
    request: &fns::downloadFile,
    cancel: Option<&CancellationToken>,
    mut progress: impl FnMut(FileProgress) + Send,
  ) -> Result<types::file> {
    self.download_inner(request, cancel, &mut progress).await
  }

  async fn download_inner(
    &self,
    request: &fns::downloadFile,
    cancel: Option<&CancellationToken>,
    progress: &mut (dyn FnMut(FileProgress) + Send),
  ) -> Result<types::file> {
    assert!(request.synchronous, "downloadFile.synchronous must be true");
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    let mut updates = client.file_updates(request.file_id);
    let mut response = pin!(client.execute_request(request, false));
    loop {
      tokio::select! {
        biased;
        result = &mut response => {
          let File::file(file) = result?;
          progress(file_progress(&file));
          return Ok(file);
        }
        update = updates.next() => {
          progress(update?);
        }
        () = cancelled(cancel) => {
          let request = fns::cancelDownloadFile { file_id: updates.id, only_if_pending: false };
          let cancellation = client.execute_request(&request, false).await;
          return match (cancellation, response.await) {
            (_, Ok(File::file(file))) => {
              progress(file_progress(&file));
              Ok(file)
            }
            (Ok(_), Err(Error::Td(_))) => Err(Error::Cancelled),
            (Err(error), Err(_)) | (Ok(_), Err(error)) => Err(error),
          };
        }
      }
    }
  }

  /// Runs preliminary file staging while reporting coalesced progress.
  ///
  /// Returns the first non-active observation, including the assigned file ID and
  /// latest byte counts. `TDLib` does not complete a preliminary upload until the
  /// file is sent in a message, so this result is not standalone upload success or
  /// failure. The `Send` progress callback keeps the operation future movable
  /// between executor threads. Cancellation awaits `cancelPreliminaryUploadFile`.
  pub async fn upload(
    &self,
    request: &fns::preliminaryUploadFile,
    cancel: Option<&CancellationToken>,
    mut progress: impl FnMut(FileProgress) + Send,
  ) -> Result<FileProgress> {
    self.upload_inner(request, cancel, &mut progress).await
  }

  async fn upload_inner(
    &self,
    request: &fns::preliminaryUploadFile,
    cancel: Option<&CancellationToken>,
    progress: &mut (dyn FnMut(FileProgress) + Send),
  ) -> Result<FileProgress> {
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    let (reply, response) = oneshot::channel();
    client.submit_request(request, false, PendingReply::Upload(reply))?;
    let mut updates = response.await.map_err(|_| Error::Disconnected)??;
    loop {
      tokio::select! {
        biased;
        update = updates.next() => {
          let update = update?;
          progress(update);
          if update.upload != TransferState::Active {
            return Ok(update);
          }
        }
        () = cancelled(cancel) => {
          let request = fns::cancelPreliminaryUploadFile { file_id: updates.id };
          client.execute_request(&request, false).await?;
          return Err(Error::Cancelled);
        }
      }
    }
  }
}

/// The sole owner of one live `TDLib` client, its ordered updates, and shutdown right.
///
/// `Client` is intentionally non-`Clone`. Use [`Self::sender`] for detached
/// request-only access and consume the client with [`Self::shutdown`].
#[must_use = "call shutdown().await to finish TDLib cleanly"]
pub struct Client {
  state: Arc<ClientState>,
  updates: mpsc::UnboundedReceiver<Result<Update>>,
  buffered_updates: VecDeque<Update>,
  closed: bool,
}

impl fmt::Debug for Client {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Client").field("id", &self.state.id).finish_non_exhaustive()
  }
}

impl Client {
  /// Creates a client and applies `TDLib` parameters without completing authorization.
  ///
  /// Drive interactive authorization with [`Self::recv_auth`] and generated
  /// authentication functions. If parameter setup fails, construction attempts
  /// graceful shutdown and returns the original failure.
  pub async fn new(parameters: fns::setTdlibParameters) -> Result<Self> {
    let client = Self::create_unconfigured();
    if let Err(error) = client.state.execute_request(&parameters, false).await {
      let _ = client.shutdown().await;
      return Err(error);
    }
    Ok(client)
  }

  /// Creates, configures, and authorizes a bot client.
  ///
  /// The helper handles `TDLib`'s parameter and bot-token states. Any other state is
  /// returned as [`Error::Auth`]. Failures attempt graceful shutdown before the
  /// original error is returned.
  pub async fn bot(parameters: fns::setTdlibParameters, token: &str) -> Result<Self> {
    let mut client = Self::new(parameters).await?;
    if let Err(error) = client.authorize_bot(token).await {
      let _ = client.shutdown().await;
      return Err(error);
    }
    Ok(client)
  }

  /// Returns a cloneable weak request capability for this client.
  pub fn sender(&self) -> Sender {
    Sender(Arc::downgrade(&self.state))
  }

  /// Receives the next ordinary update in `TDLib` order.
  ///
  /// Authorization transitions are consumed internally. Ordinary updates buffered
  /// by [`Self::recv_auth`] are returned first. After the closed authorization
  /// state and buffered updates are exhausted, this returns `Ok(None)`.
  pub async fn recv(&mut self) -> Result<Option<Update>> {
    loop {
      let update = match self.buffered_updates.pop_front() {
        Some(update) => update,
        None if self.closed => return Ok(None),
        None => self.recv_event().await?,
      };
      match update {
        Update::updateAuthorizationState(_) => {}
        update => return Ok(Some(update)),
      }
    }
  }

  /// Receives the next authorization state without losing ordinary updates.
  ///
  /// Intervening non-authorization updates are buffered for [`Self::recv`]. Once
  /// closed has been observed, subsequent calls return
  /// `authorizationStateClosed` immediately.
  pub async fn recv_auth(&mut self) -> Result<AuthorizationState> {
    if self.closed {
      return Ok(AuthorizationState::authorizationStateClosed);
    }
    loop {
      match self.recv_event().await? {
        Update::updateAuthorizationState(update) => return Ok(update.authorization_state),
        update => self.buffered_updates.push_back(update),
      }
    }
  }

  /// Gracefully closes `TDLib` and consumes the sole lifecycle owner.
  ///
  /// Shutdown closes the request gate atomically with submitting `close`, observes
  /// `authorizationStateClosed`, revokes local operations, unregisters the client,
  /// and awaits the receiver's safe idle or new-owner transition. Event failures
  /// encountered while draining are preserved while shutdown continues toward the
  /// terminal state.
  pub async fn shutdown(mut self) -> Result {
    let result = self.close_and_wait().await;
    self.state.disconnect();
    ROUTER.unregister(self.state.id).await;
    result
  }

  fn create_unconfigured() -> Self {
    // SAFETY: The call takes no arguments and returns an opaque ID by value.
    let id = unsafe { td_sys::td_create_client_id() };
    let (updates, receiver) = mpsc::unbounded_channel();
    let (next_request_id, buffered_updates) = Default::default();
    let registry = ClientRegistry { accepting_requests: true, ..Default::default() };
    let state = Arc::new(ClientState { id, next_request_id, registry: Mutex::new(registry), updates });
    ROUTER.register(id, Arc::downgrade(&state));
    Self { state, updates: receiver, buffered_updates, closed: false }
  }

  async fn authorize_bot(&mut self, token: &str) -> Result {
    loop {
      match self.recv_auth().await? {
        AuthorizationState::authorizationStateWaitTdlibParameters => {}
        AuthorizationState::authorizationStateWaitPhoneNumber => {
          let request = fns::checkAuthenticationBotToken { token: token.into() };
          self.state.execute_request(&request, false).await?;
        }
        AuthorizationState::authorizationStateReady => return Ok(()),
        state => return Err(Error::Auth(state)),
      }
    }
  }

  async fn close_and_wait(&mut self) -> Result {
    if self.closed {
      return Ok(());
    }
    self.state.execute_request(&fns::close {}, true).await?;
    let mut result = Ok(());
    while !self.closed {
      match self.recv_event().await {
        Ok(_) => {}
        Err(Error::Disconnected) => return result.and(Err(Error::Disconnected)),
        Err(error) => result = result.and(Err(error)),
      }
    }
    result
  }

  async fn recv_event(&mut self) -> Result<Update> {
    let update = self.updates.recv().await.ok_or(Error::Disconnected)??;
    if let Update::updateAuthorizationState(update) = &update
      && let AuthorizationState::authorizationStateClosed = &update.authorization_state
    {
      self.closed = true;
      self.state.disconnect();
    }
    Ok(update)
  }
}

#[derive(Serialize)]
struct OutgoingRequest<'a, F> {
  #[serde(rename = "@extra")]
  extra: u64,
  #[serde(flatten)]
  request: &'a F,
}

enum PendingReply {
  Request(oneshot::Sender<Result<Vec<u8>>>),
  Messages { many: bool, reply: oneshot::Sender<Result<Vec<MessageOperation>>> },
  Upload(oneshot::Sender<Result<FileUpdates>>),
}

impl PendingReply {
  fn fail(self, error: Error) {
    match self {
      Self::Request(reply) => drop(reply.send(Err(error))),
      Self::Messages { reply, .. } => drop(reply.send(Err(error))),
      Self::Upload(reply) => drop(reply.send(Err(error))),
    }
  }
}

#[derive(Default)]
struct ClientRegistry {
  accepting_requests: bool,
  requests: HashMap<u64, PendingReply>,
  message_sends: HashMap<MessageKey, oneshot::Sender<Result<types::message>>>,
  files: HashMap<i32, watch::Sender<FileState>>,
}

struct ClientState {
  id: i32,
  next_request_id: AtomicU64,
  registry: Mutex<ClientRegistry>,
  updates: mpsc::UnboundedSender<Result<Update>>,
}

impl ClientState {
  async fn execute_request<F: Function>(&self, request: &F, closing: bool) -> Result<F::Return> {
    let (reply, response) = oneshot::channel();
    self.submit_request(request, closing, PendingReply::Request(reply))?;
    let raw = response.await.map_err(|_| Error::Disconnected)??;
    serde_json::from_slice(&raw).map_err(Into::into)
  }

  async fn start_messages<F: Function>(&self, request: &F, many: bool) -> Result<Vec<MessageOperation>> {
    let (reply, response) = oneshot::channel();
    self.submit_request(request, false, PendingReply::Messages { many, reply })?;
    response.await.map_err(|_| Error::Disconnected)?
  }

  fn submit_request<F: Function>(&self, request: &F, closing: bool, reply: PendingReply) -> Result {
    let extra = self.next_request_id.fetch_add(1, Ordering::Relaxed);
    let mut request = serde_json::to_vec(&OutgoingRequest { extra, request })?;
    request.push(0);

    let mut registry = self.registry.lock().unwrap();
    if !registry.accepting_requests || (!closing && self.updates.is_closed()) {
      return Err(Error::Disconnected);
    }
    if closing {
      registry.accepting_requests = false;
    }
    registry.requests.insert(extra, reply);
    // Keep the request gate locked through td_send: close must not overtake a
    // request whose reply was accepted into the registry.
    // SAFETY: The client ID came from TDLib and `request` is live and NUL-terminated.
    unsafe { td_sys::td_send(self.id, request.as_ptr().cast()) };
    Ok(())
  }

  async fn delete_message(&self, MessageKey { chat_id, message_id }: MessageKey) -> Result {
    let delete = fns::deleteMessages { chat_id, message_ids: vec![message_id], revoke: true };
    self.execute_request(&delete, false).await?;
    Ok(())
  }

  fn message_pending(&self, key: MessageKey) -> bool {
    self.registry.lock().unwrap().message_sends.contains_key(&key)
  }

  fn complete_request(self: &Arc<Self>, extra: u64, r#type: &str, raw: &[u8]) {
    let Some(reply) = self.registry.lock().unwrap().requests.remove(&extra) else { return };
    if let "error" = r#type {
      return reply.fail(parse_td_error(raw));
    }
    match reply {
      PendingReply::Request(reply) => drop(reply.send(Ok(raw.to_vec()))),
      PendingReply::Messages { many, reply } => {
        // Binding occurs on the sole receiver thread before waking the requester;
        // a terminal send update therefore cannot pass an unregistered key.
        let messages = parse_messages(raw, many).and_then(|messages| self.bind_messages(messages));
        drop(reply.send(messages));
      }
      PendingReply::Upload(reply) => {
        // The preliminary response is the first authoritative source of its file
        // ID, so seed progress before waking the requester.
        let result = parse_file(raw).map(|file| self.bind_file(&file));
        drop(reply.send(result));
      }
    }
  }

  fn bind_messages(self: &Arc<Self>, messages: Vec<types::message>) -> Result<Vec<MessageOperation>> {
    // Validate the whole batch before inserting anything. Partial registration
    // would strand the unregistered messages if a later key collided.
    let mut keys: Vec<_> = messages.iter().filter(|message| is_pending(message)).map(message_key).collect();
    keys.sort_unstable();
    if let Some([key, _]) = keys.array_windows().find(|[a, b]| a == b) {
      return Err(message_collision(*key));
    }

    let mut registry = self.registry.lock().unwrap();
    registry.message_sends.retain(|_, result| !result.is_closed());
    if let Some(key) = keys.into_iter().find(|key| registry.message_sends.contains_key(key)) {
      return Err(message_collision(key));
    }

    let sends = messages.into_iter().map(|message| {
      let key = message_key(&message);
      let (result, receiver) = oneshot::channel();
      if is_pending(&message) {
        registry.message_sends.insert(key, result);
      } else {
        drop(result.send(Ok(message)));
      }
      MessageOperation { key, result: receiver }
    });

    Ok(sends.collect())
  }

  fn file_updates(self: &Arc<Self>, id: i32) -> FileUpdates {
    let mut registry = self.registry.lock().unwrap();
    registry.files.retain(|_, states| states.receiver_count() > 0);
    let states = registry.files.entry(id).or_insert_with(|| watch::channel(FileState::Unknown).0).subscribe();
    FileUpdates { id, states, unseen: true }
  }

  fn bind_file(self: &Arc<Self>, file: &types::file) -> FileUpdates {
    let file_id = file.id;
    let progress = file_progress(file);
    let mut registry = self.registry.lock().unwrap();
    registry.files.retain(|_, states| states.receiver_count() > 0);
    let sender = registry.files.entry(file_id).or_insert_with(|| watch::channel(FileState::Unknown).0);
    sender.send_replace(FileState::Known(progress));
    let states = sender.subscribe();
    FileUpdates { id: file_id, states, unseen: true }
  }

  fn route_update(self: &Arc<Self>, r#type: &str, raw: &[u8]) {
    let update = match r#type {
      "error" => Err(parse_td_error(raw)),
      _ => match serde_json::from_slice(raw) {
        Ok(update) => {
          self.observe_update(&update);
          Ok(update)
        }
        Err(error) => {
          let error = Arc::new(error);
          self.fail_operations(&error);
          Err(Error::Json(error))
        }
      },
    };
    drop(self.updates.send(update));
  }

  fn observe_update(&self, update: &Update) {
    let mut registry = self.registry.lock().unwrap();
    match update {
      Update::updateMessageSendSucceeded(update) => {
        let key = MessageKey { chat_id: update.message.chat_id, message_id: update.old_message_id };
        if let Some(sender) = registry.message_sends.remove(&key)
          && !sender.is_closed()
        {
          // The result needs one clone because the original update remains intact
          // for the ordered application queue.
          drop(sender.send(Ok(update.message.clone())));
        }
      }
      Update::updateMessageSendFailed(update) => {
        let key = MessageKey { chat_id: update.message.chat_id, message_id: update.old_message_id };
        if let Some(sender) = registry.message_sends.remove(&key)
          && !sender.is_closed()
        {
          drop(sender.send(Err(Error::MessageFailed(Box::new(update.clone())))));
        }
      }
      Update::updateDeleteMessages(update) if !update.from_cache => {
        for &message_id in &update.message_ids {
          if let Some(sender) = registry.message_sends.remove(&MessageKey { chat_id: update.chat_id, message_id }) {
            drop(sender.send(Err(Error::MessageDeleted(MessageKey { chat_id: update.chat_id, message_id }))));
          }
        }
      }
      Update::updateFile(update) if let Some(sender) = registry.files.get(&update.file.id) => {
        sender.send_replace(FileState::Known(file_progress(&update.file)));
      }
      _ => {}
    }
    drop(registry);
    if let Update::updateAuthorizationState(update) = update
      && let AuthorizationState::authorizationStateClosed = &update.authorization_state
    {
      self.disconnect();
    }
  }

  fn disconnect(&self) {
    let mut registry = self.registry.lock().unwrap();
    registry.accepting_requests = false;
    registry.requests.clear();
    registry.message_sends.clear();
    registry.files.clear();
  }

  fn fail_operations(&self, error: &Arc<serde_json::Error>) {
    let (message_sends, files) = {
      let mut registry = self.registry.lock().unwrap();
      (mem::take(&mut registry.message_sends), mem::take(&mut registry.files))
    };
    for (_, sender) in message_sends {
      drop(sender.send(Err(Error::Json(Arc::clone(error)))));
    }
    for (_, sender) in files {
      sender.send_replace(FileState::Json(Arc::clone(error)));
    }
  }

  fn fail_routing(&self, error: &Arc<serde_json::Error>) {
    let requests = mem::take(&mut self.registry.lock().unwrap().requests);
    for (_, reply) in requests {
      reply.fail(Error::Json(Arc::clone(error)));
    }
    self.fail_operations(error);
    drop(self.updates.send(Err(Error::Json(Arc::clone(error)))));
  }
}

impl Drop for ClientState {
  fn drop(&mut self) {
    // Local cleanup only: destructors never call TDLib or wait for the receiver.
    ROUTER.clients.lock().unwrap().remove(&self.id);
  }
}

fn message_key(message: &types::message) -> MessageKey {
  MessageKey { chat_id: message.chat_id, message_id: message.id }
}

fn message_collision(key: MessageKey) -> Error {
  Error::MessageCollision(key)
}

fn is_pending(message: &types::message) -> bool {
  matches!(message.sending_state, Some(MessageSendingState::messageSendingStatePending(_)))
}

async fn cancelled(cancel: Option<&CancellationToken>) {
  match cancel {
    Some(cancel) => cancel.cancelled().await,
    None => future::pending().await,
  }
}

fn file_progress(file: &types::file) -> FileProgress {
  let &types::file { id: file_id, size, expected_size, ref local, ref remote, .. } = file;
  let &types::localFile { download_offset, downloaded_prefix_size, downloaded_size, is_downloading_active, is_downloading_completed, .. } = local;
  let &types::remoteFile { uploaded_size, is_uploading_active, is_uploading_completed, .. } = remote;

  let download = transfer_state(is_downloading_active, is_downloading_completed);
  let upload = transfer_state(is_uploading_active, is_uploading_completed);
  FileProgress { file_id, size, expected_size, download_offset, downloaded_prefix_size, downloaded_size, download, uploaded_size, upload }
}

fn transfer_state(active: bool, completed: bool) -> TransferState {
  match (active, completed) {
    (_, true) => TransferState::Completed,
    (true, _) => TransferState::Active,
    (false, _) => TransferState::Inactive,
  }
}

fn parse_file(raw: &[u8]) -> Result<types::file> {
  let File::file(file) = serde_json::from_slice(raw)?;
  Ok(file)
}

fn parse_messages(raw: &[u8], many: bool) -> Result<Vec<types::message>> {
  if many {
    let Messages::messages(messages) = serde_json::from_slice(raw)?;
    Ok(messages.messages.unwrap_or_default())
  } else {
    let Message::message(message) = serde_json::from_slice(raw)?;
    Ok(vec![message])
  }
}

#[derive(Deserialize)]
struct IncomingEnvelope<'a> {
  #[serde(rename = "@client_id")]
  client_id: i32,
  #[serde(rename = "@extra")]
  extra: Option<u64>,
  #[serde(rename = "@type")]
  r#type: &'a str,
}

#[derive(Deserialize)]
struct ResponseEnvelope<'a> {
  #[serde(rename = "@type")]
  r#type: &'a str,
}

struct Router {
  clients: Mutex<HashMap<i32, Weak<ClientState>>>,
  receiver: OnceLock<thread::Thread>,
  clients_changed: watch::Sender<()>,
  receive_timeout: AtomicU64,
  native_calls: Mutex<()>,
}

impl Router {
  fn register(&self, id: i32, client: Weak<ClientState>) {
    self.clients.lock().unwrap().insert(id, client);
    self.receiver.get_or_init(|| thread::spawn(receive_loop).thread().clone()).unpark();
    self.clients_changed.send_replace(());
  }

  async fn unregister(&self, id: i32) {
    let mut changed = self.clients_changed.subscribe();
    {
      let mut clients = self.clients.lock().unwrap();
      clients.remove(&id);
      if !clients.is_empty() {
        return;
      }
      // Consume the current watch version while holding the registry lock. The
      // next version is necessarily a later receiver-idle or registration event.
      changed.borrow_and_update();
    }
    drop(changed.changed().await);
  }

  fn route(&self, raw: &[u8]) {
    let IncomingEnvelope { client_id, extra, r#type } = match serde_json::from_slice(raw) {
      Ok(envelope) => envelope,
      Err(error) => return self.broadcast(error),
    };
    let Some(client) = self.clients.lock().unwrap().get(&client_id).and_then(Weak::upgrade) else { return };
    match extra {
      Some(extra) => client.complete_request(extra, r#type, raw),
      None => client.route_update(r#type, raw),
    }
  }

  fn broadcast(&self, error: serde_json::Error) {
    let error = Arc::new(error);
    // Upgrade under the map lock, then release it before reporting. Reporting can
    // drop a final ClientState, whose destructor removes its weak map entry.
    let clients: Vec<_> = self.clients.lock().unwrap().values().filter_map(Weak::upgrade).collect();
    for client in clients {
      client.fail_routing(&error);
    }
  }

  fn receive(&self) {
    // The guard also protects the lifetime of TDLib's shared response buffer
    // through envelope parsing and full routing.
    let _native_call = self.native_calls.lock().unwrap();
    let timeout = f64::from_bits(self.receive_timeout.load(Ordering::Relaxed));
    // SAFETY: This is the process-wide sole caller of `td_receive`.
    let raw = unsafe { td_sys::td_receive(timeout) };
    if raw.is_null() {
      return;
    }
    // SAFETY: TDLib returned a non-null NUL-terminated buffer valid until the next receive.
    self.route(unsafe { CStr::from_ptr(raw) }.to_bytes());
  }
}

static ROUTER: LazyLock<Router> = LazyLock::new(|| {
  let (clients_changed, _) = watch::channel(());
  let (clients, receiver, native_calls) = Default::default();
  let receive_timeout = 1f64.to_bits().into();
  Router { clients, receiver, clients_changed, receive_timeout, native_calls }
});

fn receive_loop() {
  loop {
    if ROUTER.clients.lock().unwrap().is_empty() {
      // Publish the empty observation before parking so the last graceful
      // shutdown can finish; registration publishes too and unparks this thread.
      ROUTER.clients_changed.send_replace(());
      thread::park();
    } else {
      ROUTER.receive();
    }
  }
}

fn parse_td_error(raw: &[u8]) -> Error {
  serde_json::from_slice(raw).map_or_else(Into::into, Error::Td)
}

/// Executes a `TDLib` function through the client-independent synchronous interface.
///
/// Only functions documented by `TDLib` as synchronously executable are meaningful
/// here. The request and response retain generated typing, and `TDLib` `error`
/// objects become [`Error::Td`]. Calls are serialized with the process-wide
/// receiver because either native function may invalidate the other's response
/// buffer.
pub fn execute<F: Function>(request: &F) -> Result<F::Return> {
  let mut request = serde_json::to_vec(request)?;
  request.push(0);
  let _native_call = ROUTER.native_calls.lock().unwrap();
  // SAFETY: `request` is live and NUL-terminated, and `native_calls` excludes calls that can invalidate the returned buffer.
  let raw = unsafe { td_sys::td_execute(request.as_ptr().cast()) };
  if raw.is_null() {
    return Err(Error::UnexpectedResponse("synchronous request returned null"));
  }
  // SAFETY: TDLib returned a non-null NUL-terminated buffer that remains valid while `native_calls` is held.
  let raw = unsafe { CStr::from_ptr(raw) }.to_bytes();
  let ResponseEnvelope { r#type } = serde_json::from_slice(raw)?;
  if let "error" = r#type { Err(parse_td_error(raw)) } else { serde_json::from_slice(raw).map_err(Into::into) }
}

/// Sets the process-wide wait used by the next native receive call.
///
/// The default is one second. A change does not interrupt a receive already in
/// progress. Short values reduce final shutdown and synchronous-execution latency
/// at the cost of more native calls; zero causes polling while clients are live.
/// This is transport tuning, not a request timeout.
pub fn set_receive_timeout(timeout: Duration) {
  ROUTER.receive_timeout.store(timeout.as_secs_f64().to_bits(), Ordering::Relaxed);
}

/// Sets `TDLib`'s process-wide native log verbosity level.
pub fn set_log_level(level: i32) {
  // SAFETY: The call passes no pointers or borrowed storage.
  unsafe { td_sys::td_set_log_verbosity_level(level) };
}

/// Builds conventional `TDLib` parameters rooted at `directory`.
///
/// Databases are stored under `db`, downloaded files under `files`, and the file,
/// chat-info, and message databases are enabled. Applications remain free to
/// modify the returned generated struct before constructing [`Client`].
pub fn params(api_id: i32, api_hash: impl Into<String>, directory: impl AsRef<Path>) -> fns::setTdlibParameters {
  let directory = directory.as_ref();
  fns::setTdlibParameters {
    api_id,
    api_hash: api_hash.into(),
    database_directory: directory.join("db").display().to_string(),
    files_directory: directory.join("files").display().to_string(),
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
mod tests;
