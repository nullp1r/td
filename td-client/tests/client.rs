#![expect(missing_docs, reason = "test crate")]

use std::assert_matches;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fs, process};

use serde::Serialize;
use serde::ser::Error as _;
use tokio::time::timeout;

use td_client::client::{Client, Sender, params};
use td_client::error::{self, Error};
use td_client::native::{execute, set_log_level, set_receive_timeout};
use td_types::enums::{AuthorizationState, TestInt, Text, Update};
use td_types::traits::Function;
use td_types::{enums, fns, types};

const API_ID: i32 = 2040;
const API_HASH: &str = "b18441a1ff607e10a989891a5462e627";

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

#[tokio::test]
async fn native_client_boundary() {
  let root = test_root();
  timeout(Duration::from_secs(30), exercise(&root)).await.expect("native client test timed out");
  fs::remove_dir_all(root).expect("failed to remove test directory");
}

async fn exercise(root: &Path) {
  set_log_level(0);
  set_receive_timeout(Duration::from_millis(10));

  rejects_invalid_parameters(root).await;
  let first = Client::new(parameters(root, "first"));
  let second = Client::new(parameters(root, "second"));
  let (first, second) = tokio::join!(first, second);
  let mut first = first.expect("first client failed to start");
  let second = second.expect("second client failed to start");
  let (first_sender, second_sender) = (first.sender(), second.sender());

  requests(&first_sender, &second_sender).await;
  wait_for_phone_number(&mut first).await;
  let buffered = first.recv().await;
  assert_matches!(buffered, Some(Update::updateOption(_)));

  first_sender.send(&fns::close {}).await.expect("close request failed");
  while let Some(_) = first.recv().await {}
  first.shutdown().await.expect("already-closed client failed to shut down");

  lifecycle_races(root, second, second_sender).await;
}

async fn rejects_invalid_parameters(root: &Path) {
  let mut invalid = parameters(root, "invalid");
  invalid.system_language_code.clear();
  let result = Client::new(invalid).await;
  let error = result.err();
  assert_matches!(error, Some(Error::Td(error)) if error.code == 400);
}

async fn requests(first: &Sender, second: &Sender) {
  let result = first.send(&BadRequest).await;
  assert_matches!(result, Err(Error::Json(_)));
  let result = first.send(&WrongReturn).await;
  assert_matches!(result, Err(Error::Json(_)));

  let detached = first.clone();
  let detached = tokio::spawn(async move { square(&detached, 4).await });
  let (same_client, other_client) = tokio::join!(square(first, 3), square(second, 5));
  assert_eq!(same_client.expect("same-client request failed"), 9);
  assert_eq!(detached.await.expect("detached request task panicked").expect("detached request failed"), 16);
  assert_eq!(other_client.expect("cross-client request failed"), 25);

  let error = types::error { code: 418, message: "teapot".into() };
  let result = first.send(&fns::testReturnError { error: error.clone() }).await;
  assert_matches!(result, Err(Error::Td(actual)) if actual == error);

  let response = execute(&fns::getFileMimeType { file_name: "photo.jpg".into() }).expect("synchronous request failed");
  let Text::text(mime_type) = response;
  assert_eq!(mime_type.text, "image/jpeg");
  let result = execute(&fns::testReturnError { error: error.clone() });
  assert_matches!(result, Err(Error::Td(actual)) if actual == error);
}

async fn square(sender: &Sender, value: i32) -> error::Result<i32> {
  let TestInt::testInt(result) = sender.send(&fns::testSquareInt { x: value }).await?;
  Ok(result.value)
}

async fn wait_for_phone_number(client: &mut Client) {
  loop {
    if let AuthorizationState::authorizationStateWaitPhoneNumber = client.recv_auth().await {
      return;
    }
  }
}

async fn lifecycle_races(root: &Path, second: Client, stale: Sender) {
  let third = Client::new(parameters(root, "third")).await.expect("third client failed to start");
  let racing = Client::new(parameters(root, "racing"));
  let (second, third, racing) = tokio::join!(second.shutdown(), third.shutdown(), racing);
  second.expect("second client failed to shut down");
  third.expect("third client failed to shut down");
  racing.expect("racing client failed to start").shutdown().await.expect("racing client failed to shut down");
  let result = stale.send(&fns::testSquareInt { x: 6 }).await;
  assert_matches!(result, Err(Error::Disconnected));

  let closing = Client::new(parameters(root, "closing")).await.expect("closing client failed to start");
  let request = closing.sender();
  let request = tokio::spawn(async move { square(&request, 6).await });
  closing.shutdown().await.expect("closing client failed to shut down");
  let result = request.await.expect("racing request task panicked");
  assert_matches!(result, Ok(36) | Err(Error::Disconnected));

  let restarted = Client::new(parameters(root, "restart")).await.expect("restarted client failed to start");
  restarted.shutdown().await.expect("restarted client failed to shut down");
}

fn parameters(root: &Path, name: &str) -> fns::setTdlibParameters {
  let mut params = params(API_ID, API_HASH, root.join(name));
  params.use_file_database = false;
  params.use_chat_info_database = false;
  params.use_message_database = false;
  params
}

fn test_root() -> PathBuf {
  let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock predates Unix").as_nanos();
  env::temp_dir().join(format!("td-client-{}-{nonce}", process::id()))
}
