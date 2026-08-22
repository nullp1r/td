use std::fmt;

use td_types::enums::MessageSender;
use td_types::types;

use super::App;
use crate::client_ext::ClientExt;
use crate::util;

impl App {
  #[tracing::instrument(skip(self, sender), fields(cmd, args))]
  pub(super) async fn handle_command(&self, chat_id: i64, id: i64, sender: &MessageSender, cmd: &str, args: &str) -> td_client::Result {
    tracing::info!(%cmd, %args, "executing command");

    match cmd {
      "ping" => self.cmd_ping(chat_id, id).await,
      "ratings" | "top" | "leaderboard" => self.cmd_ratings(chat_id, id).await,
      "rating" | "my_rating" => self.cmd_my_rating(chat_id, id, sender).await,
      "me" => self.cmd_me(chat_id, id).await,
      "help" => self.cmd_help(chat_id, id).await,
      _ => Ok(()),
    }
  }

  async fn cmd_ping(&self, chat_id: i64, id: i64) -> td_client::Result {
    self.client.reply_text(chat_id, id, "🏓 Pong!").await?;
    Ok(())
  }

  async fn cmd_help(&self, chat_id: i64, id: i64) -> td_client::Result {
    self.client.reply_text(chat_id, id, "💡 Commands: `/ratings`, `/rating`, `/ping`, `/help` | Reply with `+` or `-` to rate users").await?;
    Ok(())
  }

  async fn cmd_ratings(&self, chat_id: i64, id: i64) -> td_client::Result {
    let top = self.db.top_ratings(chat_id);

    if top.is_empty() {
      self.client.reply_text(chat_id, id, "📊 No ratings recorded yet! Reply with `+` or `-` to rate chat members.").await?;
      return Ok(());
    }

    let leaderboard = fmt::from_fn(|f| {
      writeln!(f, "🏆 **Top Chat Members**:")?;
      for (rank, (uid, score)) in top.iter().take(20).enumerate() {
        writeln!(f, "{}. User `{uid}`: **{score:+}**", rank + 1)?;
      }
      Ok(())
    });

    self.client.reply_text(chat_id, id, leaderboard.to_string()).await?;
    Ok(())
  }

  async fn cmd_my_rating(&self, chat_id: i64, id: i64, sender: &MessageSender) -> td_client::Result {
    let Some(user_id) = util::extract_user_id(sender) else {
      self.client.reply_text(chat_id, id, "⚠️ Ratings are only available for individual users.").await?;
      return Ok(());
    };

    let score = self.db.get_rating(chat_id, user_id);
    self.client.reply_text(chat_id, id, format!("👤 Your rating in this chat: **{score:+}**")).await?;
    Ok(())
  }

  async fn cmd_me(&self, chat_id: i64, id: i64) -> td_client::Result {
    let types::user { id: user_id, first_name, usernames, .. } = self.client.get_me().await?;

    let username = util::primary_username(usernames.as_ref());
    let info = fmt::from_fn(|f| {
      write!(f, "🤖 {first_name}")?;
      if let Some(name) = username {
        write!(f, " (@{name})")?;
      }
      write!(f, " (ID: {user_id})")
    });

    self.client.reply_text(chat_id, id, info.to_string()).await?;
    Ok(())
  }
}
