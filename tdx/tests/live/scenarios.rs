//! Each scenario verifies one observable Telegram operation, then records cleanup IDs.

use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use tdx::client::{self, Error};
use tdx::prelude::*;

use crate::PHOTO_COUNT;

use super::media;

pub async fn exercise(
  session: &mut Session, //.
  client: &Client,
  chat_id: i64,
  root: &Path,
  messages: &mut Vec<i64>,
) -> Result<()> {
  eprintln!("discovering chat");
  discover_chat(client, chat_id).await?;

  eprintln!("testing text send, terminal update, and edit");
  text_lifecycle(session, client, chat_id, messages).await?;

  eprintln!("testing single-document upload progress");
  let document_id = document_progress(client, chat_id, root, messages).await?;

  eprintln!("testing media edit through its direct response");
  edit_document(client, chat_id, document_id, root).await?;

  eprintln!("testing ten-photo album progress");
  album_progress(client, chat_id, root, messages).await?;

  eprintln!("testing pending-message cancellation");
  cancel_document(client, chat_id, root, messages).await
}

async fn discover_chat(client: &Client, chat_id: i64) -> Result<()> {
  let enums::Chat::chat(chat) = client.send(&fns::getChat { chat_id }).await?;
  ensure!(chat.id == chat_id, "getChat returned chat {} instead of {chat_id}", chat.id);
  Ok(())
}

async fn text_lifecycle(session: &mut Session, client: &Client, chat_id: i64, messages: &mut Vec<i64>) -> Result<()> {
  let request = send::message(chat_id, "tdx live test".text());
  let sent = client.track(&request, None, None).await.context("text send failed")?;
  messages.push(sent.id);
  ensure!(sent.chat_id == chat_id && sent.is_outgoing, "text send returned the wrong message");
  ensure!(sent.sending_state.is_none(), "terminal text message still has a sending state");
  expect_text(&sent, "tdx live test")?;

  // Tracking must finish without polling updates, and leave the original update queued.
  wait_for_send_success(session, chat_id, sent.id).await?;

  let request = edit::text(&sent, "tdx live test edited");
  let enums::Message::message(edited) = client.send(&request).await.context("text edit failed")?;
  ensure!(edited.chat_id == chat_id && edited.id == sent.id, "text edit returned a different message");
  expect_text(&edited, "tdx live test edited")
}

async fn wait_for_send_success(session: &mut Session, chat_id: i64, message_id: i64) -> Result<()> {
  loop {
    let Some(update) = session.recv().await else { bail!("client closed before the terminal send update") };
    if let enums::Update::updateMessageSendSucceeded(update) = update
      && update.message.chat_id == chat_id
      && update.message.id == message_id
    {
      return Ok(());
    }
  }
}

async fn document_progress(client: &Client, chat_id: i64, root: &Path, messages: &mut Vec<i64>) -> Result<i64> {
  let document = media::document(root, "progress.bin", 16 * 1024 * 1024)?;
  let request = send::message(chat_id, document);
  let (mut partial, mut invalid_index) = (false, false);
  let message = {
    let mut observe = |index, value: client::Progress| {
      invalid_index |= index != 0;
      partial |= value.current > 0 && value.current < value.total;
    };
    client.track(&request, None, Some(&mut observe)).await.context("document send failed")?
  };
  messages.push(message.id);
  ensure!(!invalid_index, "single send reported a nonzero item index");
  ensure!(partial, "document reported no in-flight progress");
  let file = message.primary_file().context("document send returned no file")?;
  expect_uploaded(file)?;
  download_document(client, file.id, file.size).await?;
  Ok(message.id)
}

async fn edit_document(client: &Client, chat_id: i64, message_id: i64, root: &Path) -> Result<()> {
  let document = media::document(root, "edited.bin", 4 * 1024 * 1024)?;
  let request = edit::media(&(chat_id, message_id), document);
  let enums::Message::message(message) = client.send(&request).await.context("media edit failed")?;
  ensure!(message.chat_id == chat_id && message.id == message_id, "media edit returned a different message");
  ensure!(message.sending_state.is_none(), "media edit returned before upload completion");
  expect_uploaded(message.primary_file().context("media edit returned no file")?)
}

