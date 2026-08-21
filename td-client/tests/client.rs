use std::assert_matches;
use std::sync::Once;
use std::time::UNIX_EPOCH;

use tracing_subscriber::EnvFilter;

use td_client::{Config, presets};
use td_types::enums::*;
use td_types::{fns, types};

static INIT_LOGS: Once = Once::new();

fn init_logs() {
  INIT_LOGS.call_once(|| {
    td_client::set_log_verbosity_level(1);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug,td_client=trace"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).with_test_writer().try_init();
  });
}

fn test_config(name: &str) -> Config {
  let uid = UNIX_EPOCH.elapsed().map_or(0, |d| d.as_nanos());
  let td = fns::setTdlibParameters {
    database_directory: format!("../target/test/{name}_{uid}/db"),
    files_directory: format!("../target/test/{name}_{uid}/files"),
    ..presets::DESKTOP.into()
  };

  Config { td }
}

#[tokio::test]
async fn sync_execution() {
  init_logs();

  let req = fns::getOption { name: "version".into() };
  let res = td_client::execute_sync(&req);
  assert_matches!(res, Ok(OptionValue::optionValueString(ver)) if ver.value.split('.').count() == 3);

  let err_req = fns::testReturnError { error: types::error { code: 418, message: "I'm a teapot".into() } };
  let err_res = td_client::execute_sync(&err_req);
  assert_matches!(err_res, Err(td_client::Error::Td(err)) if err.code == 418);
}

#[tokio::test]
async fn async_test_calls() {
  init_logs();

  let (client, _updates) = td_client::start();

  let res = client.execute(&fns::testCallEmpty {}).await;
  assert_matches!(res, Ok(Ok::ok));

  let input = "Async TDLib runtime test 🚀".to_string();
  let res = client.execute(&fns::testCallString { x: input.clone() }).await;
  assert_matches!(res, Ok(TestString::testString(types::testString { value })) if value == input);

  let input = b"\x00\xFFbinary\xFE\x01".to_vec();
  let res = client.execute(&fns::testCallBytes { x: input.clone() }).await;
  assert_matches!(res, Ok(TestBytes::testBytes(types::testBytes { value })) if value == input);

  let input = vec![1, 2, 3, 4, 5];
  let res = client.execute(&fns::testCallVectorInt { x: input.clone() }).await;
  assert_matches!(res, Ok(TestVectorInt::testVectorInt(types::testVectorInt { value })) if value == input);

  let input = 7;
  let res = client.execute(&fns::testSquareInt { x: input }).await;
  assert_matches!(res, Ok(TestInt::testInt(types::testInt { value })) if value == input * input);

  let input = types::error { code: 404, message: "Not Found".into() };
  let res = client.execute(&fns::testReturnError { error: input.clone() }).await;
  assert_matches!(res, Err(td_client::Error::Td(value)) if value == input);
}

#[tokio::test]
async fn concurrent_request_correlation() {
  init_logs();

  let (client, _updates) = td_client::start();

  let mut handles = Vec::new();
  for i in 1..=25 {
    let client = client.clone();
    handles.push(tokio::spawn(async move {
      let res = client.execute(&fns::testSquareInt { x: i }).await;
      assert_matches!(res, Ok(TestInt::testInt(types::testInt { value })) if value == i * i);
    }));
  }

  for h in handles {
    assert_matches!(h.await, Ok(()));
  }
}

#[tokio::test]
async fn multi_client_routing() {
  init_logs();

  let (client1, _updates1) = td_client::start();
  let (client2, _updates2) = td_client::start();

  assert_ne!(client1.id(), client2.id());

  let req1 = client1.execute(&fns::testSquareInt { x: 4 });
  let req2 = client2.execute(&fns::testSquareInt { x: 6 });
  let (res1, res2) = tokio::join!(req1, req2);

  assert_matches!(res1, Ok(TestInt::testInt(types::testInt { value: 16 })));
  assert_matches!(res2, Ok(TestInt::testInt(types::testInt { value: 36 })));
}

#[tokio::test]
async fn update_receiver_stream() {
  init_logs();

  let (client, mut updates) = td_client::start();

  // Sending the first request to an active client triggers TDLib initialization updates
  let res = client.execute(&fns::getOption { name: "version".into() }).await;
  assert_matches!(res, Ok(OptionValue::optionValueString(_)));

  let update = updates.recv().await;
  assert_matches!(update, Some(Update::updateOption(_) | Update::updateAuthorizationState(_)));
}

#[tokio::test]
async fn auth_lifecycle() {
  init_logs();

  let mut auth = td_client::auth(test_config("auth_lifecycle")).await.expect("start auth failed");

  let state = auth.next().await;
  assert_matches!(state, Ok(Some(AuthorizationState::authorizationStateWaitPhoneNumber)));

  let (client, _updates) = auth.finish();
  let res = client.execute(&fns::testSquareInt { x: 5 }).await;
  assert_matches!(res, Ok(TestInt::testInt(types::testInt { value: 25 })));
}
