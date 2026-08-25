//! Asynchronous, typed access to `TDLib`'s client-ID JSON interface.
//!
//! This crate connects the generated requests and responses in `td_types::fns`
//! and `td_types::enums` to `TDLib`'s process-wide JSON transport. It preserves
//! `TDLib`'s update order,
//! correlates concurrent requests, routes multiple clients through one receiver
//! thread, and makes the native client's asynchronous shutdown protocol explicit.
//!
//! # Ownership and capabilities
//!
//! A live `TDLib` instance has exactly one owning [`Client`]. The client is
//! intentionally not cloneable: it owns the ordered update stream and is the only
//! value that can complete shutdown. [`Client::sender`] creates a cloneable
//! [`Sender`] for detached request tasks. A sender holds only a weak reference, so
//! it neither keeps the client alive nor grants access to updates or shutdown.
//!
//! ```no_run
//! use td_client::Client;
//! use td_types::fns;
//!
//! # async fn run() -> td_client::Result {
//! let params = td_client::params(123456, "api hash", ".td");
//! let mut client = Client::bot(params, "bot token").await?;
//! let sender = client.sender();
//!
//! let _me = sender.send(&fns::getMe {}).await?;
//! if let Some(_update) = client.recv().await? {
//!   // Dispatch the update without blocking the receive loop on slow work.
//! }
//!
//! drop(sender);
//! client.shutdown().await
//! # }
//! ```
//!
//! Dropping `Client` revokes new requests, including requests attempted through
//! surviving senders, but it does not call `TDLib` or finish native shutdown. Call
//! [`Client::shutdown`] and handle its result before process exit.
//!
//! # Requests and updates
//!
//! [`Sender::send`] returns a function's direct correlated response. For most
//! `TDLib` functions that is the complete operation. A `sendMessage` direct response
//! is different: it contains a temporary local message and is followed by an
//! authoritative success, failure, or deletion update. Use
//! [`Sender::send_message`] or [`Sender::send_message_until`] when that terminal
//! outcome is required. Message-edit functions already complete through their
//! direct response and should use [`Sender::send`].
//!
//! [`Client::recv`] returns ordinary updates in `TDLib` order and hides authorization
//! transitions. During an interactive login, [`Client::recv_auth`] returns those
//! transitions and buffers intervening ordinary updates for later calls to
//! `recv`. The queue is deliberately unbounded: the synchronous native receiver
//! cannot await capacity, and this library does not invent an update-dropping or
//! overflow policy.
//!
//! Dropping any request future only abandons local observation. It never cancels
//! work already submitted to `TDLib`. Deadline-based message sending performs
//! explicit compensating deletion while its future continues to be polled.
//!
//! # Process-wide receiver
//!
//! `TDLib` multiplexes every client through one `td_receive` function. This crate
//! therefore runs exactly one process-lifetime receiver thread. It routes required
//! `@client_id` values to weak client entries, then uses `@extra` to complete a
//! request-local one-shot or enqueues an uncorrelated update. When no clients are
//! registered, the thread parks without calling `TDLib`. [`set_receive_timeout`]
//! configures the next native receive wait for the whole process; it is not a
//! request deadline.
//!
//! Graceful [`Client::shutdown`] sends the generated `close` function through the
//! correlated request path, waits for `authorizationStateClosed`, disconnects all
//! local capabilities, and then waits until the receiver has crossed a safe idle
//! or new-owner boundary. Errors are reported to the caller; destructors perform
//! no blocking or native work.

use std::collections::{HashMap, VecDeque};
use std::ffi::CStr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock, Weak};
use std::time::{Duration, Instant};
use std::{fmt, mem, result, thread};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use td_types::enums::{AuthorizationState, Update};
use td_types::traits::Function;
use td_types::{fns, types};

mod message_send;

/// A `td-client` operation result.
pub type Result<T = ()> = result::Result<T, Error>;

