use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, Once, RwLock, Weak};
use std::thread;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use tracing::field::Empty;

use td_types::enums;
use td_types::traits::Function;

use crate::error::{Error, Result};
use crate::util::PoisonErrorExt;

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
  r#type: &'a str,
}

type ResponseSender = oneshot::Sender<Result<String>>;

pub struct ClientState {
  client_id: i32,
  extra: AtomicU64,
  pending: Mutex<HashMap<u64, ResponseSender>>,
  updates_tx: mpsc::Sender<enums::Update>,
  auth_state_tx: mpsc::Sender<enums::AuthorizationState>,
}

impl Drop for ClientState {
  fn drop(&mut self) {
    unsafe { td_sys::td_send(self.client_id, cr#"{"@type":"close","@extra":18446744073709551615}"#.as_ptr()) };
    for (_, sender) in self.pending.get_mut().into_inner().drain() {
      let _ = sender.send(Err(Error::Disconnected));
    }
  }
}

impl ClientState {
  pub fn create(log_verbosity_level: i32) -> (Arc<Self>, mpsc::Receiver<enums::Update>, mpsc::Receiver<enums::AuthorizationState>) {
    unsafe { td_sys::td_set_log_verbosity_level(log_verbosity_level) };

    let client_id = unsafe { td_sys::td_create_client_id() };
    tracing::debug!(client_id, "created new TDLib client instance");

    let (updates_tx, updates_rx) = mpsc::channel(2048);
    let (auth_state_tx, auth_state_rx) = mpsc::channel(2048);

    let extra = 1.into();
    let pending = Default::default();
    let state = Arc::new(Self { client_id, extra, pending, updates_tx, auth_state_tx });

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

    self.pending.lock().into_inner().insert(extra_id, tx);

    let mut bytes = serde_json::to_vec(&RequestEnvelope { extra: extra_id, function: fn_req })?;
    bytes.push(0);

    unsafe { td_sys::td_send(self.client_id, bytes.as_ptr().cast()) };

    let raw_res = rx.await.map_err(|_| {
      tracing::warn!(extra = extra_id, "client disconnected before receiving response");
      Error::Disconnected
    })??;

    serde_json::from_str(&raw_res).map_err(|source| {
      tracing::error!(extra = extra_id, %source, "failed to parse tdlib response payload");
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
    let mut clients = self.clients.write().into_inner();
    clients.retain(|_, weak| weak.strong_count() > 0);
    clients.insert(client_id, state);

    self.started.call_once(|| {
      let _ = thread::spawn(Self::worker_loop);
    });
  }

  fn worker_loop() {
    loop {
      let json_ptr = unsafe { td_sys::td_receive(1.0) };
      let 1.. = json_ptr.addr() else { continue };

      let json_bytes = unsafe { CStr::from_ptr(json_ptr) }.to_bytes();
      let json = unsafe { str::from_utf8_unchecked(json_bytes) };

      let Ok(env) = serde_json::from_str(json) else {
        tracing::error!("failed to parse envelope from td_receive");
        continue;
      };

      ROUTER.dispatch(&env, json);
    }
  }

  fn dispatch(&self, env: &RawEnvelope<'_>, raw_str: &str) {
    let state = env.client_id.and_then(|id| self.clients.read().into_inner().get(&id).and_then(Weak::upgrade));
    let update = env.extra.is_none().then(|| serde_json::from_str(raw_str));

    match (env.extra, state, update) {
      (Some(u64::MAX), _, _) => {
        tracing::debug!(client_id = ?env.client_id, type = env.r#type, "TDLib client close response received");
      }

      (Some(extra), Some(state), _) => {
        let Some(sender) = state.pending.lock().into_inner().remove(&extra) else {
          tracing::warn!(client_id = state.client_id, extra, type = env.r#type, "no pending request matching extra id");
          return;
        };

        let res = match env.r#type {
          "error" => match serde_json::from_str(raw_str) {
            Ok(err) => Err(Error::Td(err)),
            Err(e) => {
              tracing::error!(client_id = state.client_id, extra, %e, "failed to parse tdlib error response payload");
              Err(Error::Sys("failed to parse tdlib error payload".into()))
            }
          },
          _ => Ok(raw_str.to_string()),
        };

        let _ = sender.send(res);
      }

      (Some(extra), None, _) => {
        tracing::warn!(client_id = ?env.client_id, extra, type = env.r#type, "dropped response: no matching client found");
      }

      (None, Some(state), Some(Ok(enums::Update::updateAuthorizationState(auth)))) if !state.auth_state_tx.is_closed() => {
        if let Err(e) = state.auth_state_tx.try_send(auth.authorization_state) {
          tracing::warn!(client_id = state.client_id, ?e, "failed to deliver authorization state");
        }
      }

      (None, Some(state), Some(Ok(update))) => {
        if let Err(e) = state.updates_tx.try_send(update) {
          tracing::warn!(client_id = state.client_id, ?e, "failed to deliver update to receiver channel");
        }
      }

      (None, None, Some(Ok(_))) => {
        tracing::warn!(client_id = ?env.client_id, type = env.r#type, "dropped update: client instance not found");
      }

      (None, _, _) => match env.r#type {
        "ok" => {}
        "error" => match serde_json::from_str(raw_str) {
          Ok(enums::Error::error(err)) => tracing::error!(client_id = ?env.client_id, ?err, "unsolicited tdlib error received"),
          Err(e) => tracing::error!(client_id = ?env.client_id, %e, "unsolicited unparseable tdlib error received"),
        },
        _ => tracing::warn!(client_id = ?env.client_id, type = env.r#type, "ignored unhandled tdlib event"),
      },
    }
  }
}
