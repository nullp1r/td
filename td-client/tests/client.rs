//! Native request routing and lifecycle scenarios, without Telegram credentials.

use std::assert_matches;
use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::timeout;

use td_client::error::{self, Error};
use td_client::{Client, Session, execute, set_log_level, set_receive_timeout};
use td_types::enums::{AuthorizationState, TestInt, Text, Update};
use td_types::{fns, types};

mod support;

use support::{BadRequest, WrongReturn, test_parameters, test_root};

#[tokio::test]
async fn native_client_boundary() -> Result<()> {
  let root = test_root();
  let result = timeout(Duration::from_secs(30), exercise(&root)).await.context("native client test timed out");
  let cleanup = fs::remove_dir_all(root).context("failed to remove test directory");

  result??;
  cleanup
}

async fn exercise(root: &Path) -> Result<()> {
  set_log_level(0).context("failed to configure native log level")?;
  set_receive_timeout(Duration::from_millis(10));

  // A rejected construction must release its native client before the next opens.
  rejects_invalid_parameters(root).await;
  let first = Session::open(test_parameters(root, "first"));
  let second = Session::open(test_parameters(root, "second"));
  let (first, second) = tokio::join!(first, second);
  let mut first = first.context("first client failed to start")?;
  let second = second.context("second client failed to start")?;
  let (first_client, second_client) = (first.client(), second.client());

  // Request correlation is independent of authentication/update consumption.
  requests(&first_client, &second_client).await?;
  wait_for_phone_number(&mut first).await;
  let buffered = first.recv().await;
  assert_matches!(buffered, Some(Update::updateOption(_)));

  // Explicit native closure followed by Session::close must remain valid.
  first_client.send(&fns::close {}).await.context("close request failed")?;
  while let Some(_) = first.recv().await {}
  first.close().await.context("already-closed client failed to shut down")?;

  lifecycle_races(root, second, second_client).await
}

async fn rejects_invalid_parameters(root: &Path) {
  let mut invalid = test_parameters(root, "invalid");
  invalid.system_language_code.clear();
  let result = Session::open(invalid).await;
  let error = result.err();
  assert_matches!(error, Some(Error::Td(error)) if error.code == 400);
}

async fn requests(first: &Client, second: &Client) -> Result<()> {
  let result = first.send(&BadRequest).await;
  assert_matches!(result, Err(Error::Json(_)));
  let result = first.send(&WrongReturn).await;
  assert_matches!(result, Err(Error::Json(_)));

  // Overlap requests on one client and across clients; each must get its own reply.
  let detached = first.clone();
  let detached = tokio::spawn(async move { square(&detached, 4).await });
  let (same_client, other_client) = tokio::join!(square(first, 3), square(second, 5));
  assert_eq!(same_client.context("same-client request failed")?, 9);
  assert_eq!(detached.await.context("detached request task panicked")?.context("detached request failed")?, 16);
  assert_eq!(other_client.context("cross-client request failed")?, 25);

  let error = types::error { code: 418, message: "teapot".into() };
  let request = fns::testReturnError { error: error.clone() };
  let result = first.send(&request).await;
  assert_matches!(result, Err(Error::Td(actual)) if actual == error);

  let response = execute(&fns::getFileMimeType { file_name: "photo.jpg".into() }) //.
    .context("synchronous request failed")?;
  let Text::text(mime_type) = response;
  assert_eq!(mime_type.text, "image/jpeg");
  let result = execute(&fns::testReturnError { error: error.clone() });
  assert_matches!(result, Err(Error::Td(actual)) if actual == error);

  Ok(())
}

async fn square(client: &Client, value: i32) -> error::Result<i32> {
  let request = fns::testSquareInt { x: value };
  let TestInt::testInt(result) = client.send(&request).await?;
  Ok(result.value)
}

async fn wait_for_phone_number(session: &mut Session) {
  loop {
    if let AuthorizationState::authorizationStateWaitPhoneNumber = session.recv_auth().await {
      return;
    }
  }
}

async fn lifecycle_races(root: &Path, second: Session, stale: Client) -> Result<()> {
  // Register a new session while the last existing sessions are shutting down.
  let third = Session::open(test_parameters(root, "third")).await.context("third client failed to start")?;
  let racing = Session::open(test_parameters(root, "racing"));
  let (second, third, racing) = tokio::join!(second.close(), third.close(), racing);
  second.context("second client failed to shut down")?;
  third.context("third client failed to shut down")?;
  racing.context("racing client failed to start")?.close().await.context("racing client failed to shut down")?;
  let request = fns::testSquareInt { x: 6 };
  let result = stale.send(&request).await;
  assert_matches!(result, Err(Error::Disconnected));

  // Shutdown may win or lose the race with a detached request.
  let closing = Session::open(test_parameters(root, "closing")).await.context("closing client failed to start")?;
  let request = closing.client();
  let request = tokio::spawn(async move { square(&request, 6).await });
  closing.close().await.context("closing client failed to shut down")?;
  let result = request.await.context("racing request task panicked")?;
  assert_matches!(result, Ok(36) | Err(Error::Disconnected));

  // The receiver must be usable again after its registry becomes empty.
  let restarted = Session::open(test_parameters(root, "restart")).await.context("restarted client failed to start")?;
  restarted.close().await.context("restarted client failed to shut down")?;

  Ok(())
}
