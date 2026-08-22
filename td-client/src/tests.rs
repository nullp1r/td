use std::assert_matches;
use std::process;
use std::time::Duration;

use tokio::sync::oneshot::error::TryRecvError;
use tokio::time::timeout;

use td_types::enums::OptionValue;

use super::*;

fn params(name: &str) -> fns::setTdlibParameters {
  let directory = format!("/tmp/td-client-{}-{name}", process::id());
  fns::setTdlibParameters {
    api_id: 2040,
    api_hash: "b18441a1ff607e10a989891a5462e627".into(),
    database_directory: format!("{directory}/db"),
    files_directory: format!("{directory}/files"),
    ..defaults()
  }
}

fn state(id: i32) -> (Arc<State>, mpsc::UnboundedReceiver<Result<Update>>) {
  let (events, rx) = mpsc::unbounded_channel();
  let (extra, pending) = Default::default();
  let state = Arc::new(State { id, extra, pending, events });
  (state, rx)
}

fn fake_client(id: i32) -> Client {
  let (state, events) = state(id);
  let (queued, closed) = Default::default();
  Client { state, events, queued, closed }
}

fn router(states: &[&Arc<State>]) -> Router {
  let clients = states.iter().map(|state| (state.id, Arc::downgrade(state))).collect();
  let (changed, _) = watch::channel(());
  let (worker, timeout) = Default::default();
  Router { clients: Mutex::new(clients), worker, changed, timeout }
}

fn option(name: &str) -> Update {
  let value = OptionValue::optionValueString(types::optionValueString { value: name.into() });
  Update::updateOption(types::updateOption { name: name.into(), value })
}

fn auth(authorization_state: AuthorizationState) -> Update {
  Update::updateAuthorizationState(types::updateAuthorizationState { authorization_state })
}

fn emit(client: &Client, update: Update) {
  client.state.events.send(Ok(update)).expect("client unexpectedly dropped its event receiver");
}

#[tokio::test]
async fn auth_and_recv_preserve_order() {
  let mut client = fake_client(0);
  emit(&client, option("first"));
  emit(&client, auth(AuthorizationState::authorizationStateWaitPhoneNumber));
  emit(&client, auth(AuthorizationState::authorizationStateReady));
  emit(&client, option("second"));
  emit(&client, auth(AuthorizationState::authorizationStateClosed));

  assert_matches!(client.auth().await, Ok(AuthorizationState::authorizationStateWaitPhoneNumber));
  assert_matches!(client.recv().await, Ok(Some(Update::updateOption(o))) if o.name == "first");
  assert_matches!(client.recv().await, Ok(Some(Update::updateOption(o))) if o.name == "second");
  assert_matches!(client.recv().await, Ok(None));
  assert_matches!(client.recv().await, Ok(None));

  let mut bot = fake_client(0);
  emit(&bot, auth(AuthorizationState::authorizationStateClosing));
  let result = bot.authorize_bot("unused").await;
  assert_matches!(result, Err(Error::Auth(AuthorizationState::authorizationStateClosing)));
}

#[test]
fn protocol_routes_correlated_responses_and_events() {
  let req = Request { extra: 7, request: &fns::testSquareInt { x: 3 } };
  let req = serde_json::to_vec(&req).expect("request failed to serialize");
  assert_eq!(req, br#"{"@extra":7,"@type":"testSquareInt","x":3}"#);

  let (first, mut first_events) = state(1);
  let (second, mut second_events) = state(2);
  let router = router(&[&first, &second]);
  let (first_tx, mut first_reply) = oneshot::channel();
  let (second_tx, mut second_reply) = oneshot::channel();
  first.pending.lock().unwrap().insert(7, first_tx);
  second.pending.lock().unwrap().insert(7, second_tx);

  let response = br#"{"@client_id":1,"@extra":7,"@type":"testInt","value":9}"#;
  router.dispatch(response);
  let reply = first_reply.try_recv().expect("first response was not routed").expect("first request failed");
  assert_eq!(reply, response);
  let reply = second_reply.try_recv();
  assert_matches!(reply, Err(TryRecvError::Empty));
  let event = first_events.try_recv();
  assert_matches!(event, Err(mpsc::error::TryRecvError::Empty));

  router.dispatch(br#"{"@client_id":2,"@extra":7,"@type":"error","code":418,"message":"teapot"}"#);
  let reply = second_reply.try_recv();
  assert_matches!(reply, Ok(Err(Error::Td(types::error { code: 418, message }))) if message == "teapot");

  router.dispatch(br#"{"@client_id":1,"@type":"updateOption","name":"version","value":{"@type":"optionValueString","value":"1.8.66"}}"#);
  let event = first_events.try_recv();
  assert_matches!(event,
    Ok(Ok(Update::updateOption(types::updateOption { name, value: OptionValue::optionValueString(types::optionValueString { value }) })))
    if name == "version" && value == "1.8.66");
  let event = second_events.try_recv();
  assert_matches!(event, Err(mpsc::error::TryRecvError::Empty));
}

#[tokio::test]
async fn shutdown_unregisters_after_event_failure() {
  let fut = async {
    set_log_verbosity_level(0);
    set_receive_timeout(0.01);

    let mut client = Client::new(params("shutdown-error")).await.expect("client failed to start");
    let retained_state = Arc::clone(&client.state);
    let id = retained_state.id;
    client.events.close();

    let result = client.shutdown().await;
    assert_matches!(result, Err(Error::Disconnected));
    let registered = ROUTER.clients.lock().unwrap().contains_key(&id);
    assert!(!registered);
    drop(retained_state);
  };

  timeout(Duration::from_secs(30), fut).await.expect("shutdown failure test timed out");
}
