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
//! one owning [`Client`], cloneable non-owning [`Sender`] values, and retained
//! non-cloneable operations for message sends and file transfers.
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
//! the receiver thread before waking the requester, then expose [`MessageSend`]
//! operations. [`MessageSend::wait`] observes authoritative success, failure, or
//! non-cache deletion without requiring the application to poll [`Client::recv`].
//! The original terminal update is still enqueued unchanged.
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
//! let mut send = sender.send_message(&request).await?;
//! let final_message = send.wait().await?;
//! ```
//!
//! Cancellation consumes the retained operation because only one path may decide
//! its cleanup. [`MessageSend::cancel`] deletes a still-pending temporary ID and
//! awaits the terminal result. It returns `None` if deletion wins and
//! `Some(final_message)` if authoritative success was already observed; it never
//! explicitly deletes that successful final ID. This is not server-atomic: `TDLib`
//! itself may delete a concurrently accepted message after removing its pending
//! record.
//!
//! Dropping a message operation performs no cancellation. Its registry entry
//! remains until the native terminal update, so a caller that retained its
//! [`MessageKey`] may reattach with [`Sender::track_message`].
//!
//! # Files
//!
//! [`Sender::track_file`] creates a sparse, future-only [`FileWatch`] for a known
//! file ID. Watches retain only copyable [`FileProgress`] and coalesce intermediate
//! observations; every full `updateFile` remains available through
//! [`Client::recv`]. Use the generated `getFile` function separately when a full
//! current snapshot is required.
//!
//! [`Sender::download`] forces `downloadFile.synchronous = true`. In `TDLib` this is
//! an asynchronous request promise whose response becomes ready only when the
//! requested full file or exact byte range is locally available. [`Download::wait`]
//! therefore has authoritative completion and failure semantics, while
//! [`Download::progress`] exposes coalesced updates. [`Download::cancel`] awaits
//! both `cancelDownloadFile` and the original download response and reports
//! `Some(file)` when completion won the race.
//!
//! [`Sender::upload`] starts `preliminaryUploadFile` and waits only long enough to
//! receive and bind its file ID. `TDLib` exposes no authoritative standalone result
//! for that preliminary upload: [`Upload::wait`] returns the first observed
//! non-active progress state without labelling it success or failure. Completion
//! belongs to the operation that consumes the uploaded file, commonly a tracked
//! message send. [`Upload::cancel`] awaits `cancelPreliminaryUploadFile`.
//!
//! Dropping any file operation abandons local observation and performs no native
//! cancellation. Explicit cancellation makes progress only while its future is
//! polled.
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

use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::ffi::CStr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock, Weak};
use std::time::Duration;
use std::{fmt, mem, result, thread};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use td_types::enums::{AuthorizationState, Update};
use td_types::traits::Function;
use td_types::{enums, fns, types};

/// A `td-client` operation result.
pub type Result<T = ()> = result::Result<T, Error>;

/// A failure at the typed `TDLib` boundary or in a retained operation.
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
  /// `TDLib` reported the terminal failure of a tracked message send.
  #[error("message {} in chat {} failed: {} {}", .0.old_message_id, .0.message.chat_id, .0.error.code, .0.error.message)]
  MessageFailed(Arc<types::updateMessageSendFailed>),
  /// A non-cache deletion removed a tracked temporary message before success.
  #[error("message {} in chat {} was deleted while being sent", .0.message_id, .0.chat_id)]
  MessageDeleted(MessageKey),
  /// A tracked response attempted to reuse an existing temporary-message key.
  #[error("message {} in chat {} is already being tracked", .0.message_id, .0.chat_id)]
  MessageCollision(MessageKey),
  /// No pending message currently has the requested key.
  #[error("message {} in chat {} isn't pending", .0.message_id, .0.chat_id)]
  MessageNotPending(MessageKey),
  /// `TDLib` returned a structurally impossible result or an operation was awaited twice.
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

/// The stable local identity of a pending message send.
///
/// `TDLib` replaces `message_id` on success. Terminal success and failure updates
/// refer back to this temporary ID through `old_message_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageKey {
  /// The chat containing the pending message.
  pub chat_id: i64,
  /// The temporary message ID returned by the direct send response.
  pub message_id: i64,
}

/// The latest retained state of a message send.
#[derive(Debug)]
pub enum MessageState {
  /// `TDLib` returned a temporary message and no terminal update has been observed.
  Pending(types::message),
  /// The direct response was already final or `TDLib` emitted send success.
  Succeeded(types::message),
  /// `TDLib` emitted a terminal send failure.
  Failed(Arc<types::updateMessageSendFailed>),
  /// A non-cache deletion removed the temporary message.
  Deleted,
  /// A malformed update made further tracking unreliable.
  Json(Arc<serde_json::Error>),
}

