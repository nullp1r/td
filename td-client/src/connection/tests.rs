//! Request routing, ownership revocation, and wire ordering.

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

  // Bind the temporary message before handing the direct response to the caller.
  connection.complete_request(7, "message", raw);
  let key = MessageKey { chat_id: 9, message_id: 10 };
  assert!(connection.registry.lock().unwrap().pending_messages.contains_key(&key));
  let batch = response.try_recv().unwrap().unwrap();

  let raw = br#"{"@type":"updateMessageSendSucceeded","old_message_id":10,"message":{"id":20,"chat_id":9}}"#;
  // Internal completion must not consume or rewrite the application update.
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

  // A correlated failure leaves the other request pending until disconnection.
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
  for (kind, raw) in [
    ("ok", &br#"{"@type":"ok"}"#[..]), //.
    ("error", br#"{"@type":"error","code":500,"message":"failed"}"#),
  ] {
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
