use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, Once, PoisonError, RwLock, Weak};
use std::thread::Builder;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};
use tracing::field::Empty;

use td_types::enums;
use td_types::traits::Function;

use crate::error::{Error, Result};

#[derive(Serialize)]
struct RequestEnvelope<'a, F: Serialize> {
  #[serde(rename = "@extra")]
  extra: u64,
  #[serde(flatten)]
  function: &'a F,
}

#[derive(Deserialize)]
struct RawEnvelope<'a> {
  #[serde(rename = "@client_id")]
  client_id: Option<i32>,
  #[serde(rename = "@extra")]
  extra: Option<u64>,
  #[serde(rename = "@type")]
  type_name: &'a str,
}

type ResponseSender = oneshot::Sender<Result<String>>;

pub struct ClientState {
  client_id: i32,
  extra: AtomicU64,
  pending: Mutex<HashMap<u64, ResponseSender>>,
  updates_tx: mpsc::Sender<enums::Update>,
  auth_state_tx: watch::Sender<Option<enums::AuthorizationState>>,
}

impl Drop for ClientState {
  fn drop(&mut self) {
    tracing::debug!(client_id = self.client_id, "closing TDLib client instance on drop");
    // SAFETY: Notifying TDLib to close the client instance and release local resources.
    unsafe { td_sys::td_send(self.client_id, c"{\"@type\":\"close\"}".as_ptr()) };
    for (_, sender) in self.pending.get_mut().unwrap_or_else(PoisonError::into_inner).drain() {
      let _ = sender.send(Err(Error::Disconnected));
    }
  }
}

impl ClientState {
  pub fn create(log_verbosity_level: i32) -> (Arc<Self>, mpsc::Receiver<enums::Update>, watch::Receiver<Option<enums::AuthorizationState>>) {
    // SAFETY: Setting global TDLib logging verbosity level.
    unsafe { td_sys::td_set_log_verbosity_level(log_verbosity_level) };

    // SAFETY: Creating client ID for new TDLib client instance.
    let client_id = unsafe { td_sys::td_create_client_id() };
    tracing::info!(client_id, "created new TDLib client instance");
    let (updates_tx, updates_rx) = mpsc::channel(2048);
    let (auth_state_tx, auth_state_rx) = watch::channel(None);

    let state = Arc::new(Self { client_id, extra: 1.into(), pending: Default::default(), updates_tx, auth_state_tx });

    ROUTER.register(client_id, Arc::downgrade(&state));

    (state, updates_rx, auth_state_rx)
  }

  pub fn id(&self) -> i32 {
    self.client_id
  }

  #[tracing::instrument(name = "td::execute", skip(self, fn_req), fields(client_id = self.client_id, extra = Empty))]
  pub async fn execute<F: Function>(&self, fn_req: &F) -> Result<F::Return> {
    let extra_id = self.extra.fetch_add(1, Ordering::Relaxed);
    tracing::Span::current().record("extra", extra_id);
    let (tx, rx) = oneshot::channel();

    self.pending.lock().unwrap_or_else(PoisonError::into_inner).insert(extra_id, tx);

    let mut bytes = serde_json::to_vec(&RequestEnvelope { extra: extra_id, function: fn_req })?;
    bytes.push(0);

    tracing::debug!(extra = extra_id, "dispatching request to tdlib");

    // SAFETY: Passing a valid client_id and a null-terminated UTF-8 JSON C string.
    unsafe { td_sys::td_send(self.client_id, bytes.as_ptr().cast()) };

    let raw_res = rx.await.map_err(|_| {
      tracing::warn!(extra = extra_id, "client disconnected before receiving response");
      Error::Disconnected
    })??;

    tracing::trace!(extra = extra_id, raw = %raw_res, "received response from tdlib");

    serde_json::from_str(&raw_res).map_err(|source| {
      tracing::error!(extra = extra_id, %source, raw = %raw_res, "failed to parse tdlib response payload");
      Error::Json { source, raw: Some(raw_res) }
    })
  }
}

static ROUTER: LazyLock<Router> = LazyLock::new(Router::new);

struct Router {
  clients: RwLock<HashMap<i32, Weak<ClientState>>>,
  started: Once,
}

impl Router {
  fn new() -> Self {
    Self { clients: RwLock::new(HashMap::new()), started: Once::new() }
  }