/// An error produced while configuring, using, or shutting down a client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// `TDLib` returned its typed `error` object for a request or event.
  #[error("TDLib: {} {}", .0.code, .0.message)]
  Td(types::error),
  /// A request could not be serialized or a `TDLib` object could not be decoded.
  ///
  /// The parse error is shared so an unrouteable process-wide envelope can be
  /// reported to every live client without converting or duplicating it.
  #[error("JSON: {0}")]
  Json(#[source] Arc<serde_json::Error>),
  /// Bot authorization entered a state the bot-token flow cannot handle.
  #[error("unexpected auth state: {0:?}")]
  Auth(AuthorizationState),
  /// A tracked `sendMessage` requested a preview, which has no terminal send result.
  #[error("message previews have no send result; use Sender::send")]
  MessagePreview,
  /// `TDLib` reported the authoritative failure of a tracked message send.
  #[error("message {} in chat {} failed: {} {}", .0.old_message_id, .0.message.chat_id, .0.error.code, .0.error.message)]
  MessageFailed(Box<types::updateMessageSendFailed>),
  /// The temporary message for a tracked send was deleted before it completed.
  #[error("message {message_id} in chat {chat_id} was deleted while being sent")]
  MessageDeleted {
    /// Chat in which the send was pending.
    chat_id: i64,
    /// Temporary message ID that was deleted.
    message_id: i64,
  },
  /// A deadline expired and compensating deletion of the send completed.
  ///
  /// `message_id` is the temporary message identifier returned by `sendMessage`.
  #[error("message {message_id} in chat {chat_id} exceeded its send deadline and was deleted")]
  MessageDeadline {
    /// Chat in which the deadline expired.
    chat_id: i64,
    /// Temporary message ID returned by the direct response.
    message_id: i64,
  },
  /// `TDLib` returned a temporary message key already assigned to another pending send.
  #[error("TDLib reused pending message {message_id} in chat {chat_id}")]
  MessageCorrelation {
    /// Chat containing the reused temporary ID.
    chat_id: i64,
    /// Temporary message ID already bound to a tracked send.
    message_id: i64,
  },
  /// The owning client or its response path is no longer available.
  #[error("client disconnected")]
  Disconnected,
}

impl From<serde_json::Error> for Error {
  fn from(error: serde_json::Error) -> Self {
    Self::Json(Arc::new(error))
  }
}

/// A cloneable, non-owning capability for issuing `TDLib` requests.
///
/// A sender stores a weak reference to its [`Client`]. It is suitable for
/// detached tasks, but it neither extends the client's lifetime nor grants update
/// or shutdown access. Once the owner is dropped or its close operation wins the
/// request gate, new requests fail with [`Error::Disconnected`]. A request that
/// wins the race against close may still finish.
#[derive(Clone)]
pub struct Sender(Weak<ClientState>);

impl Sender {
  /// Executes `request` and returns its direct correlated `TDLib` response.
  ///
  /// Message edits complete through this direct response after their upload, server operation, and updates finish.
  /// They do not use the message-send tracker. Use [`Self::send_message`] for `sendMessage` when the later authoritative result is required.
  ///
  /// # Errors
  ///
  /// Returns [`Error::Td`] for a `TDLib` error response, [`Error::Json`] for request or response encoding failures, and [`Error::Disconnected`] if the
  /// client cannot accept or complete the request.
  ///
  /// # Cancellation
  ///
  /// Dropping this future does not cancel a request already sent to `TDLib`. If the client remains live, its eventual response is removed from the
  /// correlation table and discarded.
  pub async fn send<F: Function>(&self, request: &F) -> Result<F::Return> {
    let state = self.0.upgrade().ok_or(Error::Disconnected)?;
    state.execute_request(request, false).await
  }

  /// Sends a message and waits for its authoritative success, failure, or deletion update.
  ///
  /// The successful value is the final message from `updateMessageSendSucceeded`, not the temporary message in the direct `sendMessage` response. The
  /// terminal update is also delivered unchanged through [`Client::recv`].
  ///
  /// # Errors
  ///
  /// In addition to request and decoding failures, returns [`Error::MessagePreview`] for preview-only sends, [`Error::MessageFailed`] for
  /// `updateMessageSendFailed`, and [`Error::MessageDeleted`] when a non-cache deletion removes the temporary message.
  ///
  /// # Cancellation
  ///
  /// Dropping this future unregisters its local waiter but does not cancel or delete the native send.
  pub async fn send_message(&self, request: &fns::sendMessage) -> Result<types::message> {
    let state = self.0.upgrade().ok_or(Error::Disconnected)?;
    state.send_message(request, None).await
  }

