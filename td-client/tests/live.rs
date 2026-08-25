use std::env;
use std::fs::{self, File};
use std::path::Path;
use std::process;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use tokio::process::Command;
use tokio::time::timeout;

use td_client::{Client, Error, Sender};
use td_types::enums::{AuthorizationState, Chat, InputFile, InputMessageContent, Message, MessageContent, Update};
use td_types::{fns, types};

const TEST_TIMEOUT: Duration = Duration::from_secs(30);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const SESSION_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/live/session");

#[derive(Deserialize)]
struct Config {
  api_id: i32,
  api_hash: String,
  bot_token: String,
  chat_id: i64,
}

fn read_config() -> Result<Config> {
  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/live/config.json");
  let bytes = fs::read(path).context("missing `config.json` (copy from `config.example.json`)")?;
  serde_json::from_slice(&bytes).context("failed to parse `config.json`")
}

#[tokio::test]
#[ignore = "requires tests/live/config.json and a real Telegram test chat"]
async fn telegram_message_lifecycle() -> Result<()> {
  tracing_subscriber::fmt().with_test_writer().with_target(false).init();
  let config = read_config()?;
  let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
  let root = env::temp_dir().join(format!("td-client-live-{}-{nonce}", process::id()));
  let marker = format!("td-client-live-{nonce}");
  fs::create_dir_all(&root).context("failed to create the isolated live-test directory")?;

  // The generated update types make this future large enough to overflow libtest's worker stack
  // before its first poll unless it is heap-allocated here.
  let result = Box::pin(run(config, &root, &marker)).await;
  let cleanup = fs::remove_dir_all(&root).context("failed to remove the isolated live-test directory");
  result?;
  cleanup
}

async fn run(config: Config, root: &Path, marker: &str) -> Result<()> {
  let Config { api_id, api_hash, bot_token, chat_id } = config;
  ensure!(chat_id != 0, "chat_id must identify the dedicated Telegram test chat");

  td_client::set_log_level(0);
  td_client::set_receive_timeout(Duration::from_millis(50));
  let params = fns::setTdlibParameters {
    api_id,
    api_hash,
    database_directory: SESSION_DIRECTORY.into(),
    files_directory: format!("{}/files", root.display()),
    ..td_client::defaults()
  };
  tracing::info!("creating an isolated TDLib client");
  let mut client = Client::new(params).await.context("failed to create the live-test client")?;

  let result = timeout(TEST_TIMEOUT, async {
    authenticate_bot(&mut client, &bot_token).await.context("failed to authenticate the live-test bot")?;
    discover_chat(&client.sender(), chat_id).await?;
    exercise(&mut client, chat_id, root, marker).await
  });
  let result = match result.await {
    Ok(result) => result,
    Err(error) => Err(anyhow::Error::new(error).context("live Telegram test exceeded 30 seconds")),
  };
  tracing::info!("shutting down the TDLib client");
  let shutdown = match timeout(SHUTDOWN_TIMEOUT, client.shutdown()).await {
    Ok(result) => result.map_err(Into::into),
    Err(error) => Err(anyhow::Error::new(error).context("live-test client shutdown timed out")),
  };
  result?;
  shutdown
}

async fn authenticate_bot(client: &mut Client, token: &str) -> td_client::Result {
  tracing::info!("authenticating the bot");
  let sender = client.sender();
  loop {
    match client.recv_auth().await? {
      AuthorizationState::authorizationStateWaitTdlibParameters => {}
      AuthorizationState::authorizationStateWaitPhoneNumber => {
        sender.send(&fns::checkAuthenticationBotToken { token: token.into() }).await?;
      }
      AuthorizationState::authorizationStateReady => {
        tracing::info!("bot authenticated");
        return Ok(());
      }
      state => return Err(Error::Auth(state)),
    }
  }
}

