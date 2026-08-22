use std::assert_matches;
use std::process;
use std::time::Duration;

use tokio::sync::oneshot::error::TryRecvError;
use tokio::time::timeout;

use td_types::enums::OptionValue;

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
  let (next_extra, replies) = Default::default();
  let requests = Mutex::new(PendingRequests { accepting: true, replies });
  let state = Arc::new(ClientState { id, next_extra, requests, events });
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
  assert_matches!(client.recv().await, Ok(Some(Update::updateOption(o))) if o.name == "first");
  assert_matches!(client.recv().await, Ok(Some(Update::updateOption(o))) if o.name == "second");
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
async fn shutdown_cleanup_covers_event_error_and_disconnect() {
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

    let mut client = Client::new(params("shutdown-disconnect")).await.expect("client failed to start");
    let sender = client.sender();
    let id = client.state.id;
    client.events.close();

    let res = client.shutdown().await;
    assert_matches!(res, Err(Error::Disconnected));
    let registered = ROUTER.clients.lock().unwrap().contains_key(&id);
    assert!(!registered);
    assert_eq!(sender.0.strong_count(), 0);
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
  first.requests.lock().unwrap().replies.insert(7, first_tx);
  second.requests.lock().unwrap().replies.insert(7, second_tx);

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

  router.route_message(br#"{"@client_id":1,"@type":"updateOption","name":"version","value":{"@type":"optionValueString","value":"1.8.66"}}"#);
  let event = first_events.try_recv();
  assert_matches!(event,
    Ok(Ok(Update::updateOption(types::updateOption { name, value: OptionValue::optionValueString(o) })))
    if name == "version" && o.value == "1.8.66");
  let event = second_events.try_recv();
  assert_matches!(event, Err(mpsc::error::TryRecvError::Empty));

  router.route_message(b"{");
  let event = first_events.try_recv();
  assert_matches!(event, Ok(Err(Error::Json(_))));
  let event = second_events.try_recv();
  assert_matches!(event, Ok(Err(Error::Json(_))));
}
