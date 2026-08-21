use std::assert_matches;
use std::process;
use std::time::Duration;

use serde::Serialize;
use serde::ser::Error as _;
use tokio::time::timeout;

use td_client::{Client, Error, defaults};
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

#[tokio::test]
async fn lifecycle() {
  let client = async |name| Client::new(params(name)).await;

  let fut = async {
    td_client::set_log_verbosity_level(0);

    let mut invalid = params("invalid");
    invalid.system_language_code.clear();
    let res = Client::new(invalid).await;
    assert_matches!(res, Err(Error::Td(error)) if error.code == 400);

    let (first, second) = tokio::join!(client("first"), client("second"));
    let mut first = first.expect("first client failed to start");
    let second = second.expect("second client failed to start");

    let res = first.execute(&BadRequest).await;
    assert_matches!(res, Err(Error::Json(_)));

    let first_request = fns::testSquareInt { x: 3 };
    let first_request_too = fns::testSquareInt { x: 4 };
    let second_request = fns::testSquareInt { x: 5 };
    let (a, b, c) = tokio::join! {
      first.execute(&first_request),
      first.execute(&first_request_too),
      second.execute(&second_request),
    };
    assert_matches!(a, Ok(TestInt::testInt(types::testInt { value: 9 })));
    assert_matches!(b, Ok(TestInt::testInt(types::testInt { value: 16 })));
    assert_matches!(c, Ok(TestInt::testInt(types::testInt { value: 25 })));

    let err = types::error { code: 418, message: "teapot".into() };
    let res = first.execute(&fns::testReturnError { error: err.clone() }).await;
    assert_matches!(res, Err(Error::Td(error)) if error == err);

    loop {
      match first.auth().await.expect("authorization failed") {
        AuthorizationState::authorizationStateWaitPhoneNumber => break,
        _ => {}
      }
    }
    assert_matches!(first.recv().await, Ok(Some(Update::updateOption(_))));

    first.execute(&fns::close {}).await.expect("close request failed");
    while let Some(_) = first.recv().await.expect("receiving updates failed") {}
    assert_matches!(first.recv().await, Ok(None));

    let (first, second, third) = tokio::join!(first.shutdown(), second.shutdown(), client("third"));
    first.expect("first client failed to shut down");
    second.expect("second client failed to shut down");
    let third = third.expect("third client failed to start");
    third.shutdown().await.expect("third client failed to shut down");

    let restart = client("restart").await.expect("restarted client failed to start");
    restart.shutdown().await.expect("restarted client failed to shut down");
  };

  timeout(Duration::from_secs(30), fut).await.expect("lifecycle timed out");
}
