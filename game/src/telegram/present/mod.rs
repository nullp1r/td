//! Player-facing Telegram presentation and shared Rich Message transport helpers.

mod core;
mod fishing;
mod progression;
mod sharing;
mod social;
mod world;

pub use core::{home, home_edit, inventory};
pub use fishing::{bite, casting, caught, escaped, explored, relic, struggle};
pub use progression::{crafted, crafting, help, help_send, npc, reward, shop, sold, tasks, titles};
pub use sharing::inline_catches;
pub use social::{group_caught, group_fish, group_help, group_help_send, group_journal, group_read, group_records, group_result};
pub use world::{conditions, journal, locations, records};

use tdx::{client, prelude::*};

use crate::{fishing::Reaction, telegram::callback::Callback};

pub fn share_target() -> enums::TargetChat {
  types::targetChatChosen {
    types: types::targetChatTypes { allow_user_chats: true, allow_bot_chats: false, allow_group_chats: true, allow_channel_chats: true },
  }
  .into()
}

async fn send_ephemeral(client: &Client, update: &types::updateNewCallbackQuery, content: types::inputMessageRichMessage) -> client::Result<()> {
  client.send(&send::ephemeral(update, content)).await?;
  Ok(())
}

async fn edit_message(client: &Client, chat_id: i64, message_id: i64, content: types::inputMessageRichMessage) -> client::Result<()> {
  client.send(&edit::rich(&(chat_id, message_id), content)).await?;
  Ok(())
}

/// Edits one private game panel while keeping transport boilerplate out of the presenters.
async fn edit_panel(
  client: &Client,
  chat_id: i64,
  message_id: i64,
  content: types::inputMessageRichMessage,
  reply_markup: enums::ReplyMarkup,
) -> client::Result<()> {
  let target = (chat_id, message_id);
  let mut request = edit::rich(&target, content);
  request.reply_markup = Some(reply_markup);
  client.send(&request).await?;
  Ok(())
}

pub async fn app_error(client: &Client, chat_id: i64, message_id: i64, text: &str, shop_recovery: bool) -> client::Result<()> {
  let reply_markup = if shop_recovery {
    markup::inline(vec![
      vec![markup::callback("🎒 Inventory", Callback::Inventory.encode()), markup::callback("🏪 Shop", Callback::Shop.encode())],
      vec![markup::callback("🏠 Home", Callback::Home.encode())],
    ])
  } else {
    back_home_markup()
  };
  edit_panel(client, chat_id, message_id, rich([heading("Can't do that", 2), paragraph(text)]), reply_markup).await
}

fn back_home_markup() -> enums::ReplyMarkup {
  markup::inline([[markup::callback("🏠 Home", Callback::Home.encode())]])
}

const fn reaction_label(reaction: Reaction) -> &'static str {
  match reaction {
    Reaction::Excellent => "Excellent",
    Reaction::Good => "Good",
    Reaction::Late => "Late",
    Reaction::Missed => "Missed",
  }
}

fn format_duration(ms: i64) -> String {
  let seconds = ms.max(0) / 1_000;
  if seconds < 60 {
    format!("{}s", seconds.max(1))
  } else {
    let minutes = seconds / 60;
    let remainder = seconds % 60;
    if remainder == 0 { format!("{minutes}m") } else { format!("{minutes}m {remainder}s") }
  }
}

fn format_game_time(game_minute: u16) -> String {
  format!("{:02}:{:02}", game_minute / 60, game_minute % 60)
}

fn format_weight_u64(weight_g: u64) -> String {
  if weight_g < 1_000 { format!("{weight_g} g") } else { format!("{}.{:02} kg", weight_g / 1_000, (weight_g % 1_000) / 10) }
}

fn format_weight(weight_g: u32) -> String {
  if weight_g < 1_000 { format!("{weight_g} g") } else { format!("{:.2} kg", f64::from(weight_g) / 1_000.0) }
}
