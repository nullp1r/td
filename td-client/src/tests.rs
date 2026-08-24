use std::assert_matches;
use std::process;
use std::time::Duration;

use tokio::sync::oneshot::error::TryRecvError;
use tokio::time::timeout;

use td_types::enums::OptionValue;
use td_types::types::message;

use super::message_send::{MessageKey, MessageSendOutcome};
use super::*;

fn params(name: &str) -> fns::setTdlibParameters {
  let dir = format!("/tmp/td-client-unit-{}-{name}", process::id());
  fns::setTdlibParameters {
    api_id: 2040,
    api_hash: "b18441a1ff607e10a989891a5462e627".into(),
    database_directory: format!("{dir}/db"),
    files_directory: format!("{dir}/files"),
    ..defaults()
  }
}

fn state(id: i32) -> (Arc<ClientState>, mpsc::UnboundedReceiver<Result<Update>>) {
  let (events, rx) = mpsc::unbounded_channel();
  let (next_extra, replies, message_sends) = Default::default();
  let requests = Mutex::new(PendingRequests { accepting: true, replies });
  let message_sends = Mutex::new(message_sends);
  let state = Arc::new(ClientState { id, next_extra, requests, message_sends, events });
  (state, rx)
}

fn fake_client(id: i32) -> Client {
  let (state, events) = state(id);
  let (buffered_updates, closed) = Default::default();
  Client { state, events, buffered_updates, closed }
}

fn router(states: &[&Arc<ClientState>]) -> Router {
  let clients = states.iter().map(|state| (state.id, Arc::downgrade(state))).collect();
  let (clients_changed, _) = watch::channel(());
  let (worker, stale) = Default::default();
  let timeout = 1f64.to_bits().into();
  Router { clients: Mutex::new(clients), worker, clients_changed, stale, timeout }
}

fn option(name: &str) -> Update {
  let value = types::optionValueString { value: name.into() }.into();
  types::updateOption { name: name.into(), value }.into()
}

fn auth(authorization_state: AuthorizationState) -> Update {
  types::updateAuthorizationState { authorization_state }.into()
}

fn emit(client: &Client, update: Update) {
  client.state.events.send(Ok(update)).expect("client unexpectedly dropped its event receiver");
}

#[test]
fn debug_describes_client_state() {
  let client = fake_client(42);
  assert_eq!(format!("{client:?}"), "Client { id: 42, .. }");
}

#[tokio::test]
async fn auth_and_recv_preserve_order() {
  let mut client = fake_client(-1);
  emit(&client, option("first"));
  emit(&client, auth(AuthorizationState::authorizationStateWaitPhoneNumber));
  emit(&client, auth(AuthorizationState::authorizationStateReady));
  emit(&client, option("second"));
  emit(&client, auth(AuthorizationState::authorizationStateClosed));

  assert_matches!(client.recv_auth().await, Ok(AuthorizationState::authorizationStateWaitPhoneNumber));
  assert_matches!(client.recv().await, Ok(Some(Update::updateOption(update))) if update.name == "first");
  assert_matches!(client.recv().await, Ok(Some(Update::updateOption(update))) if update.name == "second");
  assert_matches!(client.recv().await, Ok(None));
  assert_matches!(client.recv().await, Ok(None));
  assert_matches!(client.recv_auth().await, Ok(AuthorizationState::authorizationStateClosed));

  let mut bot = fake_client(-3);
  emit(&bot, auth(AuthorizationState::authorizationStateClosing));
  let res = bot.authorize_bot("unused").await;
  assert_matches!(res, Err(Error::Auth(AuthorizationState::authorizationStateClosing)));
}

#[tokio::test]
async fn detached_requests_are_rejected_after_owner_disconnects() {
  let client = fake_client(-2);
  let sender = client.sender();
  let retained = Arc::clone(&client.state);
  drop(client);

  assert_eq!(sender.0.strong_count(), 1);
  let res = sender.send(&fns::testSquareInt { x: 2 }).await;
  assert_matches!(res, Err(Error::Disconnected));
  drop(retained);
  assert_eq!(sender.0.strong_count(), 0);
}

#[tokio::test]
async fn tracked_send_rejects_preview_requests() {
  let client = fake_client(-4);
  let options = Some(types::messageSendOptions { only_preview: true, ..Default::default() });
  let request = fns::sendMessage { options, ..Default::default() };
  let result = client.sender().send_message(&request).await;
  assert_matches!(result, Err(Error::MessagePreview));
}

