//! Private fishing encounter/result presentation.

use tdx::{client, prelude::*};

use crate::{
  telegram::callback::Callback,
  view::{BiteView, CatchView, CharacterView, EscapeView, ExploreView, RelicView, StruggleView},
};

use super::{edit_message, edit_panel, format_weight, reaction_label, share_target};

pub async fn casting(client: &Client, chat_id: i64, message_id: i64, character: &CharacterView) -> client::Result<()> {
  let content = rich([
    heading("Line in the water", 1),
    paragraph(format!("The line settles off {}.", character.location.name)),
    paragraph(format!("{} remaining · {}", character.bait_name, character.bait_left)),
    paragraph(italic("Watch the float…")),
    button_row([danger_callback_button("✂️ Cut line", Callback::CancelFishing.encode())]),
  ]);
  edit_message(client, chat_id, message_id, content).await
}

pub async fn bite(client: &Client, bite: &BiteView) -> client::Result<()> {
  let content = rich([
    heading("Bite!", 1),
    pull_quote(bite.observation),
    paragraph("React quickly. The longer you wait, the worse your timing becomes."),
    button_row([primary_callback_button("🎣 Reel now", Callback::Reel { encounter_id: bite.encounter_id, step: bite.step }.encode())]),
    button_row([danger_callback_button("✂️ Cut line", Callback::CancelFishing.encode())]),
  ]);
  edit_message(client, bite.chat_id, bite.message_id, content).await
}

pub async fn caught(client: &Client, catch: &CatchView, durable: bool) -> client::Result<()> {
  let mut blocks = vec![
    heading(
      if catch.new_species {
        "New field note"
      } else if catch.world_best {
        "World record"
      } else {
        "Catch"
      },
      1,
    ),
    paragraph(bold(catch.species_name.as_str())),
    table()
      .row(("Length", format!("{:.1} cm", f64::from(catch.length_mm) / 10.0)))
      .row(("Weight", format_weight(catch.weight_g)))
      .row(("Timing", reaction_label(catch.reaction)))
      .row(("Value", format!("{}c", catch.sale_value)))
      .compact()
      .into(),
    paragraph((bold(format!("+{} XP", catch.xp_gained)), format!(" · Level {}", catch.level))),
  ];

  if catch.new_species {
    blocks.push(block_quote([paragraph("📖 First personal record of this species. Its known field note is now available in your Journal.")]));
  }
  if catch.personal_best && !catch.new_species {
    blocks.push(paragraph("● Personal best for this species."));
  }
  if catch.world_best {
    blocks.push(block_quote([paragraph(if catch.global_first {
      "🌍 World first and current world record — nobody had recorded this species before."
    } else {
      "🌍 New world record — this is the heaviest recorded specimen of the species."
    })]));
  }
  if catch.level_up {
    blocks.push(paragraph(format!("⭐ Level {} reached.", catch.level)));
  }

  if durable {
    let markup = markup::inline([[markup::switch_inline_target("↗ Share this catch", format!("catch:{}", catch.item_id), share_target())]]);
    edit_panel(client, catch.chat_id, catch.message_id, rich(blocks), markup).await
  } else {
    blocks.push(button_row([success_callback_button("🎣 Cast again", Callback::Cast.encode())]));
    let markup = markup::inline(vec![
      vec![markup::switch_inline_target("↗ Share this catch", format!("catch:{}", catch.item_id), share_target())],
      vec![markup::callback("📖 Journal", Callback::Journal.encode()), markup::callback("🏠 Home", Callback::Home.encode())],
    ]);
    edit_panel(client, catch.chat_id, catch.message_id, rich(blocks), markup).await
  }
}

pub async fn struggle(client: &Client, struggle: &StruggleView) -> client::Result<()> {
  let content = rich([
    heading("The catch fights back", 1),
    pull_quote(struggle.observation),
    paragraph("Read what the fish is doing: press it when you have control; give line when forcing it would load the tackle."),
    button_row([
      primary_callback_button("💪 Pull", Callback::Pull { encounter_id: struggle.encounter_id, step: struggle.step }.encode()),
      callback_button("〰️ Give line", Callback::GiveLine { encounter_id: struggle.encounter_id, step: struggle.step }.encode()),
    ]),
    button_row([danger_callback_button("✂️ Cut line", Callback::CancelFishing.encode())]),
  ]);
  edit_message(client, struggle.chat_id, struggle.message_id, content).await
}

pub async fn relic(client: &Client, relic: &RelicView) -> client::Result<()> {
  let mut blocks = vec![
    heading("Something from the water", 1),
    heading(relic.name, 2),
    paragraph(relic.description),
    paragraph(format!("+{} XP · Level {}", relic.xp_gained, relic.level)),
    block_quote([paragraph("The lighthouse emblem is deliberate. Mara may recognize it; the breakwater may have more to say.")]),
  ];
  if relic.global_first {
    blocks.push(block_quote([paragraph("🌍 World first — this is the first recorded Rusted Key.")]));
  }
  if relic.level_up {
    blocks.push(paragraph(format!("⭐ Level {} reached.", relic.level)));
  }
  edit_message(client, relic.chat_id, relic.message_id, rich(blocks)).await
}

pub async fn explored(client: &Client, chat_id: i64, message_id: i64, view: &ExploreView) -> client::Result<()> {
  let mut blocks = vec![heading(format!("🔎 {}", view.title), 1), paragraph(view.text.as_str())];
  if let Some(location) = &view.discovered_location {
    blocks.push(heading("Place discovered", 2));
    blocks.push(block_quote([paragraph((bold(location.name.as_str()), " — ", location.description.as_str()))]));
  }
  if view.xp_gained > 0 {
    blocks.push(paragraph(format!("+{} XP", view.xp_gained)));
  }

  if view.discovered_location.is_some() {
    edit_message(client, chat_id, message_id, rich(blocks)).await
  } else {
    blocks.push(button_row([primary_callback_button("🔎 Explore again", Callback::Explore.encode())]));
    let markup = markup::inline([[markup::callback("← Places", Callback::Locations.encode()), markup::callback("🏠 Home", Callback::Home.encode())]]);
    edit_panel(client, chat_id, message_id, rich(blocks), markup).await
  }
}

pub async fn escaped(client: &Client, escape: &EscapeView) -> client::Result<()> {
  let content = rich([heading("It got away", 1), paragraph(escape.reason), button_row([success_callback_button("🎣 Cast again", Callback::Cast.encode())])]);
  let markup = markup::inline([[markup::callback("🎒 Inventory", Callback::Inventory.encode()), markup::callback("🏠 Home", Callback::Home.encode())]]);
  edit_panel(client, escape.chat_id, escape.message_id, content, markup).await
}
