// Shared request admission and correlation. Futures may retain Connection,
// but only Client owns update consumption and the right to keep it operational.
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

use td_types::enums::{AuthorizationState, Update};
use td_types::traits::Function;
use td_types::{enums, fns, types};

use crate::error::{Error, Result};
use crate::message::Key;
use crate::native::{self, parse_error};
use tracking::{Observation, PendingMessages, parse_messages};

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
  pending_messages: HashMap<Key, oneshot::Sender<Result<types::message>>>,
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
    native::register(id, Arc::downgrade(&connection));
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
      Err(error) => return native::report(error),
    };
    {
      let mut registry = self.registry.lock().unwrap();
      match &update {
        Update::updateAuthorizationState(update) if let AuthorizationState::authorizationStateClosed = update.authorization_state => {
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
    native::remove(self.id);
  }
}

#[cfg(test)]
mod tests {
  use std::assert_matches;

  use td_types::fns;

  use super::*;

  impl Connection {
    pub fn fixture() -> (Arc<Self>, mpsc::UnboundedReceiver<Update>) {
      let (application_updates, receiver) = mpsc::unbounded_channel();
      let registry = Mutex::new(Registry { accepting_requests: true, ..Default::default() });
      (Arc::new(Self { id: i32::MAX, registry, next_request_id: AtomicU64::new(0), application_updates }), receiver)
    }
  }

  #[test]
  fn wire_envelope_is_flat() {
    let request = fns::testSquareInt { x: 7 };
    let raw = serde_json::to_vec(&Envelope { extra: 42, request: &request }).unwrap();
    assert_eq!(raw, br#"{"@extra":42,"@type":"testSquareInt","x":7}"#);
  }

  #[test]
  fn tracked_reply_binds_before_waking_and_updates_remain_unchanged() {
    let (connection, mut updates) = Connection::fixture();
    let (reply, mut response) = oneshot::channel();
    connection.registry.lock().unwrap().pending_requests.insert(7, PendingReply::Messages { progress: true, reply });
    let raw = br#"{"@type":"message","chat_id":9,"id":10,"sending_state":{"@type":"messageSendingStatePending"}}"#;
    connection.complete_request(7, "message", raw);
    let key = Key { chat_id: 9, message_id: 10 };
    assert!(connection.registry.lock().unwrap().pending_messages.contains_key(&key));
    let batch = response.try_recv().unwrap().unwrap();

    let raw = br#"{"@type":"updateMessageSendSucceeded","old_message_id":10,"message":{"id":20,"chat_id":9}}"#;
    connection.update(raw);
    assert!(connection.registry.lock().unwrap().pending_messages.is_empty());
    let expected: Update = serde_json::from_slice(raw).unwrap();
    assert_eq!(updates.try_recv().unwrap(), expected);
    drop(batch);
  }

  #[test]
  fn correlated_errors_and_disconnect_do_not_poison_other_requests() {
    let (connection, _updates) = Connection::fixture();
    let (first, mut first_response) = oneshot::channel();
    let (second, mut second_response) = oneshot::channel();
    {
      let mut registry = connection.registry.lock().unwrap();
      registry.pending_requests.insert(7, PendingReply::Direct(first));
      registry.pending_requests.insert(8, PendingReply::Direct(second));
    }
    connection.complete_request(7, "error", br#"{"@type":"error","code":418,"message":"teapot"}"#);
    let first = first_response.try_recv();
    assert_matches!(first, Ok(Err(Error::Td(error))) if error.code == 418);
    let untouched = second_response.try_recv();
    assert_matches!(untouched, Err(oneshot::error::TryRecvError::Empty));
    connection.disconnect();
    let abandoned = second_response.try_recv();
    assert_matches!(abandoned, Err(oneshot::error::TryRecvError::Closed));
  }

  #[test]
  fn dropping_the_owner_revokes_requests_while_connection_remains_alive() {
    let (connection, updates) = Connection::fixture();
    drop(updates);
    let (reply, _response) = oneshot::channel();
    let result = connection.submit(&fns::testSquareInt { x: 1 }, PendingReply::Direct(reply));
    assert_matches!(result, Err(Error::Disconnected));
  }

  #[test]
  fn close_replies_are_typed_and_preserve_protocol_errors() {
    let (connection, _updates) = Connection::fixture();
    for (kind, raw) in [("ok", br#"{"@type":"ok"}"#.as_slice()), ("error", br#"{"@type":"error","code":500,"message":"failed"}"#)] {
      let (reply, mut response) = oneshot::channel();
      connection.registry.lock().unwrap().pending_requests.insert(1, PendingReply::Close(reply));
      connection.complete_request(1, kind, raw);
      let result = response.try_recv().unwrap();
      match kind {
        "ok" => assert_matches!(result, Ok(())),
        _ => assert_matches!(result, Err(Error::Td(error)) if error.code == 500),
      }
    }
  }
}
