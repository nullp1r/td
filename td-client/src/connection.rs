// Shared request admission and correlation. Futures may retain Connection,
// but only Session owns update consumption and the right to keep it operational.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

use td_types::enums::{AuthorizationState, Update};
use td_types::traits::Function;
use td_types::{enums, fns, types};

use crate::diagnostics;
use crate::error::{Error, Result};
use crate::message::MessageKey;
use crate::runtime::{self, parse_error};

use self::tracking::{Observation, PendingMessages, parse_messages};

pub mod tracking;

pub struct Connection {
  pub id: i32,
  registry: Mutex<Registry>,
  next_request_id: AtomicU64,
  application_updates: mpsc::UnboundedSender<Update>,
}

#[derive(Default)]
struct Registry {
  // Default is disconnected: resetting drops all reply senders and closes
  // admission together. Construction explicitly opens admission.
  accepting_requests: bool,
  pending_requests: HashMap<u64, PendingReply>,
  pending_messages: HashMap<MessageKey, oneshot::Sender<Result<types::message>>>,
  file_observers: Vec<Observation>,
}

enum PendingReply {
  Direct(oneshot::Sender<Result<Vec<u8>>>),
  Close(oneshot::Sender<Result>),
  Messages { progress: bool, reply: oneshot::Sender<Result<PendingMessages>> },
}

#[derive(Serialize)]
struct Envelope<'a, F> {
  #[serde(rename = "@extra")]
  extra: u64,
  #[serde(flatten)]
  request: &'a F,
}

impl Connection {
  pub fn create() -> (Arc<Self>, mpsc::UnboundedReceiver<Update>) {
    // SAFETY: No pointers or borrowed storage are passed; TDLib returns an opaque ID.
    let id = unsafe { td_sys::td_create_client_id() };
    let (application_updates, receiver) = mpsc::unbounded_channel();
    let registry = Mutex::new(Registry { accepting_requests: true, ..Default::default() });
    let connection = Arc::new(Self { id, registry, next_request_id: AtomicU64::new(0), application_updates });
    runtime::register(id, Arc::downgrade(&connection));
    (connection, receiver)
  }

  pub async fn request<F: Function>(&self, request: &F) -> Result<F::Return> {
    let (reply, response) = oneshot::channel();
    self.submit(request, PendingReply::Direct(reply))?;
    let raw = response.await.map_err(|_| Error::Disconnected)??;
    serde_json::from_slice(&raw).map_err(Into::into)
  }

  pub async fn close(&self) -> Result {
    let (reply, response) = oneshot::channel();
    self.submit(&fns::close {}, PendingReply::Close(reply))?;
    response.await.map_err(|_| Error::Disconnected)?
  }

  fn submit<F: Function>(&self, request: &F, reply: PendingReply) -> Result {
    let extra = self.next_request_id.fetch_add(1, Ordering::Relaxed);
    let mut request = serde_json::to_vec(&Envelope { extra, request })?;
    request.push(0);
    let mut registry = self.registry.lock().unwrap();
    // In-flight futures can retain this connection after Client drops. The closed
    // update receiver revokes their detached senders even while Weak can upgrade.
    if !registry.accepting_requests || self.application_updates.is_closed() {
      return Err(Error::Disconnected);
    }
    if let PendingReply::Close(_) = reply {
      registry.accepting_requests = false;
    }
    registry.pending_requests.insert(extra, reply);
    // Admission stays locked until submission so close cannot overtake a request.
    // SAFETY: The identifier came from TDLib; request is live and NUL-terminated.
    unsafe { td_sys::td_send(self.id, request.as_ptr().cast()) };
    Ok(())
  }

  pub fn complete_request(&self, extra: u64, kind: &str, raw: &[u8]) {
    // Decode outside the registry lock. Only the native receiver routes events,
    // so subsequent terminal updates cannot overtake binding here.
    let Some(reply) = self.registry.lock().unwrap().pending_requests.remove(&extra) else { return };
    let response = if kind == "error" { Err(parse_error(raw)) } else { Ok(raw) };
    match reply {
      PendingReply::Direct(reply) => {
        // The native buffer expires at the next receive/execute call. Direct
        // callers deserialize after wakeup and need their own byte copy.
        let _ = reply.send(response.map(<[u8]>::to_vec));
      }
      PendingReply::Close(reply) => {
        let closed = response.and_then(|raw| serde_json::from_slice::<enums::Ok>(raw).map_err(Into::into));
        let _ = reply.send(closed.map(|_| ()));
      }
      PendingReply::Messages { progress, reply } => {
        let messages = response.and_then(|raw| parse_messages(raw, kind));
        let batch = messages.map(|messages| self.registry.lock().unwrap().bind(messages, progress));
        // Direct-response-before-terminal ordering is source-derived, not a
        // schema guarantee: re-audit on TDLib upgrades. Bind every message/file
        // observer before waking the requesting future.
        let _ = reply.send(batch);
      }
    }
  }

  pub fn update(&self, raw: &[u8]) {
    let update = match serde_json::from_slice(raw) {
      Ok(update) => update,
      Err(error) => return diagnostics::report(error),
    };
    {
      let mut registry = self.registry.lock().unwrap();
      match &update {
        Update::updateAuthorizationState(types::updateAuthorizationState {
          authorization_state: AuthorizationState::authorizationStateClosed, //.
        }) => {
          // Release waiters before delivering Closed to the lifecycle owner.
          *registry = Registry::default();
        }
        Update::updateFile(update) => registry.observe_file(&update.file),
        update => registry.observe_message(update),
      }
    }
    // Tracking borrowed the update; enqueue the original. A closed receiver
    // here means the application owner abandoned its stream.
    let _ = self.application_updates.send(update);
  }

  pub fn disconnect(&self) {
    *self.registry.lock().unwrap() = Registry::default();
  }
}

impl Drop for Connection {
  fn drop(&mut self) {
    // Local bookkeeping only; native close and handoff belong to shutdown.
    runtime::remove(self.id);
  }
}

#[cfg(test)]
mod tests;
