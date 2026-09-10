//! Public and per-user ephemeral group-shoal presentation.

use tdx::{client, prelude::*};

use crate::{
  telegram::callback::Callback,
  view::{GroupCatchView, GroupEventView, JournalView, RecordsView},
};

use super::{edit_message, format_duration, format_weight, send_ephemeral};

pub async fn group_fish(client: &Client, chat_id: i64, event: &GroupEventView) -> client::Result<types::message> {
  let request = send::message(chat_id, group_content(event, false));
  client.track(&request, None, None).await
}

pub async fn group_help_send(client: &Client, message: &types::message, user_id: i64) -> client::Result<types::message> {
  let request = send::ephemeral_reply(message, user_id, group_help_content());
  client.track(&request, None, None).await
}

pub async fn group_caught(client: &Client, chat_id: i64, message_id: i64, catch: &GroupCatchView) -> client::Result<()> {
  edit_message(client, chat_id, message_id, group_content(&catch.event, catch.global_first)).await
}

pub async fn group_result(client: &Client, update: &types::updateNewCallbackQuery, catch: &GroupCatchView) -> client::Result<()> {
  send_ephemeral(client, update, group_result_content(catch)).await
}

pub async fn group_journal(client: &Client, update: &types::updateNewCallbackQuery, view: &JournalView) -> client::Result<()> {
  send_ephemeral(client, update, super::world::journal_content(view)).await
}

pub async fn group_records(client: &Client, update: &types::updateNewCallbackQuery, view: &RecordsView) -> client::Result<()> {
  send_ephemeral(client, update, super::world::records_content(view)).await
}

pub async fn group_help(client: &Client, update: &types::updateNewCallbackQuery) -> client::Result<()> {
  send_ephemeral(client, update, group_help_content()).await
}

fn group_content(event: &GroupEventView, world_first: bool) -> types::inputMessageRichMessage {
  let mut blocks = vec![
    heading("🎣 Fishing in this chat", 1),
    paragraph(("This shared shoal features ", bold(event.species_name.as_str()), ". Everyone gets one free cast during its window.")),
    paragraph(italic("Your rod, bait, and coins are untouched by this shared cast.")),
    table()
      .row(("🐟 Shoal", event.species_name.as_str()))
      .row(("👥 Casts", format_args!("{} this shoal", event.participants)))
      .row(("⏳ Rotation", relative_time(format_duration(event.resets_in_ms), event.ends_at_unix)))
      .compact()
      .into(),
  ];
  if world_first {
    blocks.push(block_quote([paragraph("🌍 This chat just recorded the first known catch of this species in Rustwater.")]));
  }
  blocks.push(button_row([
    success_callback_button("🎣 Cast once", Callback::GroupCast { cycle: event.cycle }.encode()),
    callback_button("ℹ️ How it works", Callback::GroupHelp.encode()),
  ]));
  rich(blocks)
}

fn group_result_content(catch: &GroupCatchView) -> types::inputMessageRichMessage {
  let mut blocks = vec![
    heading("🎉 Your catch", 1),
    paragraph(bold(format_args!("{} · {} · {:.1} cm", catch.species_name, format_weight(catch.weight_g), f64::from(catch.length_mm) / 10.0))),
    table().row(("✨ XP", format_args!("+{}", catch.xp_gained))).row(("⭐ Level", catch.level)).compact().into(),
  ];
  if catch.new_species {
    blocks.push(paragraph("📖 New species — added to your Journal."));
  }
  if catch.global_first {
    blocks.push(paragraph("🌍 World first — nobody had recorded this species before."));
  }
  blocks.push(paragraph("🎒 Added to your Inventory."));
  blocks.push(paragraph(italic("Only you can see this result.")));
  blocks.push(button_row([callback_button("📖 Journal", Callback::GroupJournal.encode()), callback_button("🏆 Records", Callback::GroupRecords.encode())]));
  rich(blocks)
}

fn group_help_content() -> types::inputMessageRichMessage {
  rich([
    heading("🌊 Fishing together in Rustwater", 1),
    paragraph(
      "Rustwater is the harbor region where your adventure begins. Your character, discoveries, catches, and gear follow you between private chat and group activities.",
    ),
    paragraph("Use /fish to start the current shared shoal. Everyone gets one free cast before it moves on."),
    paragraph("Your result appears only to you here in the group, and the catch is saved to the same character you play in private chat."),
    paragraph("Shared casts do not spend bait or coins, and they do not change your equipped rod."),
    paragraph("After a catch, you can check your Journal and Records without leaving the group."),
    paragraph(italic("Routine results stay private so the chat does not fill with bot messages.")),
  ])
}
