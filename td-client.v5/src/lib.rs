use std::collections::{HashMap, VecDeque};
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock, Weak};
use std::{error, fmt, result, thread, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use td_types::enums::{AuthorizationState, Update};
use td_types::traits::Function;
use td_types::{fns, types};

pub type Result<T = ()> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
  Td(types::error),
  Json(Arc<serde_json::Error>),
  Auth(AuthorizationState),
  Disconnected,
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Td(types::error { code, message }) => write!(f, "TDLib: {code} {message}"),
      Self::Json(error) => write!(f, "JSON: {error}"),
      Self::Auth(state) => write!(f, "unexpected auth state: {state:?}"),
      Self::Disconnected => f.write_str("client disconnected"),
    }
  }
}

impl error::Error for Error {
  fn source(&self) -> Option<&(dyn error::Error + 'static)> {
    let Self::Json(error) = self else { return None };
    Some(error.as_ref())
  }
}

impl From<serde_json::Error> for Error {
  fn from(error: serde_json::Error) -> Self {
    Self::Json(Arc::new(error))
  }
}

/// A non-owning request sender with no update or shutdown access.
pub struct Sender(Weak<ClientState>);

impl Sender {
  pub async fn send<F: Function>(&self, request: &F) -> Result<F::Return> {
    let state = self.0.upgrade().ok_or(Error::Disconnected)?;
    state.execute_request(request, false).await
  }
}

#[must_use = "call shutdown().await to finish TDLib cleanly"]
pub struct Client {
  state: Arc<ClientState>,
  events: mpsc::UnboundedReceiver<Result<Update>>,
  buffered_updates: VecDeque<Update>,
  closed: bool,
}

impl fmt::Debug for Client {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Client").finish_non_exhaustive()
  }
}

impl Client {
  pub async fn new(parameters: fns::setTdlibParameters) -> Result<Self> {
    let client = Self::create_unconfigured();
    if let Err(error) = client.send(&parameters).await {
      let _ = client.shutdown().await;
      return Err(error);
    }
    Ok(client)
  }

  pub async fn bot(parameters: fns::setTdlibParameters, token: &str) -> Result<Self> {
    let mut client = Self::new(parameters).await?;
    if let Err(error) = client.authorize_bot(token).await {
      let _ = client.shutdown().await;
      return Err(error);
    }
    Ok(client)
  }

  pub fn sender(&self) -> Sender {
    Sender(Arc::downgrade(&self.state))
  }

  pub async fn send<F: Function>(&self, request: &F) -> Result<F::Return> {
    self.state.execute_request(request, false).await
  }

  pub async fn recv(&mut self) -> Result<Option<Update>> {
    loop {
      let update = match self.buffered_updates.pop_front() {
        Some(update) => update,
        None if self.closed => return Ok(None),
        None => self.recv_event().await?,
      };

      let Update::updateAuthorizationState(_) = update else {
        return Ok(Some(update));
      };
    }
  }

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

  pub async fn shutdown(mut self) -> Result {
    let result = self.close_and_wait().await;
    self.state.cancel_requests();
    ROUTER.unregister_and_wait_for_receiver(self.state.id).await;
    result
  }

  fn create_unconfigured() -> Self {
    // SAFETY: The call takes no arguments and returns an opaque ID by value.
    let id = unsafe { td_sys::td_create_client_id() };
    let (tx, events) = mpsc::unbounded_channel();
    let (extra, buffered_updates, closed) = Default::default();
    let requests = Mutex::new(PendingRequests { accepting: true, replies: Default::default() });
    let state = Arc::new(ClientState { id, next_extra: extra, requests, events: tx });
    ROUTER.register(id, Arc::downgrade(&state));
    Self { state, events, buffered_updates, closed }
  }