  fn register(&self, client_id: i32, state: Weak<ClientState>) {
    tracing::debug!(client_id, "registering client in router");
    let mut clients = self.clients.write().unwrap_or_else(PoisonError::into_inner);
    clients.retain(|_, s| s.strong_count() > 0);
    clients.insert(client_id, state);
    self.ensure_worker_running();
  }

  fn ensure_worker_running(&self) {
    self.started.call_once(|| {
      let _ = Builder::new().name("tdlib-receiver".into()).spawn(Self::worker_loop);
    });
  }

  fn worker_loop() {
    tracing::debug!("started background tdlib receiver worker thread");
    loop {
      // SAFETY: td_receive is called sequentially from a single dedicated background thread.
      let ptr = unsafe { td_sys::td_receive(0.02) };
      if ptr.is_null() {
        continue;
      }

      // SAFETY: TDLib guarantees that a non-null return from td_receive is a valid null-terminated C string.
      let c_str = unsafe { CStr::from_ptr(ptr) };
      let Ok(raw_str) = c_str.to_str() else {
        tracing::error!(raw_bytes = ?c_str.to_bytes(), "invalid UTF-8 in td_receive response");
        continue;
      };

      let Ok(env) = serde_json::from_str(raw_str) else {
        tracing::error!(raw = raw_str, "failed to parse envelope from td_receive");
        continue;
      };

      ROUTER.dispatch(&env, raw_str);
    }
  }

  fn get_client(&self, id: Option<i32>) -> Option<Arc<ClientState>> {
    let clients = self.clients.read().unwrap_or_else(PoisonError::into_inner);

    match id {
      Some(id) if id > 0 => clients.get(&id)?.upgrade(),
      _ => clients.values().find_map(Weak::upgrade),
    }
  }

  fn dispatch(&self, env: &RawEnvelope<'_>, raw_str: &str) {
    let state = self.get_client(env.client_id);

    match (env.extra, state) {
      (Some(extra), Some(state)) => {
        let Some(sender) = state.pending.lock().unwrap_or_else(PoisonError::into_inner).remove(&extra) else {
          tracing::warn!(client_id = state.client_id, extra, type_name = env.type_name, "no pending request matching extra id");
          return;
        };

        let res = match env.type_name {
          "error" => match serde_json::from_str(raw_str) {
            Ok(err) => Err(Error::Td(err)),
            Err(e) => {
              tracing::error!(client_id = state.client_id, extra, %e, raw = raw_str, "failed to parse tdlib error response payload");
              Err(Error::Sys("failed to parse tdlib error payload".into()))
            }
          },
          _ => {
            tracing::trace!(client_id = state.client_id, extra, type_name = env.type_name, "tdlib response received");
            Ok(raw_str.to_string())
          }
        };

        let _ = sender.send(res);
      }

      (Some(extra), None) => {
        tracing::warn!(client_id = ?env.client_id, extra, type_name = env.type_name, raw = raw_str, "dropped response: no matching client found");
      }

      (None, state) if let Ok(update) = serde_json::from_str(raw_str) => {
        let Some(state) = state else {
          tracing::warn!(client_id = ?env.client_id, type_name = env.type_name, "dropped update: client instance not found");
          return;
        };

        if let enums::Update::updateAuthorizationState(auth) = &update {
          tracing::info!(client_id = state.client_id, state = ?auth.authorization_state, "authorization state update received");
          let _ = state.auth_state_tx.send(Some(auth.authorization_state.clone()));
        } else {
          tracing::trace!(client_id = state.client_id, type_name = env.type_name, "tdlib update dispatched");
        }

        if let Err(e) = state.updates_tx.try_send(update) {
          tracing::warn!(client_id = state.client_id, ?e, "failed to deliver update to receiver channel");
        }
      }

      (None, _) => match env.type_name {
        "ok" => tracing::trace!(client_id = ?env.client_id, "tdlib acknowledgment received"),
        "error" => match serde_json::from_str(raw_str) {
          Ok(enums::Error::error(err)) => tracing::error!(client_id = ?env.client_id, ?err, "unsolicited tdlib error received"),
          Err(_) => tracing::error!(client_id = ?env.client_id, raw = raw_str, "unsolicited unparseable tdlib error received"),
        },
        _ => tracing::warn!(client_id = ?env.client_id, type_name = env.type_name, raw = raw_str, "ignored unhandled tdlib event"),
      },
    }
  }
}
