use std::assert_matches;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::oneshot::error::TryRecvError;
use tokio::time::timeout;

use td_types::enums::{TestInt, Text};

use super::*;

fn client_state(id: i32) -> (Arc<ClientState>, mpsc::UnboundedReceiver<Result<Update>>) {
  let (updates, updates_rx) = mpsc::unbounded_channel();
  let next_request_id = Default::default();
  let registry = Mutex::new(ClientRegistry { accepting_requests: true, ..Default::default() });
  let client = Arc::new(ClientState { id, next_request_id, registry, updates });
  (client, updates_rx)
}

fn router(states: &[&Arc<ClientState>]) -> Router {
  let clients = Mutex::new(states.iter().map(|&state| (state.id, Arc::downgrade(state))).collect());
  let (clients_changed, _) = watch::channel(());
  let (receiver, native_calls) = Default::default();
  let receive_timeout = 1f64.to_bits().into();
  Router { clients, receiver, clients_changed, receive_timeout, native_calls }
}

#[test]
fn synchronous_requests_preserve_typed_results_and_errors() {
  let result = execute(&fns::getFileMimeType { file_name: "photo.jpg".into() });
  assert_matches!(result, Ok(Text::text(types::text { text })) if text == "image/jpeg");
  let error = types::error { code: 418, message: "teapot".into() };
  assert_matches!(execute(&fns::testReturnError { error: error.clone() }), Err(Error::Td(actual)) if actual == error);
}

fn pending(id: i64) -> serde_json::Value {
  serde_json::json!({
    "@type": "message",
    "id": id,
    "chat_id": 9,
    "sending_state": { "@type": "messageSendingStatePending" }
  })
}

fn file(id: i32, active: bool, uploaded: i64) -> serde_json::Value {
  serde_json::json!({
    "@type": "file",
    "id": id,
    "size": 100,
    "expected_size": 100,
    "local": {
      "path": "",
      "can_be_downloaded": true,
      "can_be_deleted": false,
      "is_downloading_active": false,
      "is_downloading_completed": false,
      "download_offset": 0,
      "downloaded_prefix_size": 0,
      "downloaded_size": 0
    },
    "remote": {
      "id": "",
      "unique_id": "",
      "is_uploading_active": active,
      "is_uploading_completed": false,
      "uploaded_size": uploaded
    }
  })
}

fn routed(mut value: serde_json::Value, client_id: i32, extra: Option<u64>) -> Vec<u8> {
  value["@client_id"] = client_id.into();
  if let Some(extra) = extra {
    value["@extra"] = extra.into();
  }
  serde_json::to_vec(&value).unwrap()
}

#[tokio::test]
async fn message_responses_bind_before_terminal_updates() {
  let (client, mut updates) = client_state(1001);
  let router = router(&[&client]);
  let (reply, response) = oneshot::channel();
  client.registry.lock().unwrap().requests.insert(7, PendingReply::Messages { many: false, reply });

  router.route(&routed(pending(10), 1001, Some(7)));
  let sends = response.await.unwrap().unwrap();
  let [send]: [MessageOperation; 1] = sends.try_into().ok().unwrap();
  assert!(client.registry.lock().unwrap().message_sends.contains_key(&MessageKey { chat_id: 9, message_id: 10 }));

  router.route(
    br#"{
      "@client_id": 1001,
      "@type": "updateMessageSendSucceeded",
      "message": {"@type": "message", "id": 20, "chat_id": 9},
      "old_message_id": 10
    }"#,
  );
  let message = send.finish(&client, None).await.unwrap();
  assert_eq!((message.chat_id, message.id), (9, 20));
  assert!(client.registry.lock().unwrap().message_sends.is_empty());
  assert_matches!(updates.try_recv(), Ok(Ok(Update::updateMessageSendSucceeded(update))) if update.old_message_id == 10);
}