/// A retained, non-owning normal-message send operation.
///
/// Observation borrows the operation and cancellation consumes it. Dropping the
/// value is inert; retain [`MessageKey`] and use [`Sender::track_message`] to
/// reattach before the native terminal update if needed.
#[must_use = "call wait().await to observe the send outcome"]
pub struct MessageSend {
  key: MessageKey,
  client: Weak<ClientState>,
  states: watch::Receiver<MessageState>,
}

impl MessageSend {
  /// Returns the chat and temporary message ID used for terminal correlation.
  pub fn key(&self) -> MessageKey {
    self.key
  }

  /// Borrows the latest state without waiting for a change.
  pub fn state(&self) -> watch::Ref<'_, MessageState> {
    self.states.borrow()
  }

  /// Waits for authoritative success, failure, or deletion.
  ///
  /// Successful messages are cloned from the retained watch state so the
  /// operation remains observable and can participate in `tokio::select!`.
  pub async fn wait(&mut self) -> Result<types::message> {
    let not_pending = |s: &MessageState| !matches!(s, MessageState::Pending(_));
    let state = self.states.wait_for(not_pending).await.map_err(|_| Error::Disconnected)?;
    match &*state {
      MessageState::Pending(_) => unreachable!(),
      MessageState::Succeeded(message) => Ok(message.clone()),
      MessageState::Failed(update) => Err(Error::MessageFailed(Arc::clone(update))),
      MessageState::Deleted => Err(Error::MessageDeleted(self.key)),
      MessageState::Json(error) => Err(Error::Json(Arc::clone(error))),
    }
  }

  /// Requests cancellation if the message is still pending and awaits its outcome.
  ///
  /// Returns `Ok(None)` when deletion wins and `Ok(Some(message))` when an
  /// authoritative success wins the race. A successful final message is never
  /// explicitly deleted by this method.
  pub async fn cancel(mut self) -> Result<Option<types::message>> {
    if self.pending()? {
      let cancellation = match self.client.upgrade() {
        Some(client) => client.delete_message(self.key).await,
        None => Err(Error::Disconnected),
      };
      // Deletion can fail after a terminal update won the race. Preserve that
      // authoritative result; surface the deletion error only while still pending.
      if let Err(error) = cancellation
        && self.pending()?
      {
        return Err(error);
      }
    }
    match self.wait().await {
      Err(Error::MessageDeleted(_)) => Ok(None),
      Ok(message) => Ok(Some(message)),
      Err(error) => Err(error),
    }
  }

  fn pending(&mut self) -> Result<bool> {
    match &*self.states.borrow_and_update() {
      MessageState::Json(error) => Err(Error::Json(Arc::clone(error))),
      MessageState::Pending(_) => Ok(true),
      _ => Ok(false),
    }
  }
}

/// The latest retained state of a watched file ID.
#[derive(Debug)]
pub enum FileState {
  /// No direct response or `updateFile` has seeded this future-only watch yet.
  Unknown,
  /// The latest coalesced transfer progress.
  Known(FileProgress),
  /// A malformed update made further observation unreliable.
  Json(Arc<serde_json::Error>),
}

/// The copy-only transfer fields retained from a `TDLib` `file` object.
///
/// Paths, remote identifiers, and other owned metadata remain in the original
/// application update and are not duplicated here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileProgress {
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

/// A retained, coalescing observer for one `TDLib` file ID.
///
/// A watch created with [`Sender::track_file`] is future-only and starts in
/// [`FileState::Unknown`]. Direct upload and download operations seed their watch
/// from the correlated file response before returning it.
pub struct FileWatch {
  id: i32,
  states: watch::Receiver<FileState>,
}

impl FileWatch {
  /// Returns the watched `TDLib` file ID.
  pub fn id(&self) -> i32 {
    self.id
  }

  /// Borrows the latest progress state without waiting for a change.
  pub fn state(&self) -> watch::Ref<'_, FileState> {
    self.states.borrow()
  }

  /// Waits until `terminal` accepts a known progress snapshot.
  ///
  /// Passive file IDs have no inherent transfer direction or terminal policy, so
  /// the caller supplies the condition. The predicate borrows the retained value;
  /// the accepted snapshot is then returned by value.
  pub async fn wait(&mut self, mut terminal: impl FnMut(&FileProgress) -> bool) -> Result<FileProgress> {
    loop {
      match &*self.states.borrow_and_update() {
        FileState::Known(progress) if terminal(progress) => return Ok(*progress),
        FileState::Json(error) => return Err(Error::Json(Arc::clone(error))),
        _ => {}
      }
      if self.states.changed().await.is_err() {
        return match &*self.states.borrow() {
          FileState::Json(error) => Err(Error::Json(Arc::clone(error))),
          _ => Err(Error::Disconnected),
        };
      }
    }
  }
}