  /// Sends a message until `deadline`, then deletes the pending or concurrently sent message.
  ///
  /// The direct response is awaited first because it supplies the temporary ID needed for cleanup. Once the deadline wins, this method deletes that ID,
  /// waits for the terminal send result, and also deletes the final ID if success raced with cancellation. It returns [`Error::MessageDeadline`] only
  /// after cleanup succeeds; a cleanup failure is preserved instead.
  ///
  /// Cancellation is compensating rather than server-atomic: recipients can briefly observe a message that concurrently succeeds before deletion. The
  /// future can therefore resolve after `deadline` while it awaits the temporary ID, terminal result, or deletion responses.
  ///
  /// # Errors
  ///
  /// Returns the errors described by [`Self::send_message`], [`Error::MessageDeadline`] after successful deadline cleanup, or the `TDLib`/transport error
  /// that prevented cleanup.
  ///
  /// # Cancellation
  ///
  /// Dropping this future unregisters local tracking and also stops any compensating cancellation still in progress.
  pub async fn send_message_until(&self, request: &fns::sendMessage, deadline: Instant) -> Result<types::message> {
    let state = self.0.upgrade().ok_or(Error::Disconnected)?;
    state.send_message(request, Some(deadline)).await
  }
}

#[must_use = "call shutdown().await to finish TDLib cleanly"]
/// The sole owner of a `TDLib` client, its update stream, and its shutdown right.
///
/// `Client` is intentionally not [`Clone`]. Shared request-only access comes from
/// [`Client::sender`], update and authorization consumption require `&mut Client`,
/// and graceful shutdown consumes the owner. Call [`Client::shutdown`] before
/// dropping a live client.
pub struct Client {
  /// Shared transport state; this is the session's sole durable strong owner.
  state: Arc<ClientState>,
  /// Ordered handoff from the synchronous process-wide receiver thread.
  events: mpsc::UnboundedReceiver<Result<Update>>,
  /// Ordinary updates temporarily displaced while authorization is consumed.
  buffered_updates: VecDeque<Update>,
  /// Whether `authorizationStateClosed` has already crossed this event receiver.
  closed: bool,
}

impl fmt::Debug for Client {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Client").field("id", &self.state.id).finish_non_exhaustive()
  }
}

impl Client {
  /// Creates a client and applies the supplied `TDLib` parameters.
  ///
  /// This configures a fresh native client but does not complete authorization.
  /// Use [`Self::recv_auth`] plus generated authentication functions for an
  /// interactive login, or [`Self::bot`] for the bot-token flow.
  ///
  /// If parameter setup fails, the constructor attempts graceful shutdown and
  /// returns the original setup error.
  ///
  /// # Errors
  ///
  /// Returns a serialization error, `TDLib`'s parameter error, or
  /// [`Error::Disconnected`] if the native response path closes.
  pub async fn new(params: fns::setTdlibParameters) -> Result<Self> {
    let client = Self::create_unconfigured();
    if let Err(err) = client.state.execute_request(&params, false).await {
      let _ = client.shutdown().await;
      return Err(err);
    }
    Ok(client)
  }

  /// Creates, configures, and authorizes a bot client.
  ///
  /// The helper handles the parameter and bot-token states and returns only once
  /// `TDLib` reaches `authorizationStateReady`. Any other authorization state is
  /// returned as [`Error::Auth`]. For user accounts or custom authorization,
  /// construct with [`Self::new`] and drive [`Self::recv_auth`] directly.
  ///
  /// If authorization fails, this method attempts graceful shutdown and preserves
  /// the original failure.
  ///
  /// # Errors
  ///
  /// Returns setup/request errors or [`Error::Auth`] for an unsupported state in
  /// the bot-token flow.
  pub async fn bot(params: fns::setTdlibParameters, token: &str) -> Result<Self> {
    let mut client = Self::new(params).await?;
    if let Err(err) = client.authorize_bot(token).await {
      let _ = client.shutdown().await;
      return Err(err);
    }
    Ok(client)
  }