#[tokio::test]
async fn message_batches_settle_independently_and_in_original_order() {
  let (client, _) = client_state(1002);
  let router = router(&[&client]);
  let (reply, response) = oneshot::channel();
  client.registry.lock().unwrap().requests.insert(8, PendingReply::Messages { many: true, reply });
  router.route(&routed(serde_json::json!({ "@type": "messages", "total_count": 2, "messages": [pending(11), pending(12)] }), 1002, Some(8)));
  let sends = response.await.unwrap().unwrap();
  let [first, second]: [MessageOperation; 2] = sends.try_into().ok().unwrap();

  router.route(
    br#"{
      "@client_id": 1002,
      "@type": "updateMessageSendFailed",
      "message": {"@type": "message", "id": 12, "chat_id": 9},
      "old_message_id": 12,
      "error": {"@type": "error", "code": 400, "message": "failed"}
    }"#,
  );
  router.route(
    br#"{
      "@client_id": 1002,
      "@type": "updateMessageSendSucceeded",
      "message": {"@type": "message", "id": 21, "chat_id": 9},
      "old_message_id": 11
    }"#,
  );

  assert_eq!(first.finish(&client, None).await.unwrap().id, 21);
  assert_matches!(second.finish(&client, None).await.err(), Some(Error::MessageFailed(update)) if update.error.code == 400);
}

#[test]
fn duplicate_message_keys_fail_without_partial_registration() {
  let (client, _) = client_state(1003);
  let messages = vec![
    types::message { chat_id: 9, id: 13, sending_state: Some(types::messageSendingStatePending::default().into()), ..Default::default() },
    types::message { chat_id: 9, id: 13, sending_state: Some(types::messageSendingStatePending::default().into()), ..Default::default() },
  ];
  let result = client.bind_messages(messages);
  assert_matches!(result.err(), Some(Error::MessageCollision(MessageKey { chat_id: 9, message_id: 13 })));
  assert!(client.registry.lock().unwrap().message_sends.is_empty());
}

#[tokio::test]
async fn observed_message_success_beats_cancellation() {
  let (client, _) = client_state(1011);
  let router = router(&[&client]);
  let messages = vec![
    types::message { chat_id: 9, id: 16, sending_state: Some(types::messageSendingStatePending::default().into()), ..Default::default() }, //.
  ];
  let [send]: [MessageOperation; 1] = client.bind_messages(messages).unwrap().try_into().ok().unwrap();
  router.route(
    br#"{
      "@client_id": 1011,
      "@type": "updateMessageSendSucceeded",
      "message": {"@type": "message", "id": 26, "chat_id": 9},
      "old_message_id": 16
    }"#,
  );

  let cancel = Cancel::new();
  cancel.cancel();
  let result = timeout(Duration::from_millis(100), send.finish(&client, Some(&cancel))).await.unwrap();
  assert_matches!(result, Ok(types::message { id: 26, .. }));
  assert!(client.registry.lock().unwrap().requests.is_empty());
}

#[tokio::test]
async fn uploads_expose_successive_coalesced_progress() {
  let (client, mut updates) = client_state(1004);
  let router = router(&[&client]);
  let (reply, response) = oneshot::channel();
  client.registry.lock().unwrap().requests.insert(9, PendingReply::Upload(reply));
  router.route(&routed(file(42, true, 1), 1004, Some(9)));
  let mut upload = response.await.unwrap().unwrap();
  assert_matches!(upload.next().await, Ok(FileProgress { file_id: 42, uploaded_size: 1, .. }));

  router.route(&routed(serde_json::json!({ "@type": "updateFile", "file": file(42, true, 60) }), 1004, None));
  assert_matches!(upload.next().await, Ok(FileProgress { uploaded_size: 60, .. }));

  router.route(&routed(serde_json::json!({ "@type": "updateFile", "file": file(42, false, 100) }), 1004, None));
  let complete = upload.next().await.unwrap();
  assert_eq!(complete.uploaded_size, 100);
  assert_matches!(updates.try_recv(), Ok(Ok(Update::updateFile(update))) if update.file.remote.uploaded_size == 60);
  assert_matches!(updates.try_recv(), Ok(Ok(Update::updateFile(update))) if update.file.remote.uploaded_size == 100);
}

