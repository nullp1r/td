#![expect(missing_docs, reason = "test crate")]

use std::array;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, process};

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use tokio::process::Command;
use tokio::time::timeout;

use td_client::error::Error;
use td_client::transfer::{CancellationToken, Progress};
use td_client::{Client, Session, parameters, set_log_level, set_receive_timeout};
use td_types::enums::{Chat, InputFile, InputMessageContent, Message, MessageContent, Update};
use td_types::{fns, types};

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/live/config.json");
const SESSION: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/live/session");
const OPERATION_TIMEOUT: Duration = Duration::from_secs(120);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(10);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const PHOTO_COUNT: usize = 10;

#[derive(Deserialize)]
struct Config {
  api_id: i32,
  api_hash: String,
  bot_token: String,
  chat_id: i64,
}

#[derive(Default)]
struct ProgressLog {
  partial: bool,
}

impl ProgressLog {
  fn observe(&mut self, progress: Progress) {
    self.partial |= progress.current > 0 && progress.current < progress.total;
  }
}

#[tokio::test]
#[ignore = "requires td-client/tests/live/config.json, FFmpeg, and a real Telegram test chat"]
async fn telegram_boundary() -> Result<()> {
  let config = read_config()?;
  let root = temporary_directory()?;
  fs::create_dir_all(&root).context("failed to create the live-test directory")?;

  let result = run(config, &root).await;
  let cleanup = fs::remove_dir_all(&root).context("failed to remove the live-test directory");
  result?;
  cleanup
}

async fn run(config: Config, root: &Path) -> Result<()> {
  let Config { api_id, api_hash, bot_token, chat_id } = config;
  ensure!(chat_id != 0, "chat_id must identify the dedicated Telegram test chat");
  let mut params = parameters(api_id, api_hash, SESSION);
  params.files_directory = root.to_string_lossy().into_owned();
  params.use_file_database = false;
  params.use_chat_info_database = false;
  params.use_message_database = false;

  set_log_level(0);
  set_receive_timeout(Duration::from_millis(50));
  let mut session = timeout(OPERATION_TIMEOUT, Session::bot(params, &bot_token)).await.context("bot construction timed out")??;
  let client = session.client();
  let mut messages = Vec::new();

  let exercise = timeout(OPERATION_TIMEOUT, exercise(&mut session, &client, chat_id, root, &mut messages)).await;
  let exercise = match exercise {
    Ok(result) => result,
    Err(error) => Err(error).context("live Telegram exercise timed out"),
  };
  let cleanup = timeout(CLEANUP_TIMEOUT, delete_messages(&client, chat_id, &messages)).await;
  let cleanup = match cleanup {
    Ok(result) => result,
    Err(error) => Err(error).context("remote cleanup timed out"),
  };
  let shutdown = timeout(SHUTDOWN_TIMEOUT, session.close()).await;
  let shutdown = match shutdown {
    Ok(result) => result.map_err(Into::into),
    Err(error) => Err(error).context("shutdown timed out"),
  };

  exercise?;
  cleanup?;
  shutdown
}

async fn exercise(session: &mut Session, client: &Client, chat_id: i64, root: &Path, messages: &mut Vec<i64>) -> Result<()> {
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
  let Chat::chat(chat) = client.send(&fns::getChat { chat_id }).await?;
  ensure!(chat.id == chat_id, "getChat returned chat {} instead of {chat_id}", chat.id);
  Ok(())
}

async fn text_lifecycle(session: &mut Session, client: &Client, chat_id: i64, messages: &mut Vec<i64>) -> Result<()> {
  let request = fns::sendMessage { chat_id, input_message_content: text_content("td-client live test"), ..Default::default() };
  let sent = client.track(&request, None, None).await.context("text send failed")?;
  messages.push(sent.id);
  ensure!(sent.chat_id == chat_id && sent.is_outgoing, "text send returned the wrong message");
  ensure!(sent.sending_state.is_none(), "terminal text message still has a sending state");
  expect_text(&sent, "td-client live test")?;
  wait_for_send_success(session, chat_id, sent.id).await?;

  let input_message_content = text_content("td-client live test edited");
  let request = fns::editMessageText { chat_id, message_id: sent.id, input_message_content, ..Default::default() };
  let Message::message(edited) = client.send(&request).await.context("text edit failed")?;
  ensure!(edited.chat_id == chat_id && edited.id == sent.id, "text edit returned a different message");
  expect_text(&edited, "td-client live test edited")
}

