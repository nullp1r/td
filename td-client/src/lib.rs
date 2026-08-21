use std::collections::{HashMap, VecDeque};
use std::error::Error as StdError;
use std::ffi::CStr;
use std::fmt;
use std::result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock, Weak};
use std::thread;

use serde::{Deserialize, Serialize};
use td_types::enums::{AuthorizationState, Update};
use td_types::traits::Function;
use td_types::{fns, types};
use tokio::sync::{mpsc, oneshot, watch};

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

impl StdError for Error {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
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

impl Client {
  pub async fn new(params: fns::setTdlibParameters) -> Result<Self> {
    // SAFETY: TDLib creates and returns a new client identifier.
    let id = unsafe { td_sys::td_create_client_id() };
    let (tx, events) = mpsc::unbounded_channel();
    let state = Arc::new(State { id, extra: 0.into(), pending: Default::default(), events: tx });
    ROUTER.insert(id, Arc::downgrade(&state));
    let client = Self { state, events, queued: VecDeque::new(), closed: false };

    if let Err(error) = client.execute(&params).await {
      let _ = client.shutdown().await;
      return Err(error);
    }
    Ok(client)
  }

  pub async fn bot(params: fns::setTdlibParameters, token: &str) -> Result<Self> {
    let mut client = Self::new(params).await?;
    let fut = async {
      loop {
        match client.auth().await? {
          AuthorizationState::authorizationStateWaitTdlibParameters => {}
          AuthorizationState::authorizationStateWaitPhoneNumber => {
            client.execute(&fns::checkAuthenticationBotToken { token: token.into() }).await?;
          }
          AuthorizationState::authorizationStateReady => return Ok(()),
          state => return Err(Error::Auth(state)),
        }
      }
    };

    if let Err(error) = fut.await {
      let _ = client.shutdown().await;
      return Err(error);
    }
    Ok(client)
  }

  pub async fn execute<F: Function>(&self, request: &F) -> Result<F::Return> {
    let extra = self.state.extra.fetch_add(1, Ordering::Relaxed);
    let mut request = serde_json::to_vec(&Request { extra, request })?;
    request.push(0);

    let (tx, rx) = oneshot::channel();
    self.state.pending.lock().unwrap().insert(extra, tx);

    // SAFETY: the client is live and `request` is NUL-terminated.
    unsafe { td_sys::td_send(self.state.id, request.as_ptr().cast()) };
    let response = rx.await.map_err(|_| Error::Disconnected)??;
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

#[derive(Deserialize)]
struct Envelope<'a> {
  #[serde(rename = "@client_id")]
  client: i32,
  #[serde(rename = "@extra")]
  extra: Option<u64>,
  #[serde(rename = "@type")]
  kind: &'a str,
}

struct Router {
  clients: Mutex<HashMap<i32, Weak<State>>>,
  worker: OnceLock<thread::Thread>,
  changed: watch::Sender<()>,
}

impl Router {
  fn insert(&self, id: i32, state: Weak<State>) {
    let mut clients = self.clients.lock().unwrap();
    clients.retain(|_, state| state.strong_count() > 0);
    clients.insert(id, state);
    drop(clients);

    self.worker.get_or_init(|| thread::spawn(worker).thread().clone()).unpark();
    self.changed.send_replace(());
  }

  async fn remove_and_wait(&self, id: i32) {
    let mut changed = self.changed.subscribe();
    {
      let mut clients = self.clients.lock().unwrap();
      clients.remove(&id);
      clients.retain(|_, state| state.strong_count() > 0);
      if !clients.is_empty() {
        return;
      }
      changed.borrow_and_update();
    }
    let _ = changed.changed().await;
  }

  fn dispatch(&self, raw: &[u8]) {
    let Ok(envelope) = serde_json::from_slice::<Envelope<'_>>(raw) else { return };
    let Some(state) = self.clients.lock().unwrap().get(&envelope.client).and_then(Weak::upgrade) else { return };

    if let Some(extra) = envelope.extra {
      let Some(tx) = state.pending.lock().unwrap().remove(&extra) else { return };
      let response = if envelope.kind == "error" { Err(td_error(raw)) } else { Ok(raw.to_vec()) };
      let _ = tx.send(response);
      return;
    }

    let event = if envelope.kind == "error" { Err(td_error(raw)) } else { serde_json::from_slice(raw).map_err(Error::Json) };
    let _ = state.events.send(event);
  }
}

static ROUTER: LazyLock<Router> = LazyLock::new(|| {
  let (changed, _) = watch::channel(());
  Router { clients: Default::default(), worker: Default::default(), changed }
});

fn worker() {
  loop {
    let idle = {
      let mut clients = ROUTER.clients.lock().unwrap();
      clients.retain(|_, state| state.strong_count() > 0);
      clients.is_empty()
    };

    if idle {
      ROUTER.changed.send_replace(());
      thread::park();
      continue;
    }

    // SAFETY: this is the sole thread that calls `td_receive`.
    let raw = unsafe { td_sys::td_receive(1.0) };
    if raw.is_null() {
      continue;
    }

    // SAFETY: TDLib returned a non-null NUL-terminated string.
    let raw = unsafe { CStr::from_ptr(raw) }.to_bytes();
    ROUTER.dispatch(raw);
  }
}

fn td_error(raw: &[u8]) -> Error {
  serde_json::from_slice(raw).map_or_else(Error::Json, Error::Td)
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

pub fn set_log_verbosity_level(level: i32) {
  // SAFETY: TDLib accepts every integer verbosity level.
  unsafe { td_sys::td_set_log_verbosity_level(level) };
}
