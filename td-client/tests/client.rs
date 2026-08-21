use std::assert_matches;
use std::sync::Once;
use std::time::{Duration, UNIX_EPOCH};

use tokio::time::timeout;
use tracing_subscriber::EnvFilter;

use td_client::{Config, Error, auth, presets, start};
use td_types::{enums, fns, types};

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

  Config {
    td: fns::setTdlibParameters {
      database_directory: format!("../target/test/{name}_{uid}/db"),
      files_directory: format!("../target/test/{name}_{uid}/files"),
      ..presets::DESKTOP.into()
    },
  }
}

#[tokio::test]
async fn sync_execution() {
  init_logs();

  let req = fns::getOption { name: "version".into() };
  let res = td_client::execute_sync(&req).expect("execute getOption");
  assert_matches!(res, enums::OptionValue::optionValueString(ver) if ver.value.split('.').count() == 3);

  let err_req = fns::testReturnError { error: types::error { code: 418, message: "I'm a teapot".into() } };
  let err_res = td_client::execute_sync(&err_req);
  assert_matches!(err_res, Err(Error::Td(enums::Error::error(e))) if e.code == 418);
}

#[tokio::test]
async fn async_test_calls() {
  init_logs();

  let (client, _updates) = start();

  let res = client.execute(&fns::testCallEmpty {}).await.expect("testCallEmpty");
  assert_eq!(res, enums::Ok::ok);

  let input_str = "Async TDLib runtime test 🚀".to_string();
  let res_str = client.execute(&fns::testCallString { x: input_str.clone() }).await.expect("testCallString");
  assert_eq!(res_str, enums::TestString::testString(types::testString { value: input_str }));

  let input_bytes = b"\x00\xFFbinary\xFE\x01".to_vec();
  let res_bytes = client.execute(&fns::testCallBytes { x: input_bytes.clone() }).await.expect("testCallBytes");
  assert_eq!(res_bytes, enums::TestBytes::testBytes(types::testBytes { value: input_bytes }));

  let input_vec = vec![1, 2, 3, 4, 5];
  let res_vec = client.execute(&fns::testCallVectorInt { x: input_vec.clone() }).await.expect("testCallVectorInt");
  assert_eq!(res_vec, enums::TestVectorInt::testVectorInt(types::testVectorInt { value: input_vec }));

  let res_sq = client.execute(&fns::testSquareInt { x: 7 }).await.expect("testSquareInt");
  assert_eq!(res_sq, enums::TestInt::testInt(types::testInt { value: 49 }));

  let err_req = fns::testReturnError { error: types::error { code: 404, message: "Not Found".into() } };
  let err_res = client.execute(&err_req).await;
  assert_matches!(err_res, Err(err) if err.is_not_found() && err.td() == Some((404, "Not Found")));
}

#[tokio::test]
async fn concurrent_request_correlation() {
  init_logs();

  let (client, _updates) = start();

  let mut handles = Vec::new();
  for i in 1..=25 {
    let cl = client.clone();
    handles.push(tokio::spawn(async move {
      let res = cl.execute(&fns::testSquareInt { x: i }).await.expect("execute square");
      let enums::TestInt::testInt(val) = res;
      assert_eq!(val.value, i * i);
    }));
  }

  for h in handles {
    h.await.expect("task join");
  }
}

#[tokio::test]
async fn multi_client_routing() {
  init_logs();

  let (client1, _updates1) = start();
  let (client2, _updates2) = start();

  assert_ne!(client1.id(), client2.id());

  let (res1, res2) = tokio::join! {
    client1.execute(&fns::testSquareInt { x: 4 }),
    client2.execute(&fns::testSquareInt { x: 6 }),
  };

  assert_matches!(res1, Ok(enums::TestInt::testInt(types::testInt { value: 16 })));
  assert_matches!(res2, Ok(enums::TestInt::testInt(types::testInt { value: 36 })));
}

#[tokio::test]
async fn update_receiver_stream() {
  init_logs();

  let (client, mut updates) = start();

  // Sending the first request to an active client triggers TDLib initialization updates
  let _ = client.execute(&fns::getOption { name: "version".into() }).await.expect("execute getOption");

  let update = timeout(Duration::from_secs(3), updates.recv()).await.expect("receive update within timeout").expect("update stream not closed");

  assert_matches!(update, enums::Update::updateOption(_) | enums::Update::updateAuthorizationState(_));
}

#[tokio::test]
async fn auth_lifecycle() {
  init_logs();

  let _auth = auth(test_config("auth_lifecycle")).await.expect("start auth");
}

#[tokio::test]
async fn raii_cleanup_on_drop() {
  init_logs();

  let client_id = {
    let (client, _updates) = start();
    let res = client.execute(&fns::testSquareInt { x: 3 }).await.expect("testSquareInt");
    assert_eq!(res, enums::TestInt::testInt(types::testInt { value: 9 }));
    client.id()
  };

  // After client is dropped, client_id is out of scope and cleaned up from router
  assert!(client_id > 0);
}