async fn wait_for_send_success(session: &mut Session, chat_id: i64, message_id: i64) -> Result<()> {
  loop {
    let Some(update) = session.recv().await else { bail!("client closed before the terminal send update") };
    if let Update::updateMessageSendSucceeded(update) = update
      && update.message.chat_id == chat_id
      && update.message.id == message_id
    {
      return Ok(());
    }
  }
}

async fn document_progress(client: &Client, chat_id: i64, root: &Path, messages: &mut Vec<i64>) -> Result<i64> {
  let path = root.join("progress.bin");
  create_file(&path, 16 * 1024 * 1024)?;
  let request = fns::sendMessage { chat_id, input_message_content: document_content(&path), ..Default::default() };
  let mut progress = ProgressLog::default();
  let mut invalid_index = false;
  let message = {
    let mut observe = |index, value| {
      invalid_index |= index != 0;
      progress.observe(value);
    };
    client.track(&request, None, Some(&mut observe)).await.context("document send failed")?
  };
  messages.push(message.id);
  ensure!(!invalid_index, "single send reported a nonzero item index");
  ensure!(progress.partial, "document reported no in-flight progress");
  let MessageContent::messageDocument(content) = &message.content else { bail!("document send returned {:?}", message.content) };
  let file = &content.document.document;
  expect_uploaded(file)?;
  download_document(client, file.id, file.size).await?;
  Ok(message.id)
}

async fn edit_document(client: &Client, chat_id: i64, message_id: i64, root: &Path) -> Result<()> {
  let path = root.join("edited.bin");
  create_file(&path, 4 * 1024 * 1024)?;
  let input_message_content = document_content(&path);
  let request = fns::editMessageMedia { chat_id, message_id, input_message_content, ..Default::default() };
  let Message::message(message) = client.send(&request).await.context("media edit failed")?;
  ensure!(message.chat_id == chat_id && message.id == message_id, "media edit returned a different message");
  ensure!(message.sending_state.is_none(), "media edit returned before upload completion");
  let MessageContent::messageDocument(content) = &message.content else { bail!("media edit didn't return a document") };
  expect_uploaded(&content.document.document)
}

async fn album_progress(client: &Client, chat_id: i64, root: &Path, sent: &mut Vec<i64>) -> Result<()> {
  let contents = photos(root).await?;
  let request = fns::sendMessageAlbum { chat_id, input_message_contents: contents, ..Default::default() };
  let mut progress: [ProgressLog; PHOTO_COUNT] = array::from_fn(|_| ProgressLog::default());
  let mut unexpected_index = None;
  let results = {
    let mut observe = |index: usize, value: Progress| match progress.get_mut(index) {
      Some(progress) => progress.observe(value),
      None => unexpected_index = Some(index),
    };
    client.track_all(&request, None, Some(&mut observe)).await.context("album request failed")?
  };

  ensure!(results.len() == PHOTO_COUNT, "album returned {} messages", results.len());
  sent.extend(results.iter().filter_map(|result| result.as_ref().ok().map(|message| message.id)));
  for (index, result) in results.into_iter().enumerate() {
    let message = result.with_context(|| format!("album item {index} failed"))?;
    let MessageContent::messagePhoto(content) = &message.content else { bail!("album item {index} isn't a photo") };
    let [.., largest] = content.photo.sizes.as_slice() else { bail!("album item {index} has no photo sizes") };
    expect_uploaded(&largest.photo)?;
  }
  if let Some(index) = unexpected_index {
    bail!("progress reported out-of-range album index {index}");
  }
  ensure!(progress.iter().any(|progress| progress.partial), "album reported no in-flight progress");
  Ok(())
}

