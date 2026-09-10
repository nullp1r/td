//! Home and inventory Rich Message presentation.

use tdx::{client, prelude::*};

use crate::{
  telegram::callback::Callback,
  view::{CharacterView, InventoryView},
};

use super::{edit_panel, format_weight};

pub async fn home(client: &Client, chat_id: i64, character: &CharacterView) -> client::Result<types::message> {
  let mut request = send::message(chat_id, home_content(character));
  request.reply_markup = Some(home_markup(character));
  client.track(&request, None, None).await
}

pub async fn home_edit(client: &Client, chat_id: i64, message_id: i64, character: &CharacterView) -> client::Result<()> {
  edit_panel(client, chat_id, message_id, home_content(character), home_markup(character)).await
}

pub async fn inventory(client: &Client, chat_id: i64, message_id: i64, view: &InventoryView) -> client::Result<()> {
  edit_panel(client, chat_id, message_id, inventory_content(view), inventory_markup()).await
}

fn home_content(character: &CharacterView) -> types::inputMessageRichMessage {
  let mut blocks = vec![
    heading(format!("🌊 {}", character.location.name), 1),
    paragraph(character.location.description.as_str()),
    block_quote([paragraph(format_args!("{} · {}", character.environment.day_part.label(), character.environment.weather.label(),))]),
    table()
      .row(("👤 Angler", character.name.as_str()))
      .row(("⭐ Level", character.level))
      .row(("✨ XP", character.xp))
      .row(("🏷 Title", character.title_name.unwrap_or("—")))
      .row(("🪙 Coins", character.coins))
      .row(("🎣 Rod", format_args!("{} · {}%", character.rod_name, character.rod_condition)))
      .row(("🪱 Bait", format_args!("{} ×{}", character.bait_name, character.bait_left)))
      .compact()
      .into(),
  ];
  if character.has_rusted_key {
    blocks.push(block_quote([paragraph("🗝 The Rusted Key still bears a lighthouse emblem. The shoreline may tell you where it belongs.")]));
  }
  if character.bait_left == 0 && character.location.fishable {
    blocks.push(block_quote([paragraph(if character.total_bait == 0 {
      "You are out of bait. The tackle stall can restock you, or you can dig up a few emergency worms."
    } else {
      "Your selected bait is empty, but you still have another bait in your inventory."
    })]));
  }
  if !character.location.fishable {
    blocks.push(block_quote([paragraph("There is nowhere useful to cast here. Explore the area instead.")]));
  }

  let actions = if character.fishing_active {
    button_row([danger_callback_button("✂️ Cut active line", Callback::CancelFishing.encode())])
  } else if character.location.fishable && character.bait_left > 0 {
    button_row([success_callback_button("🎣 Cast a line", Callback::Cast.encode()), primary_callback_button("🔎 Explore", Callback::Explore.encode())])
  } else if character.location.fishable && character.total_bait > 0 {
    button_row([callback_button("🎒 Select bait", Callback::Inventory.encode()), primary_callback_button("🔎 Explore", Callback::Explore.encode())])
  } else if character.location.fishable {
    button_row([success_callback_button("🪱 Dig for worms", Callback::ForageBait.encode()), primary_callback_button("🔎 Explore", Callback::Explore.encode())])
  } else {
    button_row([primary_callback_button("🔎 Explore", Callback::Explore.encode())])
  };
  blocks.push(actions);
  rich(blocks)
}