  /// Returns a non-owning request capability for this client.
  ///
  /// Cloning the returned [`Sender`] does not keep `self` or its native lifecycle
  /// alive. See [`Sender`] for shutdown-race behavior.
  pub fn sender(&self) -> Sender {
    Sender(Arc::downgrade(&self.state))
  }

  /// Receives the next ordinary update in `TDLib` order.
  ///
  /// Authorization-state updates are consumed internally. Ordinary updates seen
  /// by [`Self::recv_auth`] are returned here first in their original order. Once
  /// `authorizationStateClosed` has been observed and buffered updates are empty,
  /// this method returns `Ok(None)` on every call.
  ///
  /// Use `recv_auth` while driving authorization: auth transitions consumed here
  /// are not replayed later.
  ///
  /// # Errors
  ///
  /// Returns an update decoding or uncorrelated `TDLib` error, or
  /// [`Error::Disconnected`] if the event channel closes before `TDLib`'s terminal
  /// authorization state is observed.
  pub async fn recv(&mut self) -> Result<Option<Update>> {
    loop {
      let update = match self.buffered_updates.pop_front() {
        Some(update) => update,
        None if self.closed => return Ok(None),
        None => self.recv_event().await?,
      };

      let Update::updateAuthorizationState(_) = &update else {
        return Ok(Some(update));
      };
    }
  }

  /// Receives the next authorization state without losing ordinary updates.
  ///
  /// Every intervening non-authorization update is buffered for [`Self::recv`].
  /// Once the closed state has been observed, this method returns
  /// `authorizationStateClosed` immediately without consuming that buffer.
  ///
  /// # Errors
  ///
  /// Returns an update decoding or uncorrelated `TDLib` error, or
  /// [`Error::Disconnected`] if the event channel closes first.
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
  /// Unless the terminal state was already observed, shutdown closes the request
  /// gate, sends the generated `close` function through normal response
  /// correlation, and waits for `authorizationStateClosed`. It then revokes all
  /// senders, unregisters this client, and waits for the process-wide receiver to
  /// become idle or continue on behalf of another client.
  ///
  /// Event errors observed while waiting are preserved, but shutdown continues to
  /// seek the terminal state. If closing itself fails, local state is still
  /// disconnected and unregistered; an error return does not claim native
  /// shutdown completed.
  ///
  /// # Errors
  ///
  /// Returns the close response error, the first event error observed before the
  /// terminal state, or [`Error::Disconnected`] if the close handshake cannot be
  /// observed.
  pub async fn shutdown(mut self) -> Result {
    let res = self.close_and_wait().await;
    self.state.disconnect();
    ROUTER.unregister_and_wait_for_receiver(self.state.id).await;
    res
  }

  /// Creates and registers the Rust state for a fresh, unconfigured native client.
  fn create_unconfigured() -> Self {
    // SAFETY: The call takes no arguments and returns an opaque ID by value.
    let id = unsafe { td_sys::td_create_client_id() };
    let (tx, rx) = mpsc::unbounded_channel();
    let (replies, next_extra, message_sends, buffered_updates, closed) = Default::default();
    let requests = Mutex::new(PendingRequests { replies, accepting: true });
    let message_sends = Mutex::new(message_sends);
    let state = Arc::new(ClientState { id, next_extra, requests, message_sends, events: tx });
    ROUTER.register(id, Arc::downgrade(&state));
    Self { state, buffered_updates, closed, events: rx }
  }

  /// Drives the deliberately narrow bot-token authorization state machine.
  async fn authorize_bot(&mut self, token: &str) -> Result {
    loop {
      match self.recv_auth().await? {
        AuthorizationState::authorizationStateWaitTdlibParameters => {}
        AuthorizationState::authorizationStateWaitPhoneNumber => {
          self.state.execute_request(&fns::checkAuthenticationBotToken { token: token.into() }, false).await?;
        }
        AuthorizationState::authorizationStateReady => return Ok(()),
        state => return Err(Error::Auth(state)),
      }
    }
  }