async fn discover_chat(sender: &Sender, chat_id: i64) -> Result<()> {
  tracing::info!("asking TDLib to discover the configured chat");
  let response = sender.send(&fns::getChat { chat_id }).await.context("failed to discover the configured chat")?;
  let Chat::chat(chat) = response;
  ensure!(chat.id == chat_id, "getChat returned chat {} instead of {chat_id}", chat.id);
  tracing::info!("configured chat discovered");
  Ok(())
}

async fn exercise(client: &mut Client, chat_id: i64, root: &Path, marker: &str) -> Result<()> {
  text_message_lifecycle(client, chat_id, marker).await?;
  media_uploads(client, chat_id, root, marker).await?;
  media_edit(client, chat_id, root, marker).await?;
  document_deadline(client, chat_id, root, marker).await
}

async fn text_message_lifecycle(client: &mut Client, chat_id: i64, marker: &str) -> Result<()> {
  tracing::info!("sending the text message");
  let sender = client.sender();
  let input_message_content = text_content(marker);
  let request = fns::sendMessage { chat_id, input_message_content, ..Default::default() };
  let sent = sender.send_message(&request).await.context("failed to send the live-test text message")?;
  let message_id = sent.id;
  tracing::info!(message_id, "text message sent");

  let verification = async {
    ensure!(sent.chat_id == chat_id, "send result belongs to chat {} instead of {chat_id}", sent.chat_id);
    ensure!(sent.is_outgoing, "send result isn't outgoing");
    ensure!(sent.sending_state.is_none(), "authoritative send result still has a sending state");
    expect_text(&sent, marker)?;
    wait_for_send_success(client, chat_id, message_id).await?;

    tracing::info!(message_id, "editing the text message");
    let edited_text = format!("{marker} edited");
    let input_message_content = text_content(&edited_text);
    let request = fns::editMessageText { chat_id, message_id, input_message_content, ..Default::default() };
    let response = sender.send(&request).await.context("failed to edit the live-test text message")?;
    let Message::message(edited) = response;
    ensure!(edited.chat_id == chat_id && edited.id == message_id, "edit returned a different message");
    expect_text(&edited, &edited_text)
  }
  .await;

  tracing::info!(message_id, "deleting the text message");
  let deletion = delete_message(&sender, chat_id, message_id).await;
  verification?;
  deletion
}

async fn wait_for_send_success(client: &mut Client, chat_id: i64, message_id: i64) -> Result<()> {
  loop {
    let update = client.recv().await.context("failed while waiting for the terminal send update")?;
    let Some(update) = update else {
      bail!("client closed before the terminal send update");
    };
    if let Update::updateMessageSendSucceeded(update) = update
      && update.message.chat_id == chat_id
      && update.message.id == message_id
    {
      tracing::info!(message_id, "observed the terminal send-success update");
      return Ok(());
    }
  }
}

async fn media_uploads(client: &mut Client, chat_id: i64, root: &Path, marker: &str) -> Result<()> {
  for &MediaCase { kind, extension, mime_type, normal_source, forced_source, output_args } in MEDIA_CASES {
    let name = kind.name();
    let normal_name = format!("{marker}-{name}-normal.{extension}");
    let forced_name = format!("{marker}-{name}-forced.{extension}");
    let normal_path = root.join(&normal_name);
    let forced_path = root.join(&forced_name);
    tracing::info!(media = name, "generating media fixtures with ffmpeg");
    generate_media(&normal_path, normal_source, output_args).await?;
    generate_media(&forced_path, forced_source, output_args).await?;

    let normal = MediaUpload { kind, path: &normal_path, file_name: &normal_name, mime_type, caption: marker };
    upload_media(client, chat_id, normal).await?;
    let kind = MediaKind::Document;
    let forced = MediaUpload { kind, path: &forced_path, file_name: &forced_name, mime_type, caption: marker };
    upload_media(client, chat_id, forced).await?;
  }
  Ok(())
}