async fn cancel_document(client: &Client, chat_id: i64, root: &Path, sent: &mut Vec<i64>) -> Result<()> {
  let path = root.join("cancel.bin");
  create_file(&path, 64 * 1024 * 1024)?;
  let request = fns::sendMessage { chat_id, input_message_content: document_content(&path), ..Default::default() };
  let cancel = CancellationToken::new();
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

fn create_file(path: &Path, size: u64) -> Result<()> {
  let mut file = File::create(path).context("failed to create a live-test document")?;
  write!(file, "{:?}", SystemTime::now())?;
  file.set_len(size)?;
  Ok(())
}

async fn delete_messages(client: &Client, chat_id: i64, message_ids: &[i64]) -> Result<()> {
  if message_ids.is_empty() {
    return Ok(());
  }
  let request = fns::deleteMessages { chat_id, message_ids: message_ids.into(), revoke: true };
  client.send(&request).await.context("failed to delete live-test messages")?;
  Ok(())
}

fn text_content(text: &str) -> InputMessageContent {
  types::inputMessageText { text: formatted(text), ..Default::default() }.into()
}

fn document_content(path: &Path) -> InputMessageContent {
  let document = types::inputDocument { document: local_file(path), disable_content_type_detection: true, ..Default::default() };
  types::inputMessageDocument { document, caption: Some(formatted("td-client live test")) }.into()
}

fn formatted(text: &str) -> types::formattedText {
  types::formattedText { text: text.into(), ..Default::default() }
}

fn local_file(path: &Path) -> InputFile {
  types::inputFileLocal { path: path.to_string_lossy().into_owned() }.into()
}

fn expect_text(message: &types::message, expected: &str) -> Result<()> {
  let MessageContent::messageText(content) = &message.content else { bail!("expected a text message") };
  ensure!(content.text.text == expected, "message text is {:?} instead of {expected:?}", content.text.text);
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

async fn photos(root: &Path) -> Result<Vec<InputMessageContent>> {
  let mut contents = Vec::with_capacity(PHOTO_COUNT);
  for index in 0..PHOTO_COUNT {
    let path = root.join(format!("{index}.jpg"));
    let hue = format!("hue=h={}", index * 30);
    let status = Command::new("ffmpeg")
      .args(["-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=2048x2048:rate=1:duration=1"])
      .args(["-vf", hue.as_str(), "-frames:v", "1"])
      .arg(&path)
      .status()
      .await
      .context("failed to run FFmpeg")?;
    ensure!(status.success(), "FFmpeg failed for photo {index}");
    let photo = types::inputPhoto { photo: local_file(&path), width: 2048, height: 2048, ..Default::default() };
    let caption = Some(formatted("td-client live test"));
    contents.push(types::inputMessagePhoto { photo, caption, ..Default::default() }.into());
  }
  Ok(contents)
}

fn read_config() -> Result<Config> {
  let bytes = fs::read(CONFIG).context("missing live-test config")?;
  serde_json::from_slice(&bytes).context("invalid live-test config")
}

fn temporary_directory() -> Result<PathBuf> {
  let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
  Ok(env::temp_dir().join(format!("td-client-live-{}-{nonce}", process::id())))
}

async fn download_document(client: &Client, file_id: i32, size: i64) -> Result<()> {
  eprintln!("testing full and ranged synchronous download accounting");
  for (offset, limit, total) in [(0, 0, size), (4096, 8192, 8192)] {
    let request = fns::downloadFile { file_id, offset, limit, synchronous: true, priority: 1 };
    let mut invalid_sample = false;
    let file = {
      let mut observe = |index, next: Progress| {
        invalid_sample |= index != 0 || next.current < 0 || next.current > total;
      };
      client.download(&request, None, Some(&mut observe)).await.context("download failed")?
    };
    ensure!(file.id == file_id, "download returned a different file");
    ensure!(!invalid_sample, "download progress exceeded its requested range");
    let local = file.local;
    ensure!(local.download_offset <= offset, "download skipped the requested prefix");
    ensure!(local.download_offset + local.downloaded_prefix_size >= offset + total, "download returned an incomplete range");
  }
  Ok(())
}