  /// Sends `close` if necessary and drains events through the terminal auth state.
  async fn close_and_wait(&mut self) -> Result {
    if self.closed {
      return Ok(());
    }

    self.state.execute_request(&fns::close {}, true).await?;
    let mut res = Ok(());
    while !self.closed {
      match self.recv_event().await {
        Ok(_) => {}
        Err(Error::Disconnected) => return res.and(Err(Error::Disconnected)),
        // An event error does not prove that TDLib stopped. Preserve the first
        // failure while continuing to seek authorizationStateClosed.
        Err(err) => res = res.and(Err(err)),
      }
    }
    res
  }

  /// Receives one raw client event and applies lifecycle side effects.
  async fn recv_event(&mut self) -> Result<Update> {
    let update = self.events.recv().await.ok_or(Error::Disconnected)??;
    if let Update::updateAuthorizationState(update) = &update
      && let AuthorizationState::authorizationStateClosed = update.authorization_state
    {
      self.closed = true;
      self.state.disconnect();
    }
    Ok(update)
  }
}

/// Serializes request correlation beside the fields of a generated `TDLib` function.
#[derive(Serialize)]
struct OutgoingRequest<'a, F> {
  /// Per-client response correlation key serialized as `TDLib`'s `@extra` field.
  #[serde(rename = "@extra")]
  extra: u64,
  /// Generated function fields flattened beside `@extra`.
  #[serde(flatten)]
  request: &'a F,
}

/// The request/close ordering gate and all responses awaiting direct correlation.
///
/// The mutex containing this value is held through `td_send`. Consequently,
/// ordinary requests are either installed and sent before `close`, or observe
/// `accepting == false` and never reach `TDLib`.
struct PendingRequests {
  /// Whether new ordinary requests may be sent; `close` changes this to false.
  accepting: bool,
  /// Direct response waiters keyed by their per-client `@extra` value.
  replies: HashMap<u64, PendingReply>,
}

/// A direct raw-JSON response channel for an ordinary generated function.
type RequestReply = oneshot::Sender<Result<Vec<u8>>>;
/// The direct temporary-message response channel for a tracked `sendMessage`.
type MessageReply = oneshot::Sender<Result<types::message>>;

/// The continuation selected when a correlated direct response arrives.
enum PendingReply {
  /// Return the response bytes for typed deserialization by the requesting task.
  Request(RequestReply),
  /// Parse and bind a temporary message before waking the send-tracking task.
  Message(MessageReply),
}

/// Shared state used by request futures and the process-wide router.
///
/// The owning [`Client`] normally holds the only durable strong reference.
/// Individual requests temporarily acquire one by upgrading a [`Sender`], while
/// the router retains only a [`Weak`] entry.
struct ClientState {
  /// Opaque native client ID used for process-wide routing.
  id: i32,
  /// Monotonic source of per-client `@extra` correlation keys.
  next_extra: AtomicU64,
  /// Short synchronous request gate; never held across an `.await`.
  requests: Mutex<PendingRequests>,
  /// Correlation from temporary messages to authoritative send outcomes.
  message_sends: Mutex<message_send::Registry>,
  /// Ordered application-event sink and owner-liveness signal.
  events: mpsc::UnboundedSender<Result<Update>>,
}

impl ClientState {
  /// Executes a generated function and deserializes its declared return type.
  async fn execute_request<F: Function>(&self, request: &F, closing: bool) -> Result<F::Return> {
    let response = self.execute_raw_request(request, closing).await?;
    serde_json::from_slice(&response).map_err(Into::into)
  }

  /// Executes a generated function while leaving its successful response as JSON bytes.
  async fn execute_raw_request<F: Function>(&self, request: &F, closing: bool) -> Result<Vec<u8>> {
    let (extra, serialized) = self.serialize_correlated_request(request)?;
    let (reply, response) = oneshot::channel();
    self.register_and_send_request(extra, &serialized, closing, PendingReply::Request(reply))?;
    response.await.map_err(|_| Error::Disconnected)?
  }