async fn upload_media(client: &mut Client, chat_id: i64, upload: MediaUpload<'_>) -> Result<()> {
  let MediaUpload { kind, path, file_name, mime_type, caption } = upload;
  let sender = client.sender();
  let media = kind.name();
  tracing::info!(media, file_name, "uploading media");
  let input_message_content = kind.content(path, caption);
  let request = fns::sendMessage { chat_id, input_message_content, ..Default::default() };
  let message = sender.send_message(&request).await.with_context(|| format!("failed to send {file_name} as {media}"))?;
  let message_id = message.id;
  tracing::info!(message_id, media, "media uploaded");
  let verification = async {
    wait_for_send_success(client, chat_id, message_id).await?;
    kind.expect(&message, file_name, mime_type, caption)
  }
  .await;
  let deletion = delete_message(&sender, chat_id, message_id).await;
  verification?;
  deletion
}

async fn generate_media(path: &Path, source: &str, output_args: &[&str]) -> Result<()> {
  let status = Command::new("ffmpeg")
    .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-f", "lavfi", "-i", source])
    .args(output_args)
    .arg(path)
    .status()
    .await
    .context("failed to run ffmpeg; install it to run the live media tests")?;
  ensure!(status.success(), "ffmpeg exited with {status}");
  Ok(())
}

async fn media_edit(client: &mut Client, chat_id: i64, root: &Path, marker: &str) -> Result<()> {
  let original_name = format!("{marker}-edit-original.mp4");
  let edited_name = format!("{marker}-edit-document.mp4");
  let original_path = root.join(&original_name);
  let edited_path = root.join(&edited_name);
  tracing::info!("generating fresh media-edit fixtures with ffmpeg");
  generate_media(&original_path, "color=c=magenta:s=96x64:d=1", H264_ARGS).await?;
  generate_media(&edited_path, "color=c=cyan:s=96x64:d=1", H264_ARGS).await?;

  let sender = client.sender();
  let input_message_content = MediaKind::Video.content(&original_path, marker);
  let request = fns::sendMessage { chat_id, input_message_content, ..Default::default() };
  let message = sender.send_message(&request).await.context("failed to send the media-edit source")?;
  let message_id = message.id;
  let verification = async {
    wait_for_send_success(client, chat_id, message_id).await?;
    tracing::info!(message_id, "editing video to a freshly uploaded forced document");
    let input_message_content = MediaKind::Document.content(&edited_path, marker);
    let request = fns::editMessageMedia { chat_id, message_id, input_message_content, ..Default::default() };
    let response = sender.send(&request).await.context("failed to edit media through its direct response")?;
    let Message::message(edited) = response;
    ensure!(edited.chat_id == chat_id && edited.id == message_id, "media edit returned a different message");
    ensure!(edited.sending_state.is_none(), "media-edit response still has a sending state");
    MediaKind::Document.expect(&edited, &edited_name, "video/mp4", marker)
  }
  .await;

  tracing::info!(message_id, "deleting the media-edit message");
  let deletion = delete_message(&sender, chat_id, message_id).await;
  verification?;
  deletion
}

async fn document_deadline(client: &mut Client, chat_id: i64, root: &Path, marker: &str) -> Result<()> {
  tracing::info!("sending a document with an already-expired deadline");
  let path = root.join("deadline.bin");
  let file = File::create(&path).context("failed to create the deadline-test document")?;
  file.set_len(8 * 1024 * 1024).context("failed to size the deadline-test document")?;
  drop(file);
  let sender = client.sender();
  let input_message_content = MediaKind::Document.content(&path, marker);
  let request = fns::sendMessage { chat_id, input_message_content, ..Default::default() };
  let result = sender.send_message_until(&request, Instant::now()).await;

  let temporary_message_id = match result {
    Err(Error::MessageDeadline { chat_id: result_chat_id, message_id }) => {
      ensure!(result_chat_id == chat_id, "deadline result belongs to chat {result_chat_id} instead of {chat_id}");
      tracing::info!(message_id, "document send reached its deadline and was deleted");
      message_id
    }
    Ok(message) => {
      delete_message(&sender, chat_id, message.id).await?;
      bail!("the deadline-test document was sent before an already-expired deadline");
    }
    Err(error) => return Err(error).context("deadline-test document failed unexpectedly"),
  };

  expect_missing(&sender, chat_id, temporary_message_id).await?;
  if let Some(final_message_id) = wait_for_deadline_terminal(client, chat_id, temporary_message_id).await? {
    expect_missing(&sender, chat_id, final_message_id).await?;
  }
  Ok(())
}

