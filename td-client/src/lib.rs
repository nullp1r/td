use std::collections::{HashMap, VecDeque};
use std::ffi::CStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock, Weak};
use std::{error, fmt, result, thread};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use td_types::enums::{AuthorizationState, Update};
use td_types::traits::Function;
use td_types::{fns, types};

pub type Result<T = ()> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
  Td(types::error),
  Json(serde_json::Error),
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
    Some(error)
  }
}

impl From<serde_json::Error> for Error {
  fn from(error: serde_json::Error) -> Self {
    Self::Json(error)
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
    f.debug_struct("Client").field("id", &self.state.id).finish_non_exhaustive()
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

  pub async fn execute<F: Function>(&self, request: &F) -> Result<F::Return> {
    let extra = self.state.extra.fetch_add(1, Ordering::Relaxed);
    let mut request = serde_json::to_vec(&Request { extra, request })?;
    request.push(0);
    let response = self.state.send(extra, &request).await.map_err(|_| Error::Disconnected)??;
    serde_json::from_slice(&response).map_err(Into::into)
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
    loop {
      match self.event().await? {
        Update::updateAuthorizationState(update) => return Ok(update.authorization_state),
        update => self.queued.push_back(update),
      }
    }
  }

  pub async fn shutdown(mut self) -> Result {
    if !self.closed {
      self.execute(&fns::close {}).await?;
      while !self.closed {
        self.event().await?;
      }
    }
    ROUTER.remove_and_wait(self.state.id).await;
    Ok(())
  }

  fn create() -> Self {
    // SAFETY: TDLib creates and returns a new client identifier.
    let id = unsafe { td_sys::td_create_client_id() };
    let (tx, events) = mpsc::unbounded_channel();
    let (extra, pending, queued, closed) = Default::default();
    let state = Arc::new(State { id, extra, pending, events: tx });
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
    }
    Ok(update)
  }
}

#[derive(Serialize)]
struct Request<'a, F> {
  #[serde(rename = "@extra")]
  extra: u64,
  #[serde(flatten)]
  request: &'a F,
}

struct State {
  id: i32,
  extra: AtomicU64,
  pending: Mutex<HashMap<u64, oneshot::Sender<Result<Vec<u8>>>>>,
  events: mpsc::UnboundedSender<Result<Update>>,
}

impl State {
  fn send(&self, extra: u64, request: &[u8]) -> oneshot::Receiver<Result<Vec<u8>>> {
    let (tx, rx) = oneshot::channel();
    self.pending.lock().unwrap().insert(extra, tx);

    // SAFETY: the client is live and `request` is NUL-terminated.
    unsafe { td_sys::td_send(self.id, request.as_ptr().cast()) };
    rx
  }

  fn respond(&self, extra: u64, r#type: &str, raw: &[u8]) {
    let Some(tx) = self.pending.lock().unwrap().remove(&extra) else { return };
    let response = match r#type {
      "error" => Err(td_error(raw)),
      _ => Ok(raw.to_vec()),
    };
    let _ = tx.send(response);
  }

  fn emit(&self, r#type: &str, raw: &[u8]) {
    let event = match r#type {
      "error" => Err(td_error(raw)),
      _ => serde_json::from_slice(raw).map_err(Error::Json),
    };
    let _ = self.events.send(event);
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
  timeout: AtomicU64,
}

impl Router {
  fn live_clients(&self) -> MutexGuard<'_, HashMap<i32, Weak<State>>> {
    let mut clients = self.clients.lock().unwrap();
    clients.retain(|_, state| state.strong_count() > 0);
    clients
  }

  fn state(&self, id: i32) -> Option<Arc<State>> {
    Weak::upgrade(self.live_clients().get(&id)?)
  }

  fn insert(&self, id: i32, state: Weak<State>) {
    self.live_clients().insert(id, state);
    self.worker.get_or_init(|| thread::spawn(worker).thread().clone()).unpark();
    self.changed.send_replace(());
  }

  fn idle(&self) -> bool {
    self.live_clients().is_empty()
  }

  async fn remove_and_wait(&self, id: i32) {
    let mut changed = self.changed.subscribe();
    {
      let mut clients = self.live_clients();
      clients.remove(&id);
      let 0 = clients.len() else { return };
      changed.borrow_and_update();
    }
    let _ = changed.changed().await;
  }

  fn dispatch(&self, raw: &[u8]) {
    let Ok(envelope) = serde_json::from_slice::<Envelope<'_>>(raw) else { return };
    let Some(state) = self.state(envelope.client) else { return };

    match envelope.extra {
      Some(extra) => state.respond(extra, envelope.r#type, raw),
      None => state.emit(envelope.r#type, raw),
    }
  }

  fn set_receive_timeout(&self, seconds: f64) {
    self.timeout.store(seconds.to_bits(), Ordering::Relaxed);
  }

  fn receive(&self) {
    let timeout = f64::from_bits(self.timeout.load(Ordering::Relaxed));

    // SAFETY: this method is called only by the sole receiver thread.
    let raw = unsafe { td_sys::td_receive(timeout) };
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
  let timeout = 1f64.to_bits().into();
  Router { clients, worker, changed, timeout }
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
  serde_json::from_slice(raw).map_or_else(Error::Json, Error::Td)
}

pub fn set_receive_timeout(seconds: f64) {
  ROUTER.set_receive_timeout(seconds);
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