fn home_markup(character: &CharacterView) -> enums::ReplyMarkup {
  let mut rows = Vec::new();
  if character.has_npc {
    rows.push(vec![markup::primary("💬 Mara", Callback::Talk.encode()), markup::callback("📋 Harbor Board", Callback::Tasks.encode())]);
  } else {
    rows.push(vec![markup::callback("📋 Harbor Board", Callback::Tasks.encode()), markup::callback("🌦 Conditions", Callback::Conditions.encode())]);
  }
  rows.push(vec![markup::callback("🗺 Locations", Callback::Locations.encode()), markup::callback("🎒 Inventory", Callback::Inventory.encode())]);
  rows.push(vec![markup::callback("📖 Journal", Callback::Journal.encode()), markup::callback("🏆 Records", Callback::Records.encode())]);
  rows.push(vec![markup::callback("🧰 Prepare bait", Callback::Crafting.encode()), markup::callback("🏷 Titles", Callback::Titles.encode())]);
  rows.push(vec![markup::callback("🏪 Tackle stall", Callback::Shop.encode())]);
  if character.has_npc {
    rows.push(vec![markup::callback("🌦 Conditions", Callback::Conditions.encode()), markup::callback("ℹ️ Help", Callback::Help.encode())]);
  } else {
    rows.push(vec![markup::callback("ℹ️ Help", Callback::Help.encode())]);
  }
  markup::inline(rows)
}

fn inventory_content(view: &InventoryView) -> types::inputMessageRichMessage {
  let bait_table = table().header(("Bait", "Power", "Count")).rows(view.baits.iter().map(|bait| {
    let name = if bait.selected { format!("✓ {}", bait.name) } else { bait.name.clone() };
    (name, bait.power, bait.quantity)
  }));
  let rod_table = view.rods.iter().filter(|rod| rod.owned).fold(table().header(("Rod", "Control", "Condition")), |table, rod| {
    let name = if rod.equipped { format!("✓ {}", rod.name) } else { rod.name.clone() };
    table.row((name, rod.control, format_args!("{}%", rod.condition)))
  });
  let mut blocks = vec![
    heading("🎒 Inventory", 1),
    paragraph(format_args!("🪙 {} coins · 🎣 {} equipped", view.coins, view.rod_name)),
    heading("🎣 Rods", 2),
    rod_table.striped().compact().into(),
    heading("🪱 Bait", 2),
    bait_table.striped().compact().into(),
  ];
  if view.has_rusted_key {
    blocks.extend([heading("🗝 Relics", 2), paragraph("🗝 Rusted Key — an old lighthouse emblem is stamped into the bow.")]);
  }
  blocks.extend([heading("🐟 Catches", 2), paragraph(format_args!("{} stored · worth about {} coins at the tackle stall", view.catch_count, view.sell_value))]);
  if view.recent_catches.is_empty() {
    blocks.push(paragraph(italic("No catches stored yet.")));
  } else {
    let catches = view.recent_catches.iter().fold(table().header(("Recent catch", "Size", "Value")), |table, catch| {
      table.row((catch.species_name.as_str(), format_args!("{:.1} cm · {}", f64::from(catch.length_mm) / 10.0, format_weight(catch.weight_g)), catch.value))
    });
    blocks.push(catches.striped().compact().into());
  }
  blocks.extend(
    view
      .rods
      .iter()
      .filter(|rod| rod.owned && !rod.equipped)
      .map(|rod| button_row([callback_button(format_args!("Equip {} · control {}", rod.name, rod.control), Callback::EquipRod { rod_id: rod.id }.encode())])),
  );
  blocks.extend(
    view
      .baits
      .iter()
      .filter(|bait| bait.quantity > 0 && !bait.selected)
      .map(|bait| button_row([callback_button(format_args!("Use {} ×{}", bait.name, bait.quantity), Callback::SelectBait { bait_id: bait.id }.encode())])),
  );
  if view.catch_count > 0 {
    blocks.push(button_row([danger_callback_button(format_args!("Sell all fish · {} coins", view.sell_value), Callback::SellAll.encode())]));
  }
  rich(blocks)
}

fn inventory_markup() -> enums::ReplyMarkup {
  markup::inline(vec![
    vec![markup::callback("🧰 Prepare bait", Callback::Crafting.encode()), markup::callback("🏪 Tackle stall", Callback::Shop.encode())],
    vec![markup::callback("🏠 Home", Callback::Home.encode())],
  ])
}