#[tokio::test]
async fn shutdown_cleanup_preserves_event_error() {
  let fut = async {
    set_log_level(0);
    set_receive_timeout(Duration::from_millis(10));
    let client = Client::new(params("shutdown-error")).await.expect("client failed to start");
    let sender = client.sender();
    let err = serde_json::from_slice::<Update>(b"{").expect_err("invalid JSON unexpectedly parsed");
    client.state.events.send(Err(err.into())).expect("client unexpectedly dropped its event receiver");

    let id = client.state.id;
    let res = client.shutdown().await;
    assert_matches!(res, Err(Error::Json(_)));
    let registered = ROUTER.clients.lock().unwrap().contains_key(&id);
    assert!(!registered);
    assert_eq!(sender.0.strong_count(), 0);
    let res = sender.send(&fns::testSquareInt { x: 2 }).await;
    assert_matches!(res, Err(Error::Disconnected));
  };

  timeout(Duration::from_secs(30), fut).await.expect("shutdown error test timed out");
}

#[test]
fn protocol_routes_responses_events_and_failures() {
  let request = OutgoingRequest { extra: 7, request: &fns::testSquareInt { x: 3 } };
  let request = serde_json::to_vec(&request).expect("request failed to serialize");
  assert_eq!(request, br#"{"@extra":7,"@type":"testSquareInt","x":3}"#);

  let (first, mut first_events) = state(1);
  let (second, mut second_events) = state(2);
  let router = router(&[&first, &second]);
  let (first_tx, mut first_reply) = oneshot::channel();
  let (second_tx, mut second_reply) = oneshot::channel();
  first.requests.lock().unwrap().replies.insert(7, PendingReply::Request(first_tx));
  second.requests.lock().unwrap().replies.insert(7, PendingReply::Request(second_tx));

  let response = br#"{"@client_id":1,"@extra":7,"@type":"testInt","value":9}"#;
  router.route_message(response);
  let reply = first_reply.try_recv().expect("first response was not routed").expect("first request failed");
  assert_eq!(reply, response);
  let reply = second_reply.try_recv();
  assert_matches!(reply, Err(TryRecvError::Empty));
  let event = first_events.try_recv();
  assert_matches!(event, Err(mpsc::error::TryRecvError::Empty));

  router.route_message(br#"{"@client_id":2,"@extra":7,"@type":"error","code":418,"message":"teapot"}"#);
  let reply = second_reply.try_recv();
  assert_matches!(reply, Ok(Err(Error::Td(types::error { code: 418, message }))) if message == "teapot");

  router.route_message(
    br#"{
      "@client_id": 1,
      "@type": "updateOption",
      "name": "version",
      "value": {"@type": "optionValueString", "value": "1.8.66"}
    }"#,
  );
  let event = first_events.try_recv();
  assert_matches!(event,
    Ok(Ok(Update::updateOption(types::updateOption { name, value: OptionValue::optionValueString(o) })))
    if name == "version" && o.value == "1.8.66");
  let event = second_events.try_recv();
  assert_matches!(event, Err(mpsc::error::TryRecvError::Empty));

  let (first_tx, mut first_reply) = oneshot::channel();
  let (second_tx, mut second_reply) = oneshot::channel();
  first.requests.lock().unwrap().replies.insert(8, PendingReply::Request(first_tx));
  second.requests.lock().unwrap().replies.insert(8, PendingReply::Request(second_tx));
  router.route_message(b"{");
  let reply = first_reply.try_recv();
  assert_matches!(reply, Ok(Err(Error::Json(_))));
  let reply = second_reply.try_recv();
  assert_matches!(reply, Ok(Err(Error::Json(_))));
  let event = first_events.try_recv();
  assert_matches!(event, Ok(Err(Error::Json(_))));
  let event = second_events.try_recv();
  assert_matches!(event, Ok(Err(Error::Json(_))));
}

#[test]
fn message_sends_bind_before_reply_and_preserve_terminal_updates() {
  let (state, mut events) = state(1);
  let router = router(&[&state]);

  let registration_id = 7;
  let mut send_result = state.message_sends.lock().unwrap().register(registration_id);
  let (reply, mut response) = oneshot::channel();
  state.requests.lock().unwrap().replies.insert(registration_id, PendingReply::Message(reply));

  let pending = br#"{"@client_id":1,"@extra":7,"@type":"message","id":100,"chat_id":9}"#;
  router.route_message(pending);
  let pending = response.try_recv().expect("send response was not routed").expect("send request failed");
  let message { chat_id, id, .. } = pending;
  assert_eq!((chat_id, id), (9, 100));

  let succeeded = br#"{
    "@client_id": 1,
    "@type": "updateMessageSendSucceeded",
    "message": {"@type": "message", "id": 200, "chat_id": 9},
    "old_message_id": 100
  }"#;
  router.route_message(succeeded);
  let result = send_result.try_recv().expect("send result was not routed").expect("send failed");
  assert_matches!(result, MessageSendOutcome::Succeeded(message { id: 200, chat_id: 9, .. }));
  let update = events.try_recv().expect("terminal update was not routed").expect("terminal update failed");
  assert_matches!(update, Update::updateMessageSendSucceeded(update) if (update.message.id, update.old_message_id) == (200, 100));
}