async fn wait_for_deadline_terminal(client: &mut Client, chat_id: i64, temporary_message_id: i64) -> Result<Option<i64>> {
  loop {
    let update = client.recv().await.context("failed while waiting for the deadline-test terminal update")?;
    let Some(update) = update else {
      bail!("client closed before the deadline-test terminal update");
    };
    match update {
      Update::updateMessageSendSucceeded(update) => {
        let matches_send = update.message.chat_id == chat_id && update.old_message_id == temporary_message_id;
        if matches_send {
          return Ok(Some(update.message.id));
        }
      }
      Update::updateMessageSendFailed(update) => {
        let matches_send = update.message.chat_id == chat_id && update.old_message_id == temporary_message_id;
        if matches_send {
          return Ok(None);
        }
      }
      Update::updateDeleteMessages(update) => {
        let matches_chat = update.chat_id == chat_id;
        let deletes_message = update.message_ids.contains(&temporary_message_id);
        if !update.from_cache && matches_chat && deletes_message {
          return Ok(None);
        }
      }
      _ => {}
    }
  }
}

async fn delete_message(sender: &Sender, chat_id: i64, message_id: i64) -> Result<()> {
  let request = fns::deleteMessages { chat_id, message_ids: vec![message_id], revoke: true };
  sender.send(&request).await.context("failed to delete a live-test message")?;
  expect_missing(sender, chat_id, message_id).await
}

async fn expect_missing(sender: &Sender, chat_id: i64, message_id: i64) -> Result<()> {
  match sender.send(&fns::getMessage { chat_id, message_id }).await {
    Err(Error::Td(types::error { code: 404, .. })) => Ok(()),
    Ok(_) => bail!("message {message_id} in chat {chat_id} still exists after deletion"),
    Err(error) => Err(error).context("getMessage failed with an unexpected error"),
  }
}

fn expect_text(message: &types::message, expected: &str) -> Result<()> {
  let MessageContent::messageText(content) = &message.content else { bail!("expected a text message") };
  ensure!(content.text.text == expected, "message text is {:?} instead of {expected:?}", content.text.text);
  Ok(())
}

fn expect_uploaded(file: &types::file) -> Result<()> {
  let &types::file { size, ref remote, .. } = file;
  let &types::remoteFile { ref id, is_uploading_active, is_uploading_completed, .. } = remote;
  ensure!(size > 0, "uploaded file has no size");
  ensure!(!id.is_empty(), "uploaded file has no remote ID");
  ensure!(!is_uploading_active, "file is still uploading after message-send success");
  ensure!(is_uploading_completed, "file upload isn't complete after message-send success");
  Ok(())
}

fn text_content(text: &str) -> InputMessageContent {
  types::inputMessageText { text: formatted(text), ..Default::default() }.into()
}

fn formatted(text: &str) -> types::formattedText {
  types::formattedText { text: text.into(), ..Default::default() }
}

fn local_file(path: &Path) -> InputFile {
  types::inputFileLocal { path: path.to_string_lossy().into_owned() }.into()
}

#[derive(Clone, Copy)]
enum MediaKind {
  Animation,
  Audio,
  Document,
  Photo,
  Video,
  VideoNote,
  VoiceNote,
}

