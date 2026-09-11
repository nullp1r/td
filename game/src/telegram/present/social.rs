//! Public and per-user ephemeral group-shoal presentation.

use tdx::{client, prelude::*};

use crate::{
  telegram::callback::Callback,
  view::{GroupApproach, GroupCatchView, GroupEventView, JournalView, RecordsView},
};

use super::{edit_message, format_duration, format_weight, send_ephemeral, share_target};

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

pub async fn group_read(client: &Client, update: &types::updateNewCallbackQuery, event: &GroupEventView) -> client::Result<()> {
  send_ephemeral(client, update, group_read_content(event)).await
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
  let subject = event.species_name.as_deref().unwrap_or("unidentified shoal");
  let mut blocks = vec![
    heading("🌊 Something is moving here", 1),
    paragraph(("This chat has disturbed an ", bold(subject), ". The useful part is not pressing first — it is reading what the water is doing.")),
    pull_quote(event.clue),
    table()
      .row(("Chat standing", format_args!("{} · {} catches", event.standing_name, event.total_catches)))
      .row(("This shoal", format_args!("{} reads", event.participants)))
      .row(("Rotation", relative_time(format_duration(event.resets_in_ms), event.ends_at_unix)))
      .compact()
      .into(),
  ];
  if let Some(hint) = event.memory_hint {
    blocks.push(block_quote([paragraph((bold("Collective memory — "), hint))]));
  }
  if let Some(approach) = event.mastered_approach {
    blocks.push(paragraph(("This chat knows this pattern: ", bold(approach.label()), ".")));
  }
  if world_first {
    blocks.push(block_quote([paragraph("🌍 This chat just recorded the first known catch of this species in Rustwater.")]));
  }
  blocks.push(button_row([
    primary_callback_button("👁 Read the water", Callback::GroupCast { cycle: event.cycle }.encode()),
    callback_button("ℹ️ How it works", Callback::GroupHelp.encode()),
  ]));
  rich(blocks)
}

fn group_read_content(event: &GroupEventView) -> types::inputMessageRichMessage {
  let mut blocks = vec![heading("👁 Read the water", 1), pull_quote(event.clue)];
  if let Some(species) = &event.species_name {
    blocks.push(paragraph(("This chat recognizes the pattern as ", bold(species.as_str()), ".")));
  }
  if let Some(hint) = event.memory_hint {
    blocks.push(block_quote([paragraph((bold("What this chat remembers — "), hint))]));
  }
  if let Some(approach) = event.mastered_approach {
    blocks.push(paragraph(("The mastered read is ", bold(approach.label()), ", but you still choose how to commit.")));
  } else {
    blocks.push(paragraph("Choose one read. A good read biases the catch toward the stronger specimen; a bad read can still land something."));
  }
  blocks.push(button_row([
    callback_button("〰️ Let it drift", Callback::GroupApproach { cycle: event.cycle, approach: GroupApproach::Drift }.encode()),
    primary_callback_button("⚓ Hold steady", Callback::GroupApproach { cycle: event.cycle, approach: GroupApproach::Hold }.encode()),
  ]));
  rich(blocks)
}

fn group_result_content(catch: &GroupCatchView) -> types::inputMessageRichMessage {
  let mut blocks = vec![
    heading(if catch.read_correct { "🎯 Good read" } else { "🌫 You misread it" }, 1),
    paragraph(bold(format_args!("{} · {} · {:.1} cm", catch.species_name, format_weight(catch.weight_g), f64::from(catch.length_mm) / 10.0))),
    paragraph(if catch.read_correct {
      "Your approach matched the behavior. You got the better opening this shoal offered."
    } else {
      "The shoal moved against your read. You still landed a specimen, but not the stronger opening."
    }),
    table().row(("Read", catch.approach.label())).row(("XP", format_args!("+{}", catch.xp_gained))).row(("Level", catch.level)).compact().into(),
  ];
  if catch.new_species {
    blocks.push(paragraph("📖 New species — added to your Journal."));
  }
  if catch.personal_best {
    blocks.push(paragraph("● Personal best for this species."));
  }
  if catch.world_best {
    blocks.push(paragraph("🌍 World record specimen."));
  } else if catch.global_first {
    blocks.push(paragraph("🌍 World first — nobody had recorded this species before."));
  }
  blocks.push(paragraph(italic("The specimen is now part of the same character and catch history you use in private chat.")));
  blocks.push(button_row([switch_inline_button("↗ Share this catch", format!("catch:{}", catch.item_id), share_target())]));
  blocks.push(button_row([callback_button("📖 Journal", Callback::GroupJournal.encode()), callback_button("🏆 Records", Callback::GroupRecords.encode())]));
  rich(blocks)
}

fn group_help_content() -> types::inputMessageRichMessage {
  rich([
    heading("🌊 Reading a shared shoal", 1),
    paragraph("Use /fish to expose the current chat-local shoal. Everyone sees the same clue, then makes one private read during that rotation."),
    paragraph("Your choice changes the quality of the specimen you are most likely to land. It does not spend your private bait, coins, or rod condition."),
    paragraph(
      "As people catch fish in this chat, the chat itself becomes familiar with these patterns: first hints, then identification, then a mastered read.",
    ),
    paragraph("Routine choices and results stay ephemeral. The public card only carries shared state and occasional exceptional consequences."),
    paragraph(italic("Your catches, discoveries, XP, and records still belong to your persistent Rustwater character.")),
  ])
}
