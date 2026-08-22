use std::borrow::Cow;

use tokio::signal;

use td_client::Client;
use td_types::enums::{ChatAction, InputFile, Message, MessageContent, MessageSender, Update, UserStatus};
use td_types::types;

use crate::client_ext::ClientExt;
use crate::db::Database;
use crate::util;

mod commands;
mod inline;

pub(super) struct App {
  client: Client,
  db: Database,
}

impl App {
  pub(super) fn new(client: Client) -> Self {
    Self { client, db: Database::default() }
  }

  pub(super) async fn run(mut self) -> td_client::Result {
    tracing::info!("listening for updates...");

    loop {
      let update = tokio::select! {
        update = self.client.recv() => update,
        _ = signal::ctrl_c() => break,
      };

      let update = match update {
        Ok(Some(update)) => update,
        Ok(None) => break,
        Err(error) => {
          tracing::error!(%error, "update stream failed");
          let _ = self.client.shutdown().await;
          return Err(error);
        }
      };

      if let Err(error) = self.dispatch(update).await {
        tracing::error!(%error, "failed to dispatch update");
      }
    }

    tracing::info!("shutting down");
    self.client.shutdown().await
  }

  async fn dispatch(&mut self, update: Update) -> td_client::Result {
    match update {
      Update::updateNewMessage(u) if !u.message.is_outgoing => self.handle_message(u.message).await?,
      Update::updateNewMessage(_) => tracing::trace!("ignoring outgoing message update"),
      Update::updateNewInlineQuery(u) => self.handle_inline_query(u.id, &u.query).await?,
      Update::updateUserStatus(u) => handle_user_status(u.user_id, &u.status),
      Update::updateChatAction(u) => handle_chat_action(u.chat_id, &u.action),
      Update::updateFile(u) => handle_file_progress(&u.file),
      _ => tracing::trace!("unhandled update"),
    }

    Ok(())
  }

  async fn handle_message(&mut self, msg: types::message) -> td_client::Result {
    let types::message { chat_id, id, sender_id, reply_to, content, .. } = msg;

    // Trigger: for any DM message with media, download and upload back as a document
    if let (Some(file), 0..) = (util::extract_media_file(&content), chat_id) {
      return self.handle_dm_media(chat_id, id, file, &content).await;
    }

    let Some(raw_text) = util::message_text(&content) else {
      tracing::trace!("ignoring non-text message content");
      return Ok(());
    };

    let trimmed = raw_text.trim();

    // 1. Handle unaddressed commands
    if let Some(cmd_line) = trimmed.strip_prefix('/') {
      let (cmd, args) = cmd_line.split_once(char::is_whitespace).unwrap_or((cmd_line, ""));
      if !cmd.is_empty() {
        return self.handle_command(chat_id, id, &sender_id, cmd, args.trim_start()).await;
      }
    }

    // 2. Handle reply reactions (`+`, `-`, `+1`, `-1`, `👍`, `👎`, etc.)
    if let Some(delta) = parse_rating_delta(trimmed)
      && let Some(target_msg_id) = util::reply_message_id(reply_to.as_ref())
    {
      return self.handle_rating_reply(chat_id, id, &sender_id, target_msg_id, delta).await;
    }

    // 3. Conversational patterns
    self.handle_pattern(chat_id, id, trimmed).await
  }

  async fn handle_dm_media(&self, chat_id: i64, id: i64, file: &types::file, content: &MessageContent) -> td_client::Result {
    tracing::info!(file_id = file.id, "downloading media from DM");

    let downloaded = if file.local.is_downloading_completed && !file.local.path.is_empty() {
      Cow::Borrowed(file)
    } else {
      Cow::Owned(self.client.download(file.id, 32).await?)
    };

    if downloaded.local.path.is_empty() {
      tracing::warn!(file_id = downloaded.id, "download completed but local path is empty");
      return Ok(());
    }

    tracing::info!(file_id = downloaded.id, path = %downloaded.local.path, "download completed, starting upload");
    let file = self.client.upload(&downloaded.local.path, 32).await?;

    tracing::info!(file_id = file.id, path = %downloaded.local.path, "upload started, sending back as document");
    let caption = util::message_caption(content).map(Into::into);
    let document = InputFile::inputFileId(types::inputFileId { id: file.id });
    self.client.reply_document(chat_id, id, document, caption).await?;

    Ok(())
  }