#[test]
fn message_send_terminal_updates_correlate() {
  let (state, _) = state(1);
  let router = router(&[&state]);

  let registration_id = 8;
  let mut send_result = state.message_sends.lock().unwrap().register(registration_id);
  let key = MessageKey { chat_id: 9, message_id: 101 };
  let binding = state.message_sends.lock().unwrap().bind(registration_id, key);
  binding.expect("send binding failed");
  let failed = br#"{
    "@client_id": 1,
    "@type": "updateMessageSendFailed",
    "message": {"@type": "message", "id": 101, "chat_id": 9},
    "old_message_id": 101,
    "error": {"@type": "error", "code": 400, "message": "failed"}
  }"#;
  router.route_message(failed);
  let result = send_result.try_recv().expect("send failure was not routed").expect("send result channel failed");
  assert_matches!(result, MessageSendOutcome::Failed(update) if (update.old_message_id, update.error.code) == (101, 400));

  let registration_id = 9;
  let mut send_result = state.message_sends.lock().unwrap().register(registration_id);
  let key = MessageKey { chat_id: 9, message_id: 102 };
  let binding = state.message_sends.lock().unwrap().bind(registration_id, key);
  binding.expect("send binding failed");
  router.route_message(
    br#"{
      "@client_id": 1,
      "@type": "updateDeleteMessages",
      "chat_id": 9,
      "message_ids": [102],
      "is_permanent": false,
      "from_cache": true
    }"#,
  );
  let result = send_result.try_recv();
  assert_matches!(result, Err(TryRecvError::Empty));
  router.route_message(
    br#"{
      "@client_id": 1,
      "@type": "updateDeleteMessages",
      "chat_id": 9,
      "message_ids": [102],
      "is_permanent": true,
      "from_cache": false
    }"#,
  );
  let result = send_result.try_recv().expect("send deletion was not routed").expect("send result channel failed");
  assert_matches!(result, MessageSendOutcome::Deleted);

  let (first_registration_id, second_registration_id) = (10, 11);
  let mut first_result = state.message_sends.lock().unwrap().register(first_registration_id);
  let mut second_result = state.message_sends.lock().unwrap().register(second_registration_id);
  let first_key = MessageKey { chat_id: 9, message_id: 103 };
  let binding = state.message_sends.lock().unwrap().bind(first_registration_id, first_key);
  binding.expect("first send binding failed");
  let second_key = MessageKey { chat_id: 9, message_id: 104 };
  let binding = state.message_sends.lock().unwrap().bind(second_registration_id, second_key);
  binding.expect("second send binding failed");
  router.route_message(
    br#"{
      "@client_id": 1,
      "@type": "updateMessageSendSucceeded",
      "message": {"@type": "message", "id": 204, "chat_id": 9},
      "old_message_id": 104
    }"#,
  );
  router.route_message(
    br#"{
      "@client_id": 1,
      "@type": "updateMessageSendSucceeded",
      "message": {"@type": "message", "id": 203, "chat_id": 9},
      "old_message_id": 103
    }"#,
  );
  let result = first_result.try_recv();
  assert_matches!(result, Ok(Ok(MessageSendOutcome::Succeeded(message))) if message.id == 203);
  let result = second_result.try_recv();
  assert_matches!(result, Ok(Ok(MessageSendOutcome::Succeeded(message))) if message.id == 204);
}

#[test]
fn message_send_waiters_fail_on_invalid_input_and_disconnect() {
  let (state, _) = state(1);
  let router = router(&[&state]);

  let registration_id = 12;
  let mut invalid_response_result = state.message_sends.lock().unwrap().register(registration_id);
  let (reply, mut invalid_response) = oneshot::channel();
  state.requests.lock().unwrap().replies.insert(registration_id, PendingReply::Message(reply));
  router.route_message(br#"{"@client_id":1,"@extra":12,"@type":"ok"}"#);
  assert_matches!(invalid_response.try_recv(), Ok(Err(Error::Json(_))));
  assert_matches!(invalid_response_result.try_recv(), Err(TryRecvError::Closed));

  let registration_id = 13;
  let mut malformed_result = state.message_sends.lock().unwrap().register(registration_id);
  let key = MessageKey { chat_id: 9, message_id: 105 };
  let binding = state.message_sends.lock().unwrap().bind(registration_id, key);
  binding.expect("send binding failed");
  router.route_message(
    br#"{
      "@client_id": 1,
      "@type": "updateMessageSendSucceeded",
      "message": "invalid",
      "old_message_id": 105
    }"#,
  );
  assert_matches!(malformed_result.try_recv(), Ok(Err(Error::Json(_))));

  let mut disconnected = state.message_sends.lock().unwrap().register(14);
  router.route_message(
    br#"{
      "@client_id": 1,
      "@type": "updateAuthorizationState",
      "authorization_state": {"@type": "authorizationStateClosed"}
    }"#,
  );
  assert_matches!(disconnected.try_recv(), Err(TryRecvError::Closed));
}