/// A retained exact-range `downloadFile` operation.
///
/// Construction forces `TDLib`'s `synchronous` request flag. The Rust API remains
/// asynchronous; the correlated response is retained until the requested range is
/// available, fails, or is cancelled.
pub struct Download {
  client: Weak<ClientState>,
  progress: FileWatch,
  response: Option<oneshot::Receiver<Result<types::file>>>,
}

impl Download {
  /// Returns the operation's coalesced progress observer.
  pub fn progress(&self) -> &FileWatch {
    &self.progress
  }

  /// Waits for `TDLib`'s authoritative download response.
  ///
  /// The response can be taken once. Borrowing rather than consuming the operation
  /// permits a caller to race this future with a cancellation source and then call
  /// [`Self::cancel`].
  pub async fn wait(&mut self) -> Result<types::file> {
    let response = self.response.take().ok_or(Error::UnexpectedResponse("download was already awaited"))?;
    response.await.map_err(|_| Error::Disconnected)?
  }

  /// Requests download cancellation and awaits both sides of the race.
  ///
  /// Returns `Some(file)` if the original exact-range download completed and
  /// `None` if cancellation completed it with `TDLib`'s cancellation error.
  pub async fn cancel(self) -> Result<Option<types::file>> {
    let Self { client, progress, response } = self;
    let response = response.ok_or(Error::UnexpectedResponse("download was already awaited"))?;
    let Some(client) = client.upgrade() else {
      return response.await.map_err(|_| Error::Disconnected)?.map(Some);
    };
    let cancel = fns::cancelDownloadFile { file_id: progress.id, only_if_pending: false };
    let cancel = client.execute_request(&cancel, false).await;
    match (cancel, response.await.map_err(|_| Error::Disconnected)?) {
      (_, Ok(file)) => Ok(Some(file)),
      (Ok(_), Err(Error::Td(_))) => Ok(None),
      (Err(error), Err(_)) | (Ok(_), Err(error)) => Err(error),
    }
  }
}

/// A retained `preliminaryUploadFile` operation.
///
/// The direct response supplies the file ID but does not mean the preliminary
/// upload completed. Progress is observed through [`FileWatch`].
pub struct Upload {
  client: Weak<ClientState>,
  progress: FileWatch,
}

impl Upload {
  /// Returns the operation's coalesced progress observer.
  pub fn progress(&self) -> &FileWatch {
    &self.progress
  }

  /// Waits for the upload to become non-active and returns that observation.
  ///
  /// `TDLib` exposes neither authoritative standalone success nor a failure reason
  /// for preliminary uploads, so this method deliberately does not infer either.
  pub async fn wait(&mut self) -> Result<FileProgress> {
    self.progress.wait(|progress| progress.upload != TransferState::Active).await
  }