  async fn handle_rating_reply(&mut self, chat_id: i64, id: i64, sender: &MessageSender, target_msg_id: i64, delta: i64) -> td_client::Result {
    let Some(from_id) = util::extract_user_id(sender) else {
      tracing::debug!(chat_id, "ignoring rating: sender is not an individual user");
      return Ok(());
    };

    // Fetch replied message only after validating sender is a user
    let Message::message(target_msg) = self.client.get_message(chat_id, target_msg_id).await?;

    let Some(to_id) = util::extract_user_id(&target_msg.sender_id) else {
      tracing::debug!(chat_id, target_msg_id, "ignoring rating: target author is not an individual user");
      return Ok(());
    };

    if from_id == to_id {
      self.client.reply_text(chat_id, id, "⚠️ You cannot change your own rating!").await?;
      return Ok(());
    }

    let new_score = self.db.adjust_rating(chat_id, to_id, delta);

    let [icon, verb] = if let 0.. = delta { ["⭐️", "increased"] } else { ["🔻", "decreased"] };
    let reply = format!("{icon} Rating {verb} for User {to_id}! (Total: {new_score:+})");

    self.client.reply_text(chat_id, id, reply).await?;
    Ok(())
  }

  async fn handle_pattern(&self, chat_id: i64, id: i64, text: &str) -> td_client::Result {
    let (first_word, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));

    let word = first_word.trim_matches(|c: char| !c.is_alphanumeric());
    let is_standalone = rest.is_empty() || rest.trim_matches(|c: char| !c.is_alphanumeric()).is_empty();

    if is_standalone
      && (word.eq_ignore_ascii_case("hello") || word.eq_ignore_ascii_case("hi") || word.eq_ignore_ascii_case("hey") || word.eq_ignore_ascii_case("sup"))
    {
      self.client.reply_text(chat_id, id, "👋 Hello! Reply to messages with `+` or `-` to give karma!").await?;
    }

    Ok(())
  }
}

fn parse_rating_delta(text: &str) -> Option<i64> {
  let trimmed = text.trim();
  match trimmed {
    "+" | "+1" | "👍" | "👍🏻" | "👍🏼" | "👍🏽" | "👍🏾" | "👍🏿" => Some(1),
    "-" | "-1" | "👎" | "👎🏻" | "👎🏼" | "👎🏽" | "👎🏾" | "👎🏿" => Some(-1),
    _ => {
      let w = trimmed.trim_matches(|c: char| !c.is_alphanumeric());
      if w.eq_ignore_ascii_case("thanks")
        || w.eq_ignore_ascii_case("ty")
        || w.eq_ignore_ascii_case("thank you")
        || w.eq_ignore_ascii_case("tysm")
        || w.eq_ignore_ascii_case("thx")
      {
        Some(1)
      } else {
        None
      }
    }
  }
}

fn handle_user_status(user_id: i64, status: &UserStatus) {
  match status {
    UserStatus::userStatusOnline(_) => tracing::info!("User {user_id} online"),
    UserStatus::userStatusOffline(types::userStatusOffline { was_online }) => {
      tracing::info!("User {user_id} offline (last seen {was_online})");
    }
    _ => {}
  }
}

fn handle_chat_action(chat_id: i64, action: &ChatAction) {
  if let ChatAction::chatActionTyping = action {
    tracing::debug!("Typing in {chat_id}");
  }
}

fn handle_file_progress(file: &types::file) {
  match &file.local {
    l if l.is_downloading_active => tracing::debug!(id = file.id, size = l.downloaded_size, "downloading file"),
    l if l.is_downloading_completed => tracing::info!(id = file.id, path = %l.path, "download completed"),
    _ => {}
  }
}
