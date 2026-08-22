use std::collections::{HashMap, VecDeque};
use std::ffi::CStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock, Weak};
use std::{error, fmt, result, thread};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use td_types::enums::{AuthorizationState, Update};
use td_types::traits::Function;
use td_types::{fns, types};

pub type Result<T = ()> = result::Result<T, Error>;

#[derive(Debug, Clone)]
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

/// A non-owning request capability for detached tasks.
///
/// It neither keeps its [`Client`] alive nor grants update or shutdown access.
pub struct Requests {
  state: Weak<State>,
}

impl Requests {
  pub async fn execute<F: Function>(&self, request: &F) -> Result<F::Return> {
    let state = self.state.upgrade().ok_or(Error::Disconnected)?;
    state.execute(request, false).await
  }
}

#[must_use = "call shutdown().await to finish TDLib cleanly"]
pub struct Client {
  state: Arc<State>,
  events: mpsc::UnboundedReceiver<Result<Update>>,
  queued: VecDeque<Update>,
  closed: bool,
}

impl fmt::Debug for Client {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Client").finish_non_exhaustive()
  }
}

impl Client {
  pub async fn new(params: fns::setTdlibParameters) -> Result<Self> {
    let client = Self::create();
    if let Err(error) = client.execute(&params).await {
      let _ = client.shutdown().await;
      return Err(error);
    }
    Ok(client)
  }

  pub async fn bot(params: fns::setTdlibParameters, token: &str) -> Result<Self> {
    let mut client = Self::new(params).await?;
    if let Err(error) = client.authorize_bot(token).await {
      let _ = client.shutdown().await;
      return Err(error);
    }
    Ok(client)
  }

  /// Creates a non-owning request capability for application tasks.
  pub fn requests(&self) -> Requests {
    Requests { state: Arc::downgrade(&self.state) }
  }

  pub async fn execute<F: Function>(&self, request: &F) -> Result<F::Return> {
    self.state.execute(request, false).await
  }

  pub async fn recv(&mut self) -> Result<Option<Update>> {
    loop {
      let update = match self.queued.pop_front() {
        Some(update) => update,
        None if self.closed => return Ok(None),
        None => self.event().await?,
      };

      let Update::updateAuthorizationState(_) = update else {
        return Ok(Some(update));
      };
    }
  }

  pub async fn auth(&mut self) -> Result<AuthorizationState> {
    if self.closed {
      return Ok(AuthorizationState::authorizationStateClosed);
    }

    loop {
      match self.event().await? {
        Update::updateAuthorizationState(update) => return Ok(update.authorization_state),
        update => self.queued.push_back(update),
      }
    }
  }

  pub async fn shutdown(mut self) -> Result {
    let result = async {
      if !self.closed {
        self.state.execute(&fns::close {}, true).await?;
        let mut failure = None;
        while !self.closed {
          match self.event().await {
            Ok(_) => {}
            Err(Error::Disconnected) => return Err(failure.unwrap_or(Error::Disconnected)),
            Err(error) => {
              failure.get_or_insert(error);
            }
          }
        }
        return failure.map_or(Ok(()), Err);
      }
      Ok(())
    }
    .await;

    self.state.disconnect();
    ROUTER.remove_and_wait(self.state.id).await;
    result
  }

  fn create() -> Self {
    // SAFETY: TDLib creates and returns a new client identifier.
    let id = unsafe { td_sys::td_create_client_id() };
    let (tx, events) = mpsc::unbounded_channel();
    let (extra, queued, closed) = Default::default();
    let requests = Mutex::new(RequestState { open: true, pending: Default::default() });
    let state = Arc::new(State { id, extra, requests, events: tx });
    ROUTER.insert(id, Arc::downgrade(&state));
    Self { state, events, queued, closed }
  }

  async fn authorize_bot(&mut self, token: &str) -> Result {
    loop {
      match self.auth().await? {
        AuthorizationState::authorizationStateReady => return Ok(()),
        AuthorizationState::authorizationStateWaitTdlibParameters => {}
        AuthorizationState::authorizationStateWaitPhoneNumber => {
          self.execute(&fns::checkAuthenticationBotToken { token: token.into() }).await?;
        }
        state => return Err(Error::Auth(state)),
      }
    }
  }

  async fn event(&mut self) -> Result<Update> {
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

impl Drop for Client {
  fn drop(&mut self) {
    self.state.disconnect();
    ROUTER.remove(self.state.id);
  }
}

#[derive(Serialize)]
struct Request<'a, F> {
  #[serde(rename = "@extra")]
  extra: u64,
  #[serde(flatten)]
  request: &'a F,
}

struct RequestState {
  open: bool,
  pending: HashMap<u64, oneshot::Sender<Result<Vec<u8>>>>,
}

struct State {
  id: i32,
  extra: AtomicU64,
  requests: Mutex<RequestState>,
  events: mpsc::UnboundedSender<Result<Update>>,
}

impl State {
  async fn execute<F: Function>(&self, request: &F, close: bool) -> Result<F::Return> {
    let extra = self.extra.fetch_add(1, Ordering::Relaxed);
    let mut request = serde_json::to_vec(&Request { extra, request })?;
    request.push(0);
    let response = self.send(extra, &request, close)?.await.map_err(|_| Error::Disconnected)??;
    serde_json::from_slice(&response).map_err(Into::into)
  }