  async fn authorize_bot(&mut self, token: &str) -> Result {
    loop {
      match self.recv_auth().await? {
        AuthorizationState::authorizationStateReady => return Ok(()),
        AuthorizationState::authorizationStateWaitTdlibParameters => {}
        AuthorizationState::authorizationStateWaitPhoneNumber => {
          self.send(&fns::checkAuthenticationBotToken { token: token.into() }).await?;
        }
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
    let update = self.events.recv().await.ok_or(Error::Disconnected)??;
    if let Update::updateAuthorizationState(types::updateAuthorizationState { authorization_state: AuthorizationState::authorizationStateClosed }) = &update {
      self.closed = true;
      self.state.cancel_requests();
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

struct PendingRequests {
  accepting: bool,
  replies: HashMap<u64, oneshot::Sender<Result<Vec<u8>>>>,
}

struct ClientState {
  id: i32,
  next_extra: AtomicU64,
  requests: Mutex<PendingRequests>,
  events: mpsc::UnboundedSender<Result<Update>>,
}

impl ClientState {
  async fn execute_request<F: Function>(&self, request: &F, closing: bool) -> Result<F::Return> {
    let (extra, serialized) = self.serialize_correlated_request(request)?;
    let response = self.register_and_send_request(extra, &serialized, closing)?.await.map_err(|_| Error::Disconnected)??;
    serde_json::from_slice(&response).map_err(Into::into)
  }

  fn serialize_correlated_request<F: Function>(&self, request: &F) -> Result<(u64, Vec<u8>)> {
    let extra = self.next_extra.fetch_add(1, Ordering::Relaxed);
    let mut serialized = serde_json::to_vec(&OutgoingRequest { extra, request })?;
    serialized.push(0);
    Ok((extra, serialized))
  }

  fn register_and_send_request(&self, extra: u64, request: &[u8], closing: bool) -> Result<oneshot::Receiver<Result<Vec<u8>>>> {
    let mut requests = self.requests.lock().unwrap();
    if !requests.accepting || (!closing && self.events.is_closed()) {
      return Err(Error::Disconnected);
    }
    if closing {
      requests.accepting = false;
    }

    let (tx, rx) = oneshot::channel();
    requests.replies.insert(extra, tx);
    // SAFETY: `self.id` came from TDLib. `request` is live and NUL-terminated.
    unsafe { td_sys::td_send(self.id, request.as_ptr().cast()) };
    Ok(rx)
  }

  fn complete_request(&self, extra: u64, r#type: &str, raw: &[u8]) {
    let Some(tx) = self.requests.lock().unwrap().replies.remove(&extra) else { return };
    let response = match r#type {
      "error" => Err(parse_td_error(raw)),
      _ => Ok(raw.to_vec()),
    };
    let _ = tx.send(response);
  }

  fn send_event(&self, r#type: &str, raw: &[u8]) {
    let event = match r#type {
      "error" => Err(parse_td_error(raw)),
      _ => serde_json::from_slice(raw).map_err(Into::into),
    };
    let _ = self.events.send(event);
  }

  fn cancel_requests(&self) {
    let mut requests = self.requests.lock().unwrap();
    requests.accepting = false;
    requests.replies.clear();
  }
}

impl Drop for ClientState {
  fn drop(&mut self) {
    ROUTER.stale.store(true, Ordering::Release);
  }
}

#[derive(Deserialize)]
struct IncomingMessage<'a> {
  #[serde(rename = "@client_id")]
  client_id: i32,
  #[serde(rename = "@extra")]
  extra: Option<u64>,
  #[serde(rename = "@type")]
  r#type: &'a str,
}

struct Router {
  clients: Mutex<HashMap<i32, Weak<ClientState>>>,
  worker: OnceLock<thread::Thread>,
  clients_changed: watch::Sender<()>,
  stale: AtomicBool,
  timeout: AtomicU64,
}

impl Router {
  fn live_clients(&self) -> MutexGuard<'_, HashMap<i32, Weak<ClientState>>> {
    let mut clients = self.clients.lock().unwrap();
    if self.stale.swap(false, Ordering::Acquire) {
      clients.retain(|_, state| state.strong_count() > 0);
    }
    clients
  }

  fn register(&self, id: i32, state: Weak<ClientState>) {
    self.live_clients().insert(id, state);
    self.worker.get_or_init(|| thread::spawn(receive_loop).thread().clone()).unpark();
    self.clients_changed.send_replace(());
  }

  async fn unregister_and_wait_for_receiver(&self, id: i32) {
    let mut changed = self.clients_changed.subscribe();
    {
      let mut clients = self.live_clients();
      clients.remove(&id);
      let 0 = clients.len() else { return };
      changed.borrow_and_update();
    }
    let _ = changed.changed().await;
  }

  fn find_client(&self, id: i32) -> Option<Arc<ClientState>> {
    Weak::upgrade(self.live_clients().get(&id)?)
  }

  fn is_empty(&self) -> bool {
    self.live_clients().is_empty()
  }

  fn route_message(&self, raw: &[u8]) {
    let incoming = match serde_json::from_slice::<IncomingMessage<'_>>(raw) {
      Ok(incoming) => incoming,
      Err(error) => {
        self.broadcast_json_error(error);
        return;
      }
    };
    let Some(client) = self.find_client(incoming.client_id) else { return };

    match incoming.extra {
      Some(extra) => client.complete_request(extra, incoming.r#type, raw),
      None => client.send_event(incoming.r#type, raw),
    }
  }

  fn broadcast_json_error(&self, error: serde_json::Error) {
    let error = Arc::new(error);
    for state in self.live_clients().values().filter_map(Weak::upgrade) {
      let _ = state.events.send(Err(Error::Json(Arc::clone(&error))));
    }
  }

  fn set_receive_timeout(&self, timeout: Duration) {
    self.timeout.store(timeout.as_secs_f64().to_bits(), Ordering::Relaxed);
  }

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

static ROUTER: LazyLock<Router> = LazyLock::new(|| {
  let (clients_changed, _) = watch::channel(());
  let (clients, worker, stale) = Default::default();
  let timeout = 1f64.to_bits().into();
  Router { clients, worker, clients_changed, stale, timeout }
});

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

fn parse_td_error(raw: &[u8]) -> Error {
  serde_json::from_slice(raw).map_or_else(Into::into, Error::Td)
}

pub fn set_receive_timeout(timeout: Duration) {
  ROUTER.set_receive_timeout(timeout);
}

pub fn set_log_level(level: i32) {
  // SAFETY: The call passes no pointers or borrowed storage.
  unsafe { td_sys::td_set_log_verbosity_level(level) };
}

pub fn parameters() -> fns::setTdlibParameters {
  fns::setTdlibParameters {
    database_directory: ".td/db".into(),
    files_directory: ".td/files".into(),
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