async fn album_progress(client: &Client, chat_id: i64, root: &Path, sent: &mut Vec<i64>) -> Result<()> {
  let contents = media::photos(root).await?;
  let request = send::album(chat_id, contents);
  let mut partial = [false; PHOTO_COUNT];
  let mut unexpected_index = None;
  let results = {
    let mut observe = |index: usize, value: client::Progress| match partial.get_mut(index) {
      Some(partial) => *partial |= value.current > 0 && value.current < value.total,
      None => unexpected_index = Some(index),
    };
    client.track_all(&request, None, Some(&mut observe)).await.context("album request failed")?
  };

  // Register every success before checking the response, so failures still get cleanup.
  sent.extend(results.iter().filter_map(|result| result.as_ref().ok().map(|message| message.id)));
  ensure!(results.len() == PHOTO_COUNT, "album returned {} messages", results.len());
  for (index, result) in results.into_iter().enumerate() {
    let message = result.with_context(|| format!("album item {index} failed"))?;
    let file = message.primary_file().with_context(|| format!("album item {index} has no file"))?;
    expect_uploaded(file).with_context(|| format!("album item {index} is not uploaded"))?;
  }
  if let Some(index) = unexpected_index {
    bail!("progress reported out-of-range album index {index}");
  }
  ensure!(partial.contains(&true), "album reported no in-flight progress");
  Ok(())
}

async fn cancel_document(client: &Client, chat_id: i64, root: &Path, sent: &mut Vec<i64>) -> Result<()> {
  let document = media::document(root, "cancel.bin", 64 * 1024 * 1024)?;
  let request = send::message(chat_id, document);
  let cancel = client::CancellationToken::new();

  // An already-cancelled token exercises deletion of the pending temporary message.
  cancel.cancel();
  match client.track(&request, Some(&cancel), None).await {
    Err(Error::Cancelled) => Ok(()),
    Ok(message) => {
      sent.push(message.id);
      bail!("document send succeeded before cancellation won")
    }
    Err(error) => Err(error).context("document cancellation failed unexpectedly"),
  }
}

fn expect_text(message: &types::message, expected: &str) -> Result<()> {
  let text = message.text().context("expected a text message")?;
  ensure!(text.text == expected, "message text is {:?} instead of {expected:?}", text.text);
  Ok(())
}

fn expect_uploaded(file: &types::file) -> Result<()> {
  let &types::file { size, ref remote, .. } = file;
  ensure!(size > 0, "uploaded file has no size");
  ensure!(!remote.id.is_empty(), "uploaded file has no remote ID");
  ensure!(!remote.is_uploading_active, "file is still uploading after send success");
  ensure!(remote.is_uploading_completed, "file upload isn't complete after send success");
  Ok(())
}

async fn download_document(client: &Client, file_id: i32, size: i64) -> Result<()> {
  eprintln!("testing full and ranged synchronous download accounting");
  for (offset, limit, total) in [(0, 0, size), (4096, 8192, 8192)] {
    let mut request = transfer::download(file_id, 1);
    request.offset = offset;
    request.limit = limit;
    let mut invalid_sample = false;
    let file = {
      let mut observe = |index, next: client::Progress| {
        invalid_sample |= index != 0 || next.current < 0 || next.current > total;
      };
      client.download(&request, None, Some(&mut observe)).await.context("download failed")?
    };
    ensure!(file.id == file_id, "download returned a different file");
    ensure!(!invalid_sample, "download progress exceeded its requested range");
    let local = file.local;
    ensure!(local.download_offset <= offset, "download skipped the requested prefix");
    let downloaded_end = local.download_offset + local.downloaded_prefix_size;
    ensure!(downloaded_end >= offset + total, "download returned an incomplete range");
  }
  Ok(())
}