struct MediaCase {
  kind: MediaKind,
  extension: &'static str,
  mime_type: &'static str,
  normal_source: &'static str,
  forced_source: &'static str,
  output_args: &'static [&'static str],
}

struct MediaUpload<'a> {
  kind: MediaKind,
  path: &'a Path,
  file_name: &'a str,
  mime_type: &'a str,
  caption: &'a str,
}

impl MediaKind {
  const fn name(self) -> &'static str {
    match self {
      Self::Animation => "animation",
      Self::Audio => "audio",
      Self::Document => "document",
      Self::Photo => "photo",
      Self::Video => "video",
      Self::VideoNote => "video-note",
      Self::VoiceNote => "voice-note",
    }
  }

  fn content(self, path: &Path, caption: &str) -> InputMessageContent {
    match self {
      Self::Animation => {
        let animation = local_file(path);
        let animation = types::inputAnimation { animation, duration: 1, width: 96, height: 64, ..Default::default() };
        types::inputMessageAnimation { animation, caption: Some(formatted(caption)), ..Default::default() }.into()
      }
      Self::Audio => {
        let audio = local_file(path);
        let audio = types::inputAudio { audio, duration: 1, ..Default::default() };
        types::inputMessageAudio { audio, caption: Some(formatted(caption)) }.into()
      }
      Self::Document => {
        let document = local_file(path);
        let document = types::inputDocument { document, disable_content_type_detection: true, ..Default::default() };
        types::inputMessageDocument { document, caption: Some(formatted(caption)) }.into()
      }
      Self::Photo => {
        let photo = types::inputPhoto { photo: local_file(path), width: 96, height: 64, ..Default::default() };
        types::inputMessagePhoto { photo, caption: Some(formatted(caption)), ..Default::default() }.into()
      }
      Self::Video => {
        let video = local_file(path);
        let video = types::inputVideo { video, duration: 1, width: 96, height: 64, ..Default::default() };
        types::inputMessageVideo { video, caption: Some(formatted(caption)), ..Default::default() }.into()
      }
      Self::VideoNote => {
        let video_note = local_file(path);
        let video_note = types::inputVideoNote { video_note, duration: 1, length: 96, ..Default::default() };
        types::inputMessageVideoNote { video_note, ..Default::default() }.into()
      }
      Self::VoiceNote => {
        let voice_note = types::inputVoiceNote { voice_note: local_file(path), duration: 1, ..Default::default() };
        types::inputMessageVoiceNote { voice_note, caption: Some(formatted(caption)), ..Default::default() }.into()
      }
    }
  }

  fn expect(self, message: &types::message, file_name: &str, mime_type: &str, caption: &str) -> Result<()> {
    let expected = self.name();
    let actual = &message.content;
    let (file, actual_file_name, actual_mime_type, actual_caption) = match self {
      Self::Animation => {
        let MessageContent::messageAnimation(content) = actual else { bail!("expected {expected}, got {actual:?}") };
        let types::messageAnimation { animation, caption, .. } = content;
        let types::animation { file_name, mime_type, animation, .. } = animation;
        (animation, Some(file_name.as_str()), Some(mime_type.as_str()), Some(caption.text.as_str()))
      }
      Self::Audio => {
        let MessageContent::messageAudio(content) = actual else { bail!("expected {expected}, got {actual:?}") };
        let types::messageAudio { audio, caption } = content;
        let types::audio { file_name, mime_type, audio, .. } = audio;
        (audio, Some(file_name.as_str()), Some(mime_type.as_str()), Some(caption.text.as_str()))
      }
      Self::Document => {
        let MessageContent::messageDocument(content) = actual else { bail!("expected {expected}, got {actual:?}") };
        let types::messageDocument { document, caption } = content;
        let types::document { file_name, mime_type, document, .. } = document;
        (document, Some(file_name.as_str()), Some(mime_type.as_str()), Some(caption.text.as_str()))
      }
      Self::Photo => {
        let MessageContent::messagePhoto(content) = actual else { bail!("expected {expected}, got {actual:?}") };
        let types::messagePhoto { photo, caption, .. } = content;
        let [.., types::photoSize { photo, .. }] = photo.sizes.as_slice() else { bail!("photo message has no sizes") };
        (photo, None, None, Some(caption.text.as_str()))
      }
      Self::Video => {
        let MessageContent::messageVideo(content) = actual else { bail!("expected {expected}, got {actual:?}") };
        let types::messageVideo { video, caption, .. } = content;
        let types::video { file_name, mime_type, video, .. } = video;
        (video, Some(file_name.as_str()), Some(mime_type.as_str()), Some(caption.text.as_str()))
      }
      Self::VideoNote => {
        let MessageContent::messageVideoNote(content) = actual else { bail!("expected {expected}, got {actual:?}") };
        let types::messageVideoNote { video_note, .. } = content;
        let types::videoNote { video, .. } = video_note;
        (video, None, None, None)
      }
      Self::VoiceNote => {
        let MessageContent::messageVoiceNote(content) = actual else { bail!("expected {expected}, got {actual:?}") };
        let types::messageVoiceNote { voice_note, caption, .. } = content;
        let types::voiceNote { mime_type, voice, .. } = voice_note;
        (voice, None, Some(mime_type.as_str()), Some(caption.text.as_str()))
      }
    };
    if let Some(actual) = actual_file_name {
      ensure!(actual == file_name, "{expected} filename is {actual:?} instead of {file_name:?}");
    }
    if let Some(actual) = actual_mime_type {
      ensure!(actual == mime_type, "{expected} MIME type is {actual:?} instead of {mime_type:?}");
    }
    if let Some(actual) = actual_caption {
      ensure!(actual == caption, "{expected} caption is {actual:?} instead of {caption:?}");
    }
    expect_uploaded(file)
  }
}

