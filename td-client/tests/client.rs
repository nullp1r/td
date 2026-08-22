use std::assert_matches;
use std::process;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde::ser::Error as _;
use tokio::time::timeout;

use td_client::{Client, Error, parameters};
use td_types::enums::{AuthorizationState, TestInt, Update};
use td_types::traits::Function;
use td_types::{enums, fns, types};

struct BadRequest;

impl Serialize for BadRequest {
  fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
    Err(S::Error::custom("intentional failure"))
  }
}

impl Function for BadRequest {
  type Return = enums::Ok;
}

struct WrongReturn;

impl Serialize for WrongReturn {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    fns::testSquareInt { x: 2 }.serialize(serializer)
  }
}

impl Function for WrongReturn {
  type Return = enums::Ok;
}

fn params(name: &str) -> fns::setTdlibParameters {
  let directory = format!("/tmp/td-client-{}-{name}", process::id());
  fns::setTdlibParameters {
    api_id: 2040,
    api_hash: "b18441a1ff607e10a989891a5462e627".into(),
    database_directory: format!("{directory}/db"),
    files_directory: format!("{directory}/files"),
    ..parameters()
  }
}

#[tokio::test]
async fn lifecycle() {
  let fut = async {
    td_client::set_log_level(0);
    td_client::set_receive_timeout(Duration::from_millis(10));

    let mut invalid = params("invalid");
    invalid.system_language_code.clear();
    let result = Client::new(invalid).await;
    assert_matches!(result, Err(Error::Td(error)) if error.code == 400);

    let (first, second) = tokio::join!(Client::new(params("first")), Client::new(params("second")));
    let mut first = first.expect("first client failed to start");
    let second = second.expect("second client failed to start");

    let result = first.send(&BadRequest).await;
    assert_matches!(result, Err(Error::Json(_)));
    let result = first.send(&WrongReturn).await;
    assert_matches!(result, Err(Error::Json(_)));

    let detached = Arc::new(first.sender());
    let a_detached = Arc::clone(&detached);
    let a_detached = tokio::spawn(async move { a_detached.send(&fns::testSquareInt { x: 4 }).await });
    let b_detached = Arc::clone(&detached);
    let b_detached = tokio::spawn(async move { b_detached.send(&fns::testSquareInt { x: 6 }).await });
    let first_request = fns::testSquareInt { x: 3 };
    let second_request = fns::testSquareInt { x: 5 };
    let (a, c) = tokio::join!(first.send(&first_request), second.send(&second_request));
    let b = a_detached.await.expect("first detached request task panicked");
    let d = b_detached.await.expect("second detached request task panicked");
    assert_matches!(a, Ok(TestInt::testInt(types::testInt { value: 9 })));
    assert_matches!(b, Ok(TestInt::testInt(types::testInt { value: 16 })));
    assert_matches!(c, Ok(TestInt::testInt(types::testInt { value: 25 })));
    assert_matches!(d, Ok(TestInt::testInt(types::testInt { value: 36 })));

    let error = types::error { code: 418, message: "teapot".into() };
    let result = first.send(&fns::testReturnError { error: error.clone() }).await;
    assert_matches!(result, Err(Error::Td(actual)) if actual == error);

    while first.recv_auth().await.expect("authorization failed") != AuthorizationState::authorizationStateWaitPhoneNumber {}
    let update = first.recv().await;
    assert_matches!(update, Ok(Some(Update::updateOption(_))));

    first.send(&fns::close {}).await.expect("close request failed");
    while first.recv().await.expect("receiving updates failed").is_some() {}
    first.shutdown().await.expect("already-closed client failed to shut down");

    let stale = second.sender();
    let third = Client::new(params("third")).await.expect("third client failed to start");
    let (second, third, racing) = tokio::join!(second.shutdown(), third.shutdown(), Client::new(params("racing")));
    second.expect("second client failed to shut down");
    third.expect("third client failed to shut down");
    racing.expect("racing client failed to start").shutdown().await.expect("racing client failed to shut down");
    let result = stale.send(&fns::testSquareInt { x: 6 }).await;
    assert_matches!(result, Err(Error::Disconnected));

    let closing = Client::new(params("closing")).await.expect("closing client failed to start");
    let request = closing.sender();
    let request = tokio::spawn(async move { request.send(&fns::testSquareInt { x: 6 }).await });
    closing.shutdown().await.expect("closing client failed to shut down");
    let request = request.await.expect("racing request task panicked");
    assert_matches!(request, Ok(TestInt::testInt(types::testInt { value: 36 })) | Err(Error::Disconnected));

    Client::new(params("restart")).await.expect("restarted client failed to start").shutdown().await.expect("restarted client failed to shut down");
  };

  timeout(Duration::from_secs(30), fut).await.expect("lifecycle timed out");
}