  fn send(&self, extra: u64, request: &[u8], close: bool) -> Result<oneshot::Receiver<Result<Vec<u8>>>> {
    let mut requests = self.requests.lock().unwrap();
    if !requests.open {
      return Err(Error::Disconnected);
    }
    if close {
      requests.open = false;
    }

    let (tx, rx) = oneshot::channel();
    requests.pending.insert(extra, tx);
    // SAFETY: the client is live and `request` is NUL-terminated.
    unsafe { td_sys::td_send(self.id, request.as_ptr().cast()) };
    Ok(rx)
  }

  fn respond(&self, extra: u64, r#type: &str, raw: &[u8]) {
    let Some(tx) = self.requests.lock().unwrap().pending.remove(&extra) else { return };
    let response = match r#type {
      "error" => Err(td_error(raw)),
      _ => Ok(raw.to_vec()),
    };
    let _ = tx.send(response);
  }

  fn emit(&self, r#type: &str, raw: &[u8]) {
    let event = match r#type {
      "error" => Err(td_error(raw)),
      _ => serde_json::from_slice(raw).map_err(Into::into),
    };
    let _ = self.events.send(event);
  }

  fn disconnect(&self) {
    let mut requests = self.requests.lock().unwrap();
    requests.open = false;
    for (_, pending) in requests.pending.drain() {
      let _ = pending.send(Err(Error::Disconnected));
    }
  }
}

#[derive(Deserialize)]
struct Envelope<'a> {
  #[serde(rename = "@client_id")]
  client: i32,
  #[serde(rename = "@extra")]
  extra: Option<u64>,
  #[serde(rename = "@type")]
  r#type: &'a str,
}

struct Router {
  clients: Mutex<HashMap<i32, Weak<State>>>,
  worker: OnceLock<thread::Thread>,
  changed: watch::Sender<()>,
}

impl Router {
  fn state(&self, id: i32) -> Option<Arc<State>> {
    Weak::upgrade(self.clients.lock().unwrap().get(&id)?)
  }

  fn insert(&self, id: i32, state: Weak<State>) {
    self.clients.lock().unwrap().insert(id, state);
    self.worker.get_or_init(|| thread::spawn(worker).thread().clone()).unpark();
    self.changed.send_replace(());
  }

  fn remove(&self, id: i32) {
    self.clients.lock().unwrap().remove(&id);
  }

  async fn remove_and_wait(&self, id: i32) {
    let mut changed = self.changed.subscribe();
    {
      let mut clients = self.clients.lock().unwrap();
      clients.remove(&id);
      let 0 = clients.len() else { return };
      changed.borrow_and_update();
    }
    let _ = changed.changed().await;
  }

  fn idle(&self) -> bool {
    self.clients.lock().unwrap().is_empty()
  }

  fn dispatch(&self, raw: &[u8]) {
    let envelope = match serde_json::from_slice::<Envelope<'_>>(raw) {
      Ok(envelope) => envelope,
      Err(error) => {
        self.fail(&Error::from(error));
        return;
      }
    };
    let Some(state) = self.state(envelope.client) else { return };

    match envelope.extra {
      Some(extra) => state.respond(extra, envelope.r#type, raw),
      None => state.emit(envelope.r#type, raw),
    }
  }

  fn fail(&self, error: &Error) {
    for state in self.clients.lock().unwrap().values().filter_map(Weak::upgrade) {
      let _ = state.events.send(Err(error.clone()));
    }
  }

  fn receive(&self) {
    // SAFETY: this method is called only by the sole receiver thread.
    let raw = unsafe { td_sys::td_receive(1.0) };
    if raw.is_null() {
      return;
    }

    // SAFETY: TDLib returned a non-null NUL-terminated string.
    self.dispatch(unsafe { CStr::from_ptr(raw) }.to_bytes());
  }
}

static ROUTER: LazyLock<Router> = LazyLock::new(|| {
  let (changed, _) = watch::channel(());
  let (clients, worker) = Default::default();
  Router { clients, worker, changed }
});

fn worker() {
  loop {
    if ROUTER.idle() {
      ROUTER.changed.send_replace(());
      thread::park();
    } else {
      ROUTER.receive();
    }
  }
}

fn td_error(raw: &[u8]) -> Error {
  serde_json::from_slice(raw).map_or_else(Into::into, Error::Td)
}

pub fn set_log_verbosity_level(level: i32) {
  // SAFETY: TDLib accepts every integer verbosity level.
  unsafe { td_sys::td_set_log_verbosity_level(level) };
}

pub fn defaults() -> fns::setTdlibParameters {
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
