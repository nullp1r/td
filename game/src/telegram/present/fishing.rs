//! Private fishing encounter/result presentation.

use tdx::{client, prelude::*};

use crate::{
  telegram::callback::Callback,
  view::{BiteView, CatchView, CharacterView, EscapeView, ExploreView, RelicView, StruggleView},
};

use super::{edit_message, edit_panel, format_weight, reaction_label};

pub async fn casting(client: &Client, chat_id: i64, message_id: i64, character: &CharacterView) -> client::Result<()> {
  // Build formatted content before `.await`: `fmt::Arguments` is intentionally
  // non-`Send`, while callback handlers run in spawned `Send` tasks.
  let content = rich([
    heading("🎣 Line in the water", 1),
    paragraph(format_args!("The line settles off {}.", character.location.name)),
    paragraph(format_args!("{} remaining: {}", character.bait_name, character.bait_left)),
    paragraph(italic("Watch the float…")),
    button_row([danger_callback_button("✂️ Cut line", Callback::CancelFishing.encode())]),
  ]);
  edit_message(client, chat_id, message_id, content).await
}

pub async fn bite(client: &Client, bite: &BiteView) -> client::Result<()> {
  edit_message(
    client,
    bite.chat_id,
    bite.message_id,
    rich([
      heading("⚡ Bite!", 1),
      paragraph(italic(bite.observation)),
      paragraph("Watch the movement, then reel when it feels right."),
      button_row([primary_callback_button("🎣 Reel", Callback::Reel { encounter_id: bite.encounter_id, step: bite.step }.encode())]),
      button_row([danger_callback_button("✂️ Cut line", Callback::CancelFishing.encode())]),
    ]),
  )
  .await
}

pub async fn caught(client: &Client, catch: &CatchView) -> client::Result<()> {
  let mut blocks = vec![
    heading(if catch.new_species { "📖 New species!" } else { "🎣 Catch" }, 1),
    paragraph(bold(catch.species_name.as_str())),
    table()
      .row(("📏 Length", format_args!("{:.1} cm", f64::from(catch.length_mm) / 10.0)))
      .row(("⚖️ Weight", format_weight(catch.weight_g)))
      .row(("⚡ Timing", reaction_label(catch.reaction)))
      .row(("🪙 Value", catch.sale_value))
      .compact()
      .into(),
    paragraph(("✨ ", bold(format_args!("+{} XP", catch.xp_gained)), format_args!(" · Level {}", catch.level))),
  ];
  if catch.global_first {
    blocks.push(block_quote([paragraph("🌍 World first — nobody has recorded this species before.")]));
  }
  if catch.level_up {
    blocks.push(paragraph(format_args!("⭐ Level {} reached.", catch.level)));
  }
  blocks.push(button_row([success_callback_button("🎣 Cast again", Callback::Cast.encode())]));

  let markup = markup::inline([[markup::callback("← Journal", Callback::Journal.encode()), markup::callback("🏠 Home", Callback::Home.encode())]]);
  edit_panel(client, catch.chat_id, catch.message_id, rich(blocks), markup).await
}

pub async fn struggle(client: &Client, struggle: &StruggleView) -> client::Result<()> {
  edit_message(
    client,
    struggle.chat_id,
    struggle.message_id,
    rich([
      heading("🌊 The catch fights back", 1),
      paragraph(italic(struggle.observation)),
      paragraph("Pull to press the fish; give line when forcing it would risk the tackle."),
      button_row([
        primary_callback_button("💪 Pull", Callback::Pull { encounter_id: struggle.encounter_id, step: struggle.step }.encode()),
        callback_button("〰️ Give line", Callback::GiveLine { encounter_id: struggle.encounter_id, step: struggle.step }.encode()),
      ]),
      button_row([danger_callback_button("✂️ Cut line", Callback::CancelFishing.encode())]),
    ]),
  )
  .await
}

pub async fn relic(client: &Client, relic: &RelicView) -> client::Result<()> {
  let mut blocks = vec![
    heading("🗝 Something from the water", 1),
    heading(relic.name, 2),
    paragraph(relic.description),
    paragraph(format_args!("✨ +{} XP · Level {}", relic.xp_gained, relic.level)),
    block_quote([paragraph("The lighthouse emblem looks deliberate. The breakwater may have more to tell you.")]),
  ];
  if relic.global_first {
    blocks.push(block_quote([paragraph("🌍 World first — this is the first recorded Rusted Key.")]));
  }
  if relic.level_up {
    blocks.push(paragraph(format_args!("⭐ Level {} reached.", relic.level)));
  }
  blocks.push(button_row([primary_callback_button("🔎 Follow the clue", Callback::Explore.encode())]));
  let markup = markup::inline([[markup::callback("← Locations", Callback::Locations.encode()), markup::callback("🏠 Home", Callback::Home.encode())]]);
  edit_panel(client, relic.chat_id, relic.message_id, rich(blocks), markup).await
}

pub async fn explored(client: &Client, chat_id: i64, message_id: i64, view: &ExploreView) -> client::Result<()> {
  let mut blocks = vec![heading(format_args!("🔎 {}", view.title), 1), paragraph(view.text.as_str())];
  if let Some(location) = &view.discovered_location {
    blocks.push(heading("🗺 Location discovered", 2));
    blocks.push(block_quote([paragraph(format_args!("{} — {}", location.name, location.description))]));
  }
  if view.xp_gained > 0 {
    blocks.push(paragraph(format_args!("✨ +{} XP", view.xp_gained)));
  }
  blocks.push(button_row([primary_callback_button("🔎 Explore again", Callback::Explore.encode())]));
  let markup = markup::inline([[markup::callback("← Locations", Callback::Locations.encode()), markup::callback("🏠 Home", Callback::Home.encode())]]);
  edit_panel(client, chat_id, message_id, rich(blocks), markup).await
}

pub async fn escaped(client: &Client, escape: &EscapeView) -> client::Result<()> {
  let content = rich([heading("🌫 It got away", 1), paragraph(escape.reason), button_row([success_callback_button("🎣 Cast again", Callback::Cast.encode())])]);
  let markup = markup::inline([[markup::callback("🎒 Inventory", Callback::Inventory.encode()), markup::callback("🏠 Home", Callback::Home.encode())]]);
  edit_panel(client, escape.chat_id, escape.message_id, content, markup).await
}