  /// Allocates an `@extra` key and serializes a NUL-terminated native request.
  fn serialize_correlated_request<F: Function>(&self, request: &F) -> Result<(u64, Vec<u8>)> {
    let extra = self.next_extra.fetch_add(1, Ordering::Relaxed);
    let mut serialized = serde_json::to_vec(&OutgoingRequest { extra, request })?;
    serialized.push(0);
    Ok((extra, serialized))
  }

  /// Atomically orders, registers, and sends one request against shutdown.
  fn register_and_send_request(&self, extra: u64, request: &[u8], closing: bool, reply: PendingReply) -> Result {
    let mut requests = self.requests.lock().unwrap();
    if !requests.accepting || (!closing && self.events.is_closed()) {
      return Err(Error::Disconnected);
    }
    if closing {
      requests.accepting = false;
    }

    requests.replies.insert(extra, reply);
    // Keep the gate locked through td_send: releasing it here would allow close
    // to overtake a request that had already been accepted.
    // SAFETY: `self.id` came from TDLib. `request` is live and NUL-terminated.
    unsafe { td_sys::td_send(self.id, request.as_ptr().cast()) };
    Ok(())
  }

  /// Completes and removes the direct waiter for a routed `@extra` response.
  fn complete_request(&self, extra: u64, r#type: &str, raw: &[u8]) {
    let Some(reply) = self.requests.lock().unwrap().replies.remove(&extra) else { return };
    match reply {
      PendingReply::Request(reply) => {
        let response = match r#type {
          "error" => Err(parse_td_error(raw)),
          _ => Ok(raw.to_vec()),
        };
        let _ = reply.send(response);
      }
      PendingReply::Message(reply) => self.complete_message_request(extra, r#type, raw, reply),
    }
  }

  /// Parses an uncorrelated `TDLib` object and enqueues it for the owning client.
  fn send_event(&self, r#type: &str, raw: &[u8]) {
    let event = match r#type {
      "error" => Err(parse_td_error(raw)),
      _ => self.parse_update(raw),
    };
    let _ = self.events.send(event);
  }

  /// Deserializes an update and applies internal observers before returning it.
  fn parse_update(&self, raw: &[u8]) -> Result<Update> {
    let update = match serde_json::from_slice(raw) {
      Ok(update) => update,
      Err(error) => {
        let error = Arc::new(error);
        self.message_sends.lock().unwrap().fail_json(&error);
        return Err(Error::Json(error));
      }
    };
    // Internal progress cannot depend on the application polling Client::recv.
    // send_event subsequently enqueues this same update unchanged.
    self.observe_update(&update);
    Ok(update)
  }

  /// Completes internal message/lifecycle work implied by one application update.
  fn observe_update(&self, update: &Update) {
    self.message_sends.lock().unwrap().observe(update);
    if let Update::updateAuthorizationState(update) = update
      && let AuthorizationState::authorizationStateClosed = update.authorization_state
    {
      self.disconnect();
    }
  }

  /// Revokes new work and drops every local request or message-send waiter.
  fn disconnect(&self) {
    let mut requests = self.requests.lock().unwrap();
    requests.accepting = false;
    requests.replies.clear();
    drop(requests);
    self.message_sends.lock().unwrap().disconnect();
  }

  /// Fails every waiter and the event stream after an unrouteable JSON envelope.
  fn report_json_error(&self, error: &Arc<serde_json::Error>) {
    let replies = {
      let mut requests = self.requests.lock().unwrap();
      mem::take(&mut requests.replies)
    };
    for (_, reply) in replies {
      match reply {
        PendingReply::Request(reply) => {
          let _ = reply.send(Err(Error::Json(Arc::clone(error))));
        }
        PendingReply::Message(reply) => {
          let _ = reply.send(Err(Error::Json(Arc::clone(error))));
        }
      }
    }
    self.message_sends.lock().unwrap().fail_json(error);
    let _ = self.events.send(Err(Error::Json(Arc::clone(error))));
  }
}

