//! Application core state, event router, and rating system.

mod commands;
mod inline;

use std::any::type_name_of_val;
use std::sync::{Arc, RwLock};

use td_client::{ClientHandle, Error as ClientError, UpdateReceiver};
use td_types::{enums, fns, types};
use tokio::time::sleep;

use crate::db::Database;
use crate::util;

#[derive(Clone)]
pub struct App {
  client: ClientHandle,
  db: Arc<RwLock<Database>>,
}

impl App {
  #[must_use]
  pub fn new(client: ClientHandle) -> Self {
    Self { client, db: Arc::new(RwLock::new(Database::new())) }
  }

  pub async fn run(&self, mut updates: UpdateReceiver) -> Result<(), ClientError> {
    tracing::info!("listening for updates...");

    while let Some(update) = updates.recv().await {
      if let Err(err) = self.dispatch(update).await {
        if let Some(wait) = err.flood_wait() {
          tracing::warn!(?wait, "hit Telegram FLOOD_WAIT, backing off");
          sleep(wait).await;
        } else {
          tracing::error!(error = %err, "failed to dispatch update");
        }
      }
    }

    tracing::info!("update stream closed, shutting down");
    Ok(())
  }

  #[tracing::instrument(skip(self, update))]
  pub async fn dispatch(&self, update: enums::Update) -> Result<(), ClientError> {
    match update {
      enums::Update::updateNewMessage(u) if !u.message.is_outgoing => self.handle_message(u.message).await?,
      enums::Update::updateNewMessage(_) => tracing::trace!("ignoring outgoing message update"),
      enums::Update::updateNewInlineQuery(u) => self.handle_inline_query(u.id, &u.query).await?,
      enums::Update::updateUserStatus(u) => handle_user_status(u.user_id, &u.status),
      enums::Update::updateChatAction(u) => handle_chat_action(u.chat_id, &u.action),
      enums::Update::updateFile(u) => handle_file_progress(&u.file),
      other => tracing::trace!(update_type = type_name_of_val(&other), "unhandled update"),
    }

    Ok(())
  }

  #[tracing::instrument(skip(self, msg), fields(chat_id = msg.chat_id, msg_id = msg.id))]
  async fn handle_message(&self, msg: types::message) -> Result<(), ClientError> {
    let types::message { chat_id, id, sender_id, reply_to, content, .. } = msg;
    let Some(raw_text) = util::message_text(&content) else {
      tracing::trace!("ignoring non-text message content");
      return Ok(());
    };

    let trimmed = raw_text.trim();

    // 1. Handle commands: `/cmd@bot args` (routed to commands submodule)
    if let Some(cmd_line) = trimmed.strip_prefix('/') {
      let (raw_cmd, args) = cmd_line.split_once(char::is_whitespace).unwrap_or((cmd_line, ""));
      let (cmd, _) = raw_cmd.split_once('@').unwrap_or((raw_cmd, ""));
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

  async fn handle_rating_reply(&self, chat_id: i64, id: i64, sender: &enums::MessageSender, target_msg_id: i64, delta: i64) -> Result<(), ClientError> {
    let Some(from_id) = util::extract_user_id(sender) else {
      tracing::debug!(chat_id, "ignoring rating: sender is not an individual user");
      return Ok(());
    };

    // Fetch replied message only after validating sender is a user
    let req = fns::getMessage { chat_id, message_id: target_msg_id };
    let Ok(enums::Message::message(target_msg)) = self.client.execute(&req).await else {
      tracing::debug!(chat_id, target_msg_id, "replied message not found or inaccessible");
      return Ok(());
    };

    let Some(to_id) = util::extract_user_id(&target_msg.sender_id) else {
      tracing::debug!(chat_id, target_msg_id, "ignoring rating: target author is not an individual user");
      return Ok(());
    };

    if from_id == to_id {
      self.client.reply_text(chat_id, id, "⚠️ You cannot change your own rating!").await?;
      return Ok(());
    }

    let new_score = self.db.write().map_or(0, |mut db| db.adjust_rating(chat_id, to_id, delta));

    let [icon, verb] = if let 0.. = delta { ["⭐️", "increased"] } else { ["🔻", "decreased"] };
    let reply = format!("{icon} Rating {verb} for User {to_id}! (Total: {new_score:+})");

    self.client.reply_text(chat_id, id, reply).await?;
    Ok(())
  }

  async fn handle_pattern(&self, chat_id: i64, id: i64, text: &str) -> Result<(), ClientError> {
    let (first_word, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));

    if first_word.eq_ignore_ascii_case("!roll") {
      let limit = rest.trim().parse::<u32>().unwrap_or(100).max(1);
      self.client.reply_text(chat_id, id, format!("🎲 42 (1-{limit})")).await?;
      return Ok(());
    }

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
    "+" | "++" | "+1" | "+ 1" | "👍" | "👍🏻" | "👍🏼" | "👍🏽" | "👍🏾" | "👍🏿" => Some(1),
    "-" | "--" | "-1" | "- 1" | "👎" | "👎🏻" | "👎🏼" | "👎🏽" | "👎🏾" | "👎🏿" => Some(-1),
    _ => {
      let cleaned = trimmed.trim_matches(|c: char| !c.is_alphanumeric());
      match cleaned {
        w if w.eq_ignore_ascii_case("thanks")
          || w.eq_ignore_ascii_case("ty")
          || w.eq_ignore_ascii_case("thank you")
          || w.eq_ignore_ascii_case("tysm")
          || w.eq_ignore_ascii_case("thx") =>
        {
          Some(1)
        }
        _ => None,
      }
    }
  }
}

fn handle_user_status(user_id: i64, status: &enums::UserStatus) {
  match status {
    enums::UserStatus::userStatusOnline(_) => tracing::info!("User {user_id} online"),
    enums::UserStatus::userStatusOffline(types::userStatusOffline { was_online }) => {
      tracing::info!("User {user_id} offline (last seen {was_online})");
    }
    _ => {}
  }
}

fn handle_chat_action(chat_id: i64, action: &enums::ChatAction) {
  if matches!(action, enums::ChatAction::chatActionTyping) {
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