const H264_ARGS: &[&str] = &["-c:v", "libx264", "-pix_fmt", "yuv420p"];

const MEDIA_CASES: &[MediaCase] = &[
  MediaCase {
    kind: MediaKind::Animation,
    extension: "mp4",
    mime_type: "video/mp4",
    normal_source: "testsrc=size=96x64:rate=10:duration=1",
    forced_source: "testsrc2=size=96x64:rate=10:duration=1",
    output_args: H264_ARGS,
  },
  MediaCase {
    kind: MediaKind::Audio,
    extension: "mp3",
    mime_type: "audio/mpeg",
    normal_source: "sine=frequency=440:duration=1",
    forced_source: "sine=frequency=554:duration=1",
    output_args: &["-c:a", "libmp3lame", "-q:a", "6"],
  },
  MediaCase {
    kind: MediaKind::Photo,
    extension: "jpg",
    mime_type: "image/jpeg",
    normal_source: "color=c=red:s=96x64",
    forced_source: "color=c=blue:s=96x64",
    output_args: &["-frames:v", "1", "-q:v", "2"],
  },
  MediaCase {
    kind: MediaKind::Video,
    extension: "mp4",
    mime_type: "video/mp4",
    normal_source: "color=c=red:s=96x64:d=1",
    forced_source: "color=c=blue:s=96x64:d=1",
    output_args: H264_ARGS,
  },
  MediaCase {
    kind: MediaKind::VideoNote,
    extension: "mp4",
    mime_type: "video/mp4",
    normal_source: "color=c=green:s=96x96:d=1",
    forced_source: "color=c=yellow:s=96x96:d=1",
    output_args: H264_ARGS,
  },
  MediaCase {
    kind: MediaKind::VoiceNote,
    extension: "ogg",
    mime_type: "audio/ogg",
    normal_source: "sine=frequency=660:duration=1",
    forced_source: "sine=frequency=880:duration=1",
    output_args: &["-c:a", "libopus", "-b:a", "24k"],
  },
];
