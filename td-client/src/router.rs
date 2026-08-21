use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, Once, Weak};
use std::thread;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use td_types::enums;
use td_types::traits::Function;

use crate::error::{Error, Result};
use crate::util::{self, PoisonErrorExt};

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

pub struct ClientState {
  client_id: i32,
  extra: AtomicU64,
  pending: Mutex<HashMap<u64, oneshot::Sender<Result<Vec<u8>>>>>,
  updates_tx: mpsc::Sender<enums::Update>,
  auth_state_tx: mpsc::Sender<enums::AuthorizationState>,
}

impl Drop for ClientState {
  fn drop(&mut self) {
    // SAFETY: The client ID is live for the duration of `drop`, and the C string is valid and nul-terminated.
    unsafe { td_sys::td_send(self.client_id, cr#"{"@type":"close"}"#.as_ptr()) };
    ROUTER.unregister(self.client_id);
    for (_, sender) in self.pending.get_mut().into_inner().drain() {
      let _ = sender.send(Err(Error::Disconnected));
    }
  }
}

impl ClientState {
  pub(crate) fn create() -> (Arc<Self>, mpsc::Receiver<enums::Update>, mpsc::Receiver<enums::AuthorizationState>) {
    // SAFETY: TDLib creates and returns a new client ID.
    let client_id = unsafe { td_sys::td_create_client_id() };
    tracing::debug!(client_id, "created new TDLib client instance");

    let (updates_tx, updates_rx) = mpsc::channel(2048);
    let (auth_state_tx, auth_state_rx) = mpsc::channel(2048);

    let extra = AtomicU64::new(1);
    let pending = Default::default();
    let state = Arc::new(Self { client_id, extra, pending, updates_tx, auth_state_tx });

    ROUTER.register(client_id, Arc::downgrade(&state));

    (state, updates_rx, auth_state_rx)
  }

  pub fn id(&self) -> i32 {
    self.client_id
  }

  pub async fn execute<F: Function>(&self, fn_req: &F) -> Result<F::Return> {
    let extra_id = self.extra.fetch_add(1, Ordering::Relaxed);
    let bytes = util::to_c_json(&RequestEnvelope { extra: extra_id, function: fn_req })?;

    let (tx, rx) = oneshot::channel();
    self.pending.lock().into_inner().insert(extra_id, tx);

    // SAFETY: `bytes` is a valid, nul-terminated request kept alive until `td_send` returns.
    unsafe { td_sys::td_send(self.client_id, bytes.as_ptr().cast()) };

    let raw_res = rx.await.map_err(|_| {
      tracing::warn!(extra = extra_id, "client disconnected before receiving response");
      Error::Disconnected
    })??;

    serde_json::from_slice(&raw_res).map_err(|source| {
      tracing::error!(extra = extra_id, %source, "failed to parse tdlib response payload");
      Error::json(source, &raw_res)
    })
  }
}

static ROUTER: LazyLock<Router> = LazyLock::new(Router::new);

struct Router {
  clients: Mutex<HashMap<i32, Weak<ClientState>>>,
  started: Once,
}

impl Router {
  fn new() -> Self {
    Self { clients: Mutex::new(HashMap::new()), started: Once::new() }
  }

  fn register(&self, client_id: i32, state: Weak<ClientState>) {
    let mut clients = self.clients.lock().into_inner();
    clients.retain(|_, weak| weak.strong_count() > 0);
    clients.insert(client_id, state);

    self.started.call_once(|| {
      let _ = thread::spawn(Self::worker_loop);
    });
  }

  fn unregister(&self, client_id: i32) {
    self.clients.lock().into_inner().remove(&client_id);
  }

  fn worker_loop() {
    loop {
      // SAFETY: TDLib returns either null or a valid pointer to a nul-terminated JSON string.
      let json_ptr = unsafe { td_sys::td_receive(1.0) };
      let 1.. = json_ptr.addr() else { continue };

      // SAFETY: `json_ptr` is non-null and TDLib guarantees a nul-terminated response.
      let json = unsafe { util::c_json(json_ptr) };

      let Ok(env) = util::from_c_json(json) else {
        tracing::error!("failed to parse envelope from td_receive");
        continue;
      };

      ROUTER.dispatch(&env, json);
    }
  }

  fn dispatch(&self, env: &RawEnvelope<'_>, raw: &[u8]) {
    let state = env.client_id.and_then(|id| self.clients.lock().into_inner().get(&id).and_then(Weak::upgrade));

    match (env.extra, state) {
      (Some(extra), Some(state)) => {
        let Some(sender) = state.pending.lock().into_inner().remove(&extra) else {
          tracing::warn!(client_id = state.client_id, extra, type = env.r#type, "no pending request matching extra id");
          return;
        };

        let res = match env.r#type {
          "error" => match util::from_c_json(raw) {
            Ok(err) => Err(Error::Td(err)),
            Err(e) => {
              tracing::error!(client_id = state.client_id, extra, %e, "failed to parse tdlib error response payload");
              Err(Error::json(e, raw))
            }
          },
          _ => Ok(raw.to_vec()),
        };

        let _ = sender.send(res);
      }

      (Some(extra), None) => {
        tracing::warn!(client_id = ?env.client_id, extra, type = env.r#type, "dropped response: no matching client found");
      }

      (None, Some(state)) => {
        let Ok(update) = util::from_c_json(raw) else {
          tracing::error!(client_id = state.client_id, type = env.r#type, "failed to parse update payload");
          return;
        };

        match update {
          enums::Update::updateAuthorizationState(auth) if !state.auth_state_tx.is_closed() => {
            if let Err(e) = state.auth_state_tx.try_send(auth.authorization_state) {
              tracing::warn!(client_id = state.client_id, ?e, "failed to deliver authorization state");
            }
          }
          update => {
            if let Err(e) = state.updates_tx.try_send(update) {
              tracing::warn!(client_id = state.client_id, ?e, "failed to deliver update to receiver channel");
            }
          }
        }
      }

      (None, None) => match env.r#type {
        "ok" => {}
        "error" => match util::from_c_json(raw) {
          Ok(enums::Error::error(err)) => tracing::error!(client_id = ?env.client_id, ?err, "unsolicited tdlib error received"),
          Err(e) => tracing::error!(client_id = ?env.client_id, %e, "unsolicited unparseable tdlib error received"),
        },
        _ => tracing::warn!(client_id = ?env.client_id, type = env.r#type, "ignored unhandled tdlib event"),
      },
    }
  }
}