  /// Requests and awaits `cancelPreliminaryUploadFile`.
  pub async fn cancel(self) -> Result {
    let client = self.client.upgrade().ok_or(Error::Disconnected)?;
    let cancel = fns::cancelPreliminaryUploadFile { file_id: self.progress.id };
    client.execute_request(&cancel, false).await?;
    Ok(())
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

  /// Starts tracking one actual non-preview normal-message send.
  ///
  /// The request must return `enums::Message` and obey the tracked-request
  /// invariant described in this crate's module-level documentation. The direct
  /// response is parsed and bound before this method returns.
  pub async fn send_message<F: Function<Return = enums::Message>>(&self, request: &F) -> Result<MessageSend> {
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    let sends = client.track_messages(request, false).await?;
    let [send] = sends.try_into().map_err(|_| Error::UnexpectedResponse("expected one message"))?;
    Ok(send)
  }

  /// Starts tracking a batch of actual non-preview normal-message sends.
  ///
  /// Registration is atomic: duplicate temporary keys or collisions with existing
  /// sends fail without registering only part of the batch. Returned operations
  /// preserve direct-response order and settle independently.
  pub async fn send_messages<F: Function<Return = enums::Messages>>(&self, request: &F) -> Result<Vec<MessageSend>> {
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    client.track_messages(request, true).await
  }

  /// Attaches another operation to a currently pending message key.
  ///
  /// Reattachment is possible after dropping another observer because a pending
  /// registry entry remains until its native terminal update.
  pub fn track_message(&self, key: MessageKey) -> Result<MessageSend> {
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    client.track_message(key)
  }

  /// Creates a future-only coalescing watch for `file_id`.
  ///
  /// This performs no `getFile` request. Use [`Self::send`] with the generated
  /// function when a full current snapshot is also needed.
  pub fn track_file(&self, file_id: i32) -> Result<FileWatch> {
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    Ok(client.track_file(file_id))
  }

  /// Starts an exact-range download and returns immediately after submission.
  ///
  /// Any caller-supplied `synchronous` value is replaced with `true`, selecting
  /// `TDLib`'s authoritative completion promise. Request serialization can fail
  /// before an operation is returned.
  pub fn download(&self, mut request: fns::downloadFile) -> Result<Download> {
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    // TDLib's synchronous flag delays this request's response until its exact
    // range is available; it does not block this Rust thread.
    request.synchronous = true;
    let progress = client.track_file(request.file_id);
    let (reply, response) = oneshot::channel();
    client.submit_request(&request, false, PendingReply::Download(reply))?;
    Ok(Download { client: Arc::downgrade(&client), progress, response: Some(response) })
  }

  /// Starts a preliminary upload and returns after its file ID is bound.
  ///
  /// This method is asynchronous because `TDLib` assigns the observable file ID in
  /// the direct response. It does not wait for upload inactivity or completion.
  pub async fn upload(&self, request: &fns::preliminaryUploadFile) -> Result<Upload> {
    let client = self.0.upgrade().ok_or(Error::Disconnected)?;
    let (reply, response) = oneshot::channel();
    client.submit_request(request, false, PendingReply::Upload(reply))?;
    response.await.map_err(|_| Error::Disconnected)?
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
  Messages { many: bool, reply: oneshot::Sender<Result<Vec<MessageSend>>> },
  Download(oneshot::Sender<Result<types::file>>),
  Upload(oneshot::Sender<Result<Upload>>),
}

impl PendingReply {
  fn fail(self, error: Error) {
    match self {
      Self::Request(reply) => drop(reply.send(Err(error))),
      Self::Messages { reply, .. } => drop(reply.send(Err(error))),
      Self::Download(reply) => drop(reply.send(Err(error))),
      Self::Upload(reply) => drop(reply.send(Err(error))),
    }
  }
}

#[derive(Default)]
struct ClientRegistry {
  accepting_requests: bool,
  requests: HashMap<u64, PendingReply>,
  message_sends: HashMap<MessageKey, watch::Sender<MessageState>>,
  file_watches: HashMap<i32, watch::Sender<FileState>>,
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

  async fn track_messages<F: Function>(&self, request: &F, many: bool) -> Result<Vec<MessageSend>> {
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
      PendingReply::Download(reply) => {
        let result = parse_file(raw).inspect(|file| self.publish_file(file));
        drop(reply.send(result));
      }
      PendingReply::Upload(reply) => {
        let client = Arc::downgrade(self);
        // The preliminary response is the first authoritative source of its file
        // ID, so seed the watch before exposing the operation.
        let result = parse_file(raw).map(|file| Upload { client, progress: self.bind_file(&file) });
        drop(reply.send(result));
      }
    }
  }

  fn bind_messages(self: &Arc<Self>, messages: Vec<types::message>) -> Result<Vec<MessageSend>> {
    // Validate the whole batch before inserting anything. Partial registration
    // would strand the unregistered messages if a later key collided.
    let mut keys: Vec<_> = messages.iter().filter(|message| is_pending(message)).map(message_key).collect();
    keys.sort_unstable();
    if let Some([key, _]) = keys.array_windows().find(|[a, b]| a == b) {
      return Err(Error::MessageCollision(*key));
    }

    let mut registry = self.registry.lock().unwrap();
    if let Some(key) = keys.into_iter().find(|key| registry.message_sends.contains_key(key)) {
      return Err(Error::MessageCollision(key));
    }

    let client = Arc::downgrade(self);
    let sends = messages.into_iter().map(|message| {
      let key = message_key(&message);
      let pending = is_pending(&message);
      let initial_state = if pending { MessageState::Pending(message) } else { MessageState::Succeeded(message) };
      let (tx, rx) = watch::channel(initial_state);
      if pending {
        registry.message_sends.insert(key, tx);
      }
      MessageSend { key, client: client.clone(), states: rx }
    });

    Ok(sends.collect())
  }

  fn track_message(self: &Arc<Self>, key: MessageKey) -> Result<MessageSend> {
    match self.registry.lock().unwrap().message_sends.get(&key) {
      Some(states) => Ok(MessageSend { key, client: Arc::downgrade(self), states: states.subscribe() }),
      None => Err(Error::MessageNotPending(key)),
    }
  }

  fn track_file(self: &Arc<Self>, id: i32) -> FileWatch {
    let mut registry = self.registry.lock().unwrap();
    registry.file_watches.retain(|_, states| states.receiver_count() > 0);
    let states = registry.file_watches.entry(id).or_insert_with(|| watch::channel(FileState::Unknown).0).subscribe();
    FileWatch { id, states }
  }

  fn bind_file(self: &Arc<Self>, file: &types::file) -> FileWatch {
    let file_id = file.id;
    let progress = file_progress(file);
    let mut registry = self.registry.lock().unwrap();
    registry.file_watches.retain(|_, states| states.receiver_count() > 0);
    let sender = registry.file_watches.entry(file_id).or_insert_with(|| watch::channel(FileState::Unknown).0);
    sender.send_replace(FileState::Known(progress));
    let states = sender.subscribe();
    FileWatch { id: file_id, states }
  }

  fn publish_file(&self, file: &types::file) {
    let mut registry = self.registry.lock().unwrap();
    if let Entry::Occupied(entry) = registry.file_watches.entry(file.id) {
      match entry.get().receiver_count() {
        0 => drop(entry.remove()),
        _ => drop(entry.get().send_replace(FileState::Known(file_progress(file)))),
      }
    }
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
        if let Some(sender) = registry.message_sends.remove(&key) {
          // The application queue must receive the original update unchanged, so
          // retained message state necessarily owns a clone of the final message.
          sender.send_replace(MessageState::Succeeded(update.message.clone()));
        }
      }
      Update::updateMessageSendFailed(update) => {
        let key = MessageKey { chat_id: update.message.chat_id, message_id: update.old_message_id };
        if let Some(sender) = registry.message_sends.remove(&key) {
          // Share the cloned failure between all operation observers while the
          // original update continues to the application queue.
          sender.send_replace(MessageState::Failed(Arc::new(update.clone())));
        }
      }
      Update::updateDeleteMessages(update) if !update.from_cache => {
        for &message_id in &update.message_ids {
          if let Some(sender) = registry.message_sends.remove(&MessageKey { chat_id: update.chat_id, message_id }) {
            sender.send_replace(MessageState::Deleted);
          }
        }
      }
      Update::updateFile(update) if let Some(sender) = registry.file_watches.get(&update.file.id) => {
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
    registry.file_watches.clear();
  }

  fn fail_operations(&self, error: &Arc<serde_json::Error>) {
    let (message_sends, file_watches) = {
      let mut registry = self.registry.lock().unwrap();
      (mem::take(&mut registry.message_sends), mem::take(&mut registry.file_watches))
    };
    for (_, sender) in message_sends {
      sender.send_replace(MessageState::Json(Arc::clone(error)));
    }
    for (_, sender) in file_watches {
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

fn is_pending(message: &types::message) -> bool {
  matches!(message.sending_state, Some(enums::MessageSendingState::messageSendingStatePending(_)))
}

fn file_progress(file: &types::file) -> FileProgress {
  let &types::file { size, expected_size, ref local, ref remote, .. } = file;
  let &types::localFile { download_offset, downloaded_prefix_size, downloaded_size, is_downloading_active, is_downloading_completed, .. } = local;
  let &types::remoteFile { uploaded_size, is_uploading_active, is_uploading_completed, .. } = remote;

  let download = transfer_state(is_downloading_active, is_downloading_completed);
  let upload = transfer_state(is_uploading_active, is_uploading_completed);
  FileProgress { size, expected_size, download_offset, downloaded_prefix_size, downloaded_size, download, uploaded_size, upload }
}

fn transfer_state(active: bool, completed: bool) -> TransferState {
  match (active, completed) {
    (_, true) => TransferState::Completed,
    (true, _) => TransferState::Active,
    (false, _) => TransferState::Inactive,
  }
}

fn parse_file(raw: &[u8]) -> Result<types::file> {
  let enums::File::file(file) = serde_json::from_slice(raw)?;
  Ok(file)
}

fn parse_messages(raw: &[u8], many: bool) -> Result<Vec<types::message>> {
  if many {
    let enums::Messages::messages(messages) = serde_json::from_slice(raw)?;
    Ok(messages.messages.unwrap_or_default())
  } else {
    let enums::Message::message(message) = serde_json::from_slice(raw)?;
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