impl Drop for ClientState {
  fn drop(&mut self) {
    // Destruction stays lock-free and native-call-free. The receiver prunes the
    // dead weak entry at its next registry access.
    ROUTER.stale.store(true, Ordering::Release);
  }
}

/// Borrowed routing fields parsed before a full response or update is decoded.
///
/// `TDLib` guarantees `@client_id` and `@type`, so both remain required. `@extra`
/// is absent only for uncorrelated updates and events.
#[derive(Deserialize)]
struct IncomingEnvelope<'a> {
  /// Native client to which the object belongs.
  #[serde(rename = "@client_id")]
  client_id: i32,
  /// Optional request key distinguishing a response from an event.
  #[serde(rename = "@extra")]
  extra: Option<u64>,
  /// `TDLib` object discriminator, borrowed from the receive buffer.
  #[serde(rename = "@type")]
  r#type: &'a str,
}

/// Process-wide registry and sole native receiver-thread coordination state.
struct Router {
  /// Weak client routes keyed by `TDLib`'s native client ID.
  clients: Mutex<HashMap<i32, Weak<ClientState>>>,
  /// Handle used to unpark the process-lifetime receiver thread after registration.
  worker: OnceLock<thread::Thread>,
  /// Versioned signal for registration and receiver idle transitions.
  clients_changed: watch::Sender<()>,
  /// Cheap indication that at least one weak entry may now be dead.
  stale: AtomicBool,
  /// Process-wide receive timeout stored as the bits of an `f64` number of seconds.
  timeout: AtomicU64,
}