#[tokio::test]
async fn upload_response_registers_progress_before_waking_requester() {
  let (client, _) = client_state(1005);
  let router = router(&[&client]);
  let (reply, response) = oneshot::channel();
  client.registry.lock().unwrap().requests.insert(9, PendingReply::Upload(reply));
  router.route(&routed(file(43, true, 1), 1005, Some(9)));
  let mut upload = response.await.unwrap().unwrap();
  assert_matches!(upload.next().await, Ok(FileProgress { file_id: 43, uploaded_size: 1, .. }));

  router.route(&routed(serde_json::json!({ "@type": "updateFile", "file": file(43, false, 100) }), 1005, None));
  let stopped = upload.next().await.unwrap();
  assert_eq!((stopped.upload, stopped.uploaded_size), (TransferState::Inactive, 100));
}

#[tokio::test]
async fn auth_waiting_buffers_application_updates() {
  let (state, updates) = client_state(1012);
  let mut client = Client { state: Arc::clone(&state), updates, buffered_updates: VecDeque::new(), closed: false };
  state.updates.send(Ok(types::updateOption::default().into())).unwrap();
  let auth = types::updateAuthorizationState { authorization_state: AuthorizationState::authorizationStateWaitPhoneNumber };
  state.updates.send(Ok(auth.into())).unwrap();

  assert_matches!(client.recv_auth().await, Ok(AuthorizationState::authorizationStateWaitPhoneNumber));
  assert_matches!(client.recv().await, Ok(Some(Update::updateOption(_))));
}

#[tokio::test]
async fn malformed_updates_wake_file_waiters_with_the_original_error() {
  let (client, mut updates) = client_state(1006);
  let router = router(&[&client]);
  let mut upload = client.file_updates(44);
  router.route(br#"{"@client_id":1006,"@type":"updateFile","file":"invalid"}"#);
  assert_matches!(upload.next().await, Err(Error::Json(_)));
  assert_matches!(updates.try_recv(), Ok(Err(Error::Json(_))));
}

#[test]
fn malformed_envelopes_fail_every_routable_request() {
  let (first, _) = client_state(1007);
  let (second, _) = client_state(1008);
  let router = router(&[&first, &second]);
  let (first_tx, mut first_rx) = oneshot::channel();
  let (second_tx, mut second_rx) = oneshot::channel();
  first.registry.lock().unwrap().requests.insert(1, PendingReply::Request(first_tx));
  second.registry.lock().unwrap().requests.insert(1, PendingReply::Request(second_tx));
  router.route(b"{");
  assert_matches!(first_rx.try_recv(), Ok(Err(Error::Json(_))));
  assert_matches!(second_rx.try_recv(), Ok(Err(Error::Json(_))));
  assert_matches!(first_rx.try_recv(), Err(TryRecvError::Closed));
}

#[tokio::test(flavor = "multi_thread")]
async fn native_multi_client_lifecycle() {
  let test = async {
    set_log_level(0);
    set_receive_timeout(Duration::from_millis(10));
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let first = Client::new(params(2040, "b18441a1ff607e10a989891a5462e627", format!("/tmp/td-client-{nonce}-first")));
    let second = Client::new(params(2040, "b18441a1ff607e10a989891a5462e627", format!("/tmp/td-client-{nonce}-second")));
    let (first, second) = tokio::join!(first, second);
    let (first, second) = (first.unwrap(), second.unwrap());
    let mime_type = execute(&fns::getFileMimeType { file_name: "photo.jpg".into() });
    assert_matches!(mime_type, Ok(Text::text(types::text { text })) if text == "image/jpeg");
    let (first_sender, second_sender) = (first.sender(), second.sender());
    let (first_request, second_request) = (fns::testSquareInt { x: 3 }, fns::testSquareInt { x: 4 });
    let (a, b) = tokio::join!(first_sender.send(&first_request), second_sender.send(&second_request));
    assert_matches!(a, Ok(TestInt::testInt(types::testInt { value: 9 })));
    assert_matches!(b, Ok(TestInt::testInt(types::testInt { value: 16 })));
    let (a, b) = tokio::join!(first.shutdown(), second.shutdown());
    a.unwrap();
    b.unwrap();
  };
  timeout(Duration::from_secs(30), test).await.unwrap();
}
