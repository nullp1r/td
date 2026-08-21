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
  fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
    Err(S::Error::custom("intentional failure"))
  }
}

impl Function for BadRequest {
  type Return = enums::Ok;
}

fn parameters(name: &str) -> fns::setTdlibParameters {
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
  timeout(Duration::from_secs(30), async {
    td_client::set_log_verbosity_level(0);
    let client = async |name| Client::new(parameters(name)).await.unwrap();

    let mut invalid = parameters("invalid");
    invalid.system_language_code.clear();
    assert_matches!(Client::new(invalid).await.err(), Some(Error::Td(error)) if error.code == 400);

    let (first, second) = tokio::join!(client("first"), client("second"));
    let mut first = first;
    let second = second;

    assert_matches!(first.execute(&BadRequest).await, Err(Error::Json(_)));
    let first_request = fns::testSquareInt { x: 3 };
    let first_request_too = fns::testSquareInt { x: 4 };
    let second_request = fns::testSquareInt { x: 5 };
    let (a, b, c) = tokio::join!(first.execute(&first_request), first.execute(&first_request_too), second.execute(&second_request),);
    assert_matches!(a, Ok(TestInt::testInt(types::testInt { value: 9 })));
    assert_matches!(b, Ok(TestInt::testInt(types::testInt { value: 16 })));
    assert_matches!(c, Ok(TestInt::testInt(types::testInt { value: 25 })));
    let expected = types::error { code: 418, message: "teapot".into() };
    assert_matches!(first.execute(&fns::testReturnError { error: expected.clone() }).await, Err(Error::Td(error)) if error == expected);

    while !matches!(first.auth().await.unwrap(), AuthorizationState::authorizationStateWaitPhoneNumber) {}
    assert_matches!(first.recv().await.unwrap(), Some(Update::updateOption(_)));

    first.execute(&fns::close {}).await.unwrap();
    while first.recv().await.unwrap().is_some() {}
    assert_matches!(first.recv().await, Ok(None));

    let (a, b, third) = tokio::join!(first.shutdown(), second.shutdown(), client("third"));
    a.unwrap();
    b.unwrap();
    third.shutdown().await.unwrap();

    client("restart").await.shutdown().await.unwrap();
  })
  .await
  .unwrap();
}