impl Router {
  /// Locks the client registry, pruning dead weak entries only when requested.
  fn live_clients(&self) -> MutexGuard<'_, HashMap<i32, Weak<ClientState>>> {
    let mut clients = self.clients.lock().unwrap();
    if self.stale.swap(false, Ordering::Acquire) {
      clients.retain(|_, state| state.strong_count() > 0);
    }
    clients
  }

  /// Registers a weak route, starts or unparks the worker, and publishes the change.
  fn register(&self, id: i32, state: Weak<ClientState>) {
    self.live_clients().insert(id, state);
    self.worker.get_or_init(|| thread::spawn(receive_loop).thread().clone()).unpark();
    self.clients_changed.send_replace(());
  }

  /// Removes a route and, if it was last, awaits a safe receiver transition.
  ///
  /// The transition is either the receiver observing the empty registry before it
  /// parks, or another registration taking ownership of its continued native work.
  async fn unregister_and_wait_for_receiver(&self, id: i32) {
    let mut changed = self.clients_changed.subscribe();
    {
      let mut clients = self.live_clients();
      clients.remove(&id);
      let 0 = clients.len() else { return };
      // Subscribe before removal and consume the current version under the same
      // registry lock. The next version must describe a later ownership boundary.
      changed.borrow_and_update();
    }
    let _ = changed.changed().await;
  }

  /// Upgrades the weak state for one native client ID.
  fn find_client(&self, id: i32) -> Option<Arc<ClientState>> {
    Weak::upgrade(self.live_clients().get(&id)?)
  }

  /// Reports whether the registry has no live clients after stale pruning.
  fn is_empty(&self) -> bool {
    self.live_clients().is_empty()
  }

  /// Routes one borrowed native buffer by `@client_id`, then optional `@extra`.
  fn route_message(&self, raw: &[u8]) {
    let envelope = match serde_json::from_slice::<IncomingEnvelope<'_>>(raw) {
      Ok(envelope) => envelope,
      Err(err) => {
        self.broadcast_json_error(err);
        return;
      }
    };

    let IncomingEnvelope { client_id, extra, r#type } = envelope;
    let Some(client) = self.find_client(client_id) else { return };

    match extra {
      Some(extra) => client.complete_request(extra, r#type, raw),
      None => client.send_event(r#type, raw),
    }
  }

  /// Reports an envelope error to every client because no route can be trusted.
  fn broadcast_json_error(&self, error: serde_json::Error) {
    let error = Arc::new(error);
    for state in self.live_clients().values().filter_map(Weak::upgrade) {
      state.report_json_error(&error);
    }
  }

  /// Updates the timeout used by the receiver's next native call.
  fn set_receive_timeout(&self, timeout: Duration) {
    self.timeout.store(timeout.as_secs_f64().to_bits(), Ordering::Relaxed);
  }

  /// Performs at most one native receive and routes its borrowed response buffer.
  fn receive_one(&self) {
    let timeout = f64::from_bits(self.timeout.load(Ordering::Relaxed));

    // SAFETY: Only the process-wide receiver thread calls `td_receive`;
    // this crate never calls `td_execute`.
    let raw = unsafe { td_sys::td_receive(timeout) };
    if raw.is_null() {
      return;
    }

    // SAFETY: `raw` is non-null and points to TDLib's NUL-terminated buffer,
    // which remains valid until the next receive or execute call.
    self.route_message(unsafe { CStr::from_ptr(raw) }.to_bytes());
  }
}

/// The lazily initialized process-wide client router.
static ROUTER: LazyLock<Router> = LazyLock::new(|| {
  let (clients_changed, _) = watch::channel(());
  let (clients, worker, stale) = Default::default();
  let timeout = 1f64.to_bits().into();
  Router { clients, worker, clients_changed, stale, timeout }
});

/// Runs the sole process-lifetime native receiver, parking between live registries.
fn receive_loop() {
  loop {
    if ROUTER.is_empty() {
      ROUTER.clients_changed.send_replace(());
      thread::park();
    } else {
      ROUTER.receive_one();
    }
  }
}

/// Decodes a `TDLib` `error`, preserving a malformed error object as JSON failure.
fn parse_td_error(raw: &[u8]) -> Error {
  serde_json::from_slice(raw).map_or_else(Into::into, Error::Td)
}

/// Sets the maximum wait used by the process-wide receiver's next `TDLib` call.
///
/// The default is one second. Changing it does not interrupt a receive already in
/// progress; it takes effect when the receiver next calls `TDLib`. A shorter value
/// can reduce final-shutdown handoff latency at the cost of more native calls, and
/// zero can cause the receiver thread to poll continuously while clients are live.
///
/// This is transport tuning, not a request, retry, or operation timeout.
pub fn set_receive_timeout(timeout: Duration) {
  ROUTER.set_receive_timeout(timeout);
}

/// Sets `TDLib`'s process-wide internal log verbosity.
///
/// `TDLib` defaults to level 5. Levels 0 through 5 select fatal, error,
/// warning/debug-warning, informational, debug, and verbose-debug logging;
/// higher supported values enable progressively more detail.
pub fn set_log_level(level: i32) {
  // SAFETY: The call passes no pointers or borrowed storage.
  unsafe { td_sys::td_set_log_verbosity_level(level) };
}

/// Returns a small server-oriented starting point for `TDLib` parameters with the
/// given credentials and rooted at `dir`.
///
/// The value enables the file, chat-info, and message databases and stores databases
/// and files below `{dir}/db` and `{dir}/files` on top of the settings provided by
/// [`defaults`].
pub fn params(api_id: i32, api_hash: impl Into<String>, dir: impl AsRef<Path>) -> fns::setTdlibParameters {
  let dir = dir.as_ref();
  fns::setTdlibParameters {
    api_id,
    api_hash: api_hash.into(),
    database_directory: dir.join("db").display().to_string(),
    files_directory: dir.join("files").display().to_string(),
    use_file_database: true,
    use_chat_info_database: true,
    use_message_database: true,
    ..defaults()
  }
}

/// Returns a small server-oriented starting point for `TDLib` parameters.
///
/// The value uses English and the `Server` device model, and reports this crate's
/// version as the application version. All databases remain disabled, and required
/// credentials and storage directories retain their generated defaults.
pub fn defaults() -> fns::setTdlibParameters {
  fns::setTdlibParameters {
    system_language_code: "en".into(),
    device_model: "Server".into(),
    application_version: env!("CARGO_PKG_VERSION").into(),
    ..Default::default()
  }
}

#[cfg(test)]
mod tests;
